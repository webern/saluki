//! Datadog Agent attachment used by streamed configuration authority.

use std::time::Duration;

use datadog_agent_commons::ipc::{
    client::RemoteAgentClient,
    config::RemoteAgentClientConfiguration,
    session::{SessionId, SessionIdHandle},
};
use datadog_protos::agent::{config_event, ConfigSnapshot};
use futures::StreamExt;
use prost_types::value::Kind;
use saluki_common::task::spawn_traced_named;
use saluki_config::{dynamic::ConfigUpdate, upsert};
use saluki_error::{generic_error, GenericError};
use saluki_io::net::GrpcTargetAddress;
use serde_json::{Map, Value};
use tokio::{sync::mpsc, sync::oneshot, time::interval};
use tonic::server::NamedService;
use tracing::{debug, error, info, warn};

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const REFRESH_FAILED_RETRY_INTERVAL: Duration = Duration::from_secs(5);

/// Long-lived Datadog Agent attachment established by the configuration system.
#[derive(Clone)]
pub struct DatadogAgentConnection {
    client: RemoteAgentClient,
    session_id: SessionIdHandle,
    service_names: Vec<String>,
}

impl std::fmt::Debug for DatadogAgentConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatadogAgentConnection")
            .field("session_id", &self.session_id)
            .field("service_names", &self.service_names)
            .finish_non_exhaustive()
    }
}

impl DatadogAgentConnection {
    /// Connects to the Datadog Agent from typed IPC configuration.
    pub async fn connect(
        client_config: RemoteAgentClientConfiguration, session_id: SessionIdHandle, service_names: Vec<String>,
    ) -> Result<Self, GenericError> {
        let client = RemoteAgentClient::connect(client_config).await?;
        Ok(Self {
            client,
            session_id,
            service_names,
        })
    }

    /// Connects to the Datadog Agent, registers ADP as a remote agent, and starts the registration refresh loop.
    pub async fn connect_and_register(
        client_config: RemoteAgentClientConfiguration, api_listen_addr: GrpcTargetAddress, service_names: Vec<String>,
    ) -> Result<Self, GenericError> {
        let client = RemoteAgentClient::connect(client_config).await?;
        let (state, init_reg_rx) = RemoteAgentState::new(api_listen_addr, service_names.clone());
        let session_id = state.session_id.clone();

        spawn_traced_named(
            "adp-remote-agent-task",
            run_remote_agent_registration_loop(client.clone(), state),
        );

        match init_reg_rx.await {
            Ok(Ok(())) => (),
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(generic_error!(
                    "Failed to initialize remote agent state. Registration task failed unexpectedly."
                ))
            }
        }

        Ok(Self {
            client,
            session_id,
            service_names,
        })
    }

    /// Creates a dynamic configuration stream from the Datadog Agent config-stream endpoint.
    pub fn create_config_stream(&self) -> mpsc::Receiver<ConfigUpdate> {
        let (sender, receiver) = mpsc::channel(100);
        let client = self.client.clone();
        let session_id = self.session_id.clone();

        tokio::spawn(run_config_stream_event_loop(client, sender, session_id));

        receiver
    }

    /// Returns a clone of the IPC client capability.
    pub fn client(&self) -> RemoteAgentClient {
        self.client.clone()
    }

    /// Returns the remote-agent session handle.
    pub const fn session_id(&self) -> &SessionIdHandle {
        &self.session_id
    }

    /// Returns the service names registered through this connection.
    pub fn service_names(&self) -> &[String] {
        &self.service_names
    }
}

/// Returns the gRPC service names ADP exposes when registered as a Datadog remote agent.
pub fn remote_agent_service_names() -> Vec<String> {
    use datadog_protos::agent::{
        flare::v1::flare_provider_server::FlareProviderServer,
        status::v1::status_provider_server::StatusProviderServer,
        telemetry::v1::telemetry_provider_server::TelemetryProviderServer,
    };

    vec![
        <StatusProviderServer<()> as NamedService>::NAME.to_string(),
        <FlareProviderServer<()> as NamedService>::NAME.to_string(),
        <TelemetryProviderServer<()> as NamedService>::NAME.to_string(),
    ]
}

struct RemoteAgentState {
    pid: u32,
    display_name: String,
    flavor: String,
    api_listen_addr: String,
    session_id: SessionIdHandle,
    service_names: Vec<String>,
    initial_registration_tx: Option<oneshot::Sender<Result<(), GenericError>>>,
}

impl RemoteAgentState {
    fn new(
        api_listen_addr: GrpcTargetAddress, service_names: Vec<String>,
    ) -> (Self, oneshot::Receiver<Result<(), GenericError>>) {
        let app_details = saluki_metadata::get_app_details();
        let display_name = app_details.full_name().to_string();
        let flavor = app_details.full_name().replace(" ", "_").to_lowercase();

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

async fn run_remote_agent_registration_loop(mut client: RemoteAgentClient, mut state: RemoteAgentState) {
    let mut loop_timer = interval(DEFAULT_REFRESH_INTERVAL);
    loop_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    debug!("Remote Agent registration task started.");

    loop {
        loop_timer.tick().await;

        match state.session_id.get() {
            Some(session_id) => {
                debug!(%session_id, "Refreshing registration with Datadog Agent.");

                if client.refresh_remote_agent_request(&session_id).await.is_err() {
                    loop_timer.reset_after(REFRESH_FAILED_RETRY_INTERVAL);
                    state.session_id.update(None);
                    warn!("Failed to refresh registration with the Datadog Agent. Resetting session ID and attempting to re-register shortly.");

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
                            warn!(error = %e, "Received invalid session ID from Datadog Agent after registation. Registration will be retried periodically in the background.");
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
                    warn!(error = %e, "Failed to register with the Datadog Agent. Registration will be retried periodically in the background.");
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
                            let map = snapshot_to_map(&snapshot);
                            Some(ConfigUpdate::Snapshot(map))
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
                Err(e) => {
                    error!("Error while reading config event stream: {}.", e);
                }
            }
        }

        debug!("Config stream ended, retrying in 5 seconds...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

fn snapshot_to_map(snapshot: &ConfigSnapshot) -> Value {
    let mut root = Value::Object(Map::new());

    for setting in &snapshot.settings {
        let value = proto_value_to_serde_value(&setting.value);
        upsert(&mut root, &setting.key, value);
    }

    root
}

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
        Kind::StructValue(s) => {
            let json_map: Map<String, Value> = s
                .fields
                .iter()
                .map(|(k, v)| (k.clone(), proto_value_to_serde_value(&Some(v.clone()))))
                .collect();
            Value::Object(json_map)
        }
        Kind::ListValue(l) => {
            let json_list: Vec<Value> = l
                .values
                .iter()
                .map(|v| proto_value_to_serde_value(&Some(v.clone())))
                .collect();
            Value::Array(json_list)
        }
    }
}
