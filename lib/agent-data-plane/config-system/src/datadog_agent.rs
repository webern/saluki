//! Datadog Agent attachment used by a streamed configuration authority.
//!
//! This owns the *config-authority* half of what used to be the bin-local `RemoteAgentBootstrap`:
//! connecting to Agent IPC, registering as a remote agent, owning the `RemoteAgentClient` and the
//! session handle, and creating the configuration stream. The *service-implementation* half (status,
//! flare, telemetry) stays in the binary and consumes the typed attachment exposed here.

use std::time::Duration;

use datadog_agent_commons::ipc::client::RemoteAgentClient;
use datadog_agent_commons::ipc::config::RemoteAgentClientConfiguration;
use datadog_agent_commons::ipc::session::{SessionId, SessionIdHandle};
use datadog_protos::agent::{config_event, ConfigSnapshot};
use futures::StreamExt as _;
use prost_types::value::Kind;
use saluki_config::{dynamic::ConfigUpdate, upsert, GenericConfiguration};
use saluki_error::{generic_error, GenericError};
use saluki_io::net::GrpcTargetAddress;
use serde_json::{Map, Value};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, error, info, warn};

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const REFRESH_FAILED_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const CONFIG_STREAM_RETRY_INTERVAL: Duration = Duration::from_secs(5);

/// Long-lived Datadog Agent attachment established by the configuration system.
///
/// Holds the client/session capability needed by the configuration stream and by ADP's Datadog
/// Agent integrations. It is cloneable so the configuration system can retain its own handle while
/// the binary's integrations share the same connection.
#[derive(Clone)]
pub struct DatadogAgentConnection {
    client: RemoteAgentClient,
    session_id: SessionIdHandle,
    service_names: Vec<String>,
}

impl std::fmt::Debug for DatadogAgentConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatadogAgentConnection")
            .field("service_names", &self.service_names)
            .finish_non_exhaustive()
    }
}

impl DatadogAgentConnection {
    /// Connects to the Datadog Agent IPC endpoint and registers as a remote agent.
    ///
    /// `secure_api_listen_address` is the privileged control API address published to the Agent at
    /// registration. `service_names` are the gRPC services the binary will expose (status, flare,
    /// telemetry); they are advertised during registration but implemented by the binary.
    ///
    /// Configuration is parsed from `config` here — the only point a raw map touches IPC setup —
    /// then handed to the side-effectful [`RemoteAgentClient::connect`].
    pub async fn connect(
        config: &GenericConfiguration, secure_api_listen_address: &saluki_io::net::ListenAddress,
        service_names: Vec<String>,
    ) -> Result<Self, GenericError> {
        let api_listen_addr = GrpcTargetAddress::try_from_listen_addr(secure_api_listen_address).ok_or_else(|| {
            generic_error!("Failed to derive a gRPC target address from the secure API listen address.")
        })?;

        let ipc_config = RemoteAgentClientConfiguration::from_configuration(config)?;
        let client = RemoteAgentClient::connect(ipc_config).await?;

        let (state, init_reg_rx) = RegistrationState::new(api_listen_addr, service_names.clone());
        let session_id = state.session_id.clone();

        let registration_client = client.clone();
        tokio::spawn(run_registration_loop(registration_client, state));

        match init_reg_rx.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(generic_error!(
                    "Remote agent registration task failed before initial registration completed."
                ))
            }
        }

        Ok(Self {
            client,
            session_id,
            service_names,
        })
    }

    /// Returns the session handle, shared with ADP Datadog Agent integrations.
    pub fn session_id(&self) -> SessionIdHandle {
        self.session_id.clone()
    }

    /// Returns a clone of the underlying client capability.
    pub fn client(&self) -> RemoteAgentClient {
        self.client.clone()
    }

    /// Returns the advertised service names.
    pub fn service_names(&self) -> &[String] {
        &self.service_names
    }

    /// Creates a configuration stream, returning a receiver of typed config updates.
    ///
    /// The configuration system is the sole receiver of inbound config updates; downstream typed,
    /// scoped update delivery is layered on top of this single stream.
    pub fn create_config_stream(&self) -> mpsc::Receiver<ConfigUpdate> {
        let (sender, receiver) = mpsc::channel(100);
        tokio::spawn(run_config_stream_event_loop(
            self.client.clone(),
            sender,
            self.session_id.clone(),
        ));
        receiver
    }
}

struct RegistrationState {
    pid: u32,
    display_name: String,
    flavor: String,
    api_listen_addr: String,
    session_id: SessionIdHandle,
    service_names: Vec<String>,
    initial_registration_tx: Option<oneshot::Sender<Result<(), GenericError>>>,
}

impl RegistrationState {
    fn new(
        api_listen_addr: GrpcTargetAddress, service_names: Vec<String>,
    ) -> (Self, oneshot::Receiver<Result<(), GenericError>>) {
        let app_details = saluki_metadata::get_app_details();
        let display_name = app_details.full_name().to_string();
        let flavor = app_details.full_name().replace(' ', "_").to_lowercase();

        let (init_reg_tx, init_reg_rx) = oneshot::channel();

        let state = Self {
            pid: std::process::id(),
            display_name,
            flavor,
            api_listen_addr: api_listen_addr.to_string(),
            session_id: SessionIdHandle::empty(),
            service_names,
            initial_registration_tx: Some(init_reg_tx),
        };

        (state, init_reg_rx)
    }
}

async fn run_registration_loop(mut client: RemoteAgentClient, mut state: RegistrationState) {
    let mut loop_timer = interval(DEFAULT_REFRESH_INTERVAL);
    loop_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

    debug!("Remote agent registration task started.");

    loop {
        loop_timer.tick().await;

        match state.session_id.get() {
            Some(session_id) => {
                debug!(%session_id, "Refreshing registration with Datadog Agent.");
                if client.refresh_remote_agent_request(&session_id).await.is_err() {
                    loop_timer.reset_after(REFRESH_FAILED_RETRY_INTERVAL);
                    state.session_id.update(None);
                    warn!("Failed to refresh registration with the Datadog Agent. Resetting session ID and re-registering shortly.");
                    continue;
                }
            }
            None => match client
                .register_remote_agent_request(
                    state.pid,
                    &state.display_name,
                    &state.flavor,
                    &state.api_listen_addr,
                    state.service_names.clone(),
                )
                .await
            {
                Ok(resp) => {
                    let resp = resp.into_inner();
                    let new_session_id = match SessionId::new(&resp.session_id) {
                        Ok(session_id) => session_id,
                        Err(e) => {
                            warn!(error = %e, "Received invalid session ID from Datadog Agent. Registration will be retried.");
                            loop_timer.reset_after(DEFAULT_REFRESH_INTERVAL);
                            continue;
                        }
                    };
                    let new_refresh_interval = resp.recommended_refresh_interval_secs;
                    info!(session_id = %new_session_id, "Successfully registered with the Datadog Agent. Refreshing every {} seconds.", new_refresh_interval);

                    state.session_id.update(Some(new_session_id));
                    loop_timer.reset_after(Duration::from_secs(new_refresh_interval as u64));

                    if let Some(tx) = state.initial_registration_tx.take() {
                        let _ = tx.send(Ok(()));
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to register with the Datadog Agent. Registration will be retried.");
                    loop_timer.reset_after(DEFAULT_REFRESH_INTERVAL);
                    if let Some(tx) = state.initial_registration_tx.take() {
                        let _ = tx.send(Err(e));
                    }
                }
            },
        }
    }
}

async fn run_config_stream_event_loop(
    mut client: RemoteAgentClient, sender: mpsc::Sender<ConfigUpdate>, session_id: SessionIdHandle,
) {
    loop {
        debug!("Establishing a new config stream connection to the Core Agent.");

        let current_session_id = session_id.wait_for_update().await;

        let mut stream = client.stream_config_events(&current_session_id);
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    let update = match event.event {
                        Some(config_event::Event::Snapshot(snapshot)) => {
                            Some(ConfigUpdate::Snapshot(snapshot_to_map(&snapshot)))
                        }
                        Some(config_event::Event::Update(update)) => {
                            update.setting.map(|setting| ConfigUpdate::Partial {
                                key: setting.key,
                                value: proto_value_to_serde_value(&setting.value),
                            })
                        }
                        None => {
                            error!("Received a configuration update event with no data.");
                            None
                        }
                    };

                    if let Some(update) = update {
                        if sender.send(update).await.is_err() {
                            warn!("Dynamic configuration channel closed. Config stream shutting down.");
                            return;
                        }
                    }
                }
                Err(e) => error!("Error while reading config event stream: {}.", e),
            }
        }

        debug!("Config stream ended, retrying shortly...");
        tokio::time::sleep(CONFIG_STREAM_RETRY_INTERVAL).await;
    }
}

/// Converts a `ConfigSnapshot` into a nested `serde_json::Value::Object`.
fn snapshot_to_map(snapshot: &ConfigSnapshot) -> Value {
    let mut root = Value::Object(Map::new());
    for setting in &snapshot.settings {
        let value = proto_value_to_serde_value(&setting.value);
        upsert(&mut root, &setting.key, value);
    }
    root
}

/// Recursively converts a `google::protobuf::Value` into a `serde_json::Value`.
fn proto_value_to_serde_value(proto_val: &Option<prost_types::Value>) -> Value {
    let Some(kind) = proto_val.as_ref().and_then(|v| v.kind.as_ref()) else {
        return Value::Null;
    };

    match kind {
        Kind::NullValue(_) => Value::Null,
        Kind::NumberValue(n) => {
            if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                Value::from(*n as i64)
            } else {
                Value::from(*n)
            }
        }
        Kind::StringValue(s) => Value::String(s.clone()),
        Kind::BoolValue(b) => Value::Bool(*b),
        Kind::StructValue(s) => Value::Object(
            s.fields
                .iter()
                .map(|(k, v)| (k.clone(), proto_value_to_serde_value(&Some(v.clone()))))
                .collect(),
        ),
        Kind::ListValue(l) => Value::Array(
            l.values
                .iter()
                .map(|v| proto_value_to_serde_value(&Some(v.clone())))
                .collect(),
        ),
    }
}
