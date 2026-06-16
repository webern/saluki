//! Configuration API handler.

use async_trait::async_trait;
use http::StatusCode;
use saluki_api::{
    extract::State,
    response::IntoResponse,
    routing::{get, Router},
    APIHandler, DynamicRoute, EndpointType,
};
use saluki_common::sync::shutdown::ShutdownHandle;
use saluki_core::runtime::{state::DataspaceRegistry, InitializationError, Supervisable, SupervisorFuture};
use saluki_error::generic_error;
use serde_json::Value;
use tokio::sync::watch;

/// Live view of the runtime configuration exposed by the config API.
#[derive(Clone, Debug)]
pub struct ConfigView {
    rx: watch::Receiver<Value>,
}

impl ConfigView {
    /// Creates a config view from a watch receiver.
    pub const fn new(rx: watch::Receiver<Value>) -> Self {
        Self { rx }
    }

    /// Creates a fixed config view.
    pub fn fixed(value: Value) -> Self {
        let (_tx, rx) = watch::channel(value);
        Self { rx }
    }

    /// Returns the current configuration value.
    pub fn current(&self) -> Value {
        self.rx.borrow().clone()
    }
}

/// State used for the config API handler.
#[derive(Clone)]
pub struct ConfigState {
    config: ConfigView,
}

/// An API handler for returning the current configuration.
///
/// This handler exposes a single route -- `/config` -- that returns the current configuration in its serialized JSON
/// form. This allows determining exactly how the process' configuration looks based on the various providers being
/// used, including any dynamic changes being applied.
pub struct ConfigAPIHandler {
    state: ConfigState,
}

impl ConfigAPIHandler {
    fn new(config: ConfigView) -> Self {
        Self {
            state: ConfigState { config },
        }
    }

    async fn config_handler(State(state): State<ConfigState>) -> impl IntoResponse {
        let config = state.config.current();
        match serde_json::to_string(&config) {
            Ok(body) => (StatusCode::OK, body).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize configuration: {}", e),
            )
                .into_response(),
        }
    }
}

impl APIHandler for ConfigAPIHandler {
    type State = ConfigState;

    fn generate_initial_state(&self) -> Self::State {
        self.state.clone()
    }

    fn generate_routes(&self) -> Router<Self::State> {
        Router::new().route("/config", get(Self::config_handler))
    }
}

/// A worker for exposing an endpoint that returns the current configuration.
///
/// When running, the worker asserts a set of routes (based on [`ConfigAPIHandler`]) that allow querying the current
/// configuration. As the configuration may contain sensitive data, these routes are only present on the privileged API
/// endpoint.
pub struct ConfigWorker {
    handler: ConfigAPIHandler,
}

impl ConfigWorker {
    /// Creates a new [`ConfigWorker`] with the given configuration view.
    pub fn new(config: ConfigView) -> Self {
        Self {
            handler: ConfigAPIHandler::new(config),
        }
    }
}

#[async_trait]
impl Supervisable for ConfigWorker {
    fn name(&self) -> &str {
        "config-api"
    }

    async fn initialize(&self, process_shutdown: ShutdownHandle) -> Result<SupervisorFuture, InitializationError> {
        let config_route = DynamicRoute::http(EndpointType::Privileged, &self.handler);

        Ok(Box::pin(async move {
            DataspaceRegistry::try_current()
                .ok_or_else(|| generic_error!("Dataspace not available."))?
                .assert(config_route, "config-api");

            process_shutdown.await;
            Ok(())
        }))
    }
}
