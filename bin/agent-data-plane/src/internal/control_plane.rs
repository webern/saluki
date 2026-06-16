use std::path::PathBuf;

use agent_data_plane_config::DataPlaneConfig;
use agent_data_plane_config_system::ScopedConfigHandle;
use datadog_agent_commons::ipc::tls::build_ipc_server_tls_config;
use resource_accounting::ComponentRegistry;
use saluki_api::EndpointType;
use saluki_app::{
    accounting::ResourceTelemetryWorker, config::ConfigWorker, dynamic_api::DynamicAPIBuilder,
    logging::LoggingOverrideController,
};
use saluki_core::{
    health::HealthRegistry,
    runtime::{RestartStrategy, RuntimeConfiguration, Supervisor},
};
use saluki_error::GenericError;

use crate::internal::{
    logging::DynamicLogLevelWorker, remote_agent::RemoteAgentServices, telemetry::InternalTelemetryAPIWorker,
    TopologyControlSurfaces,
};

/// Creates the control plane supervisor.
///
/// This supervisor manages the health registry, unprivileged and privileged APIs, and optionally the remote agent
/// registration task.
///
/// It runs on a dedicated single-threaded runtime.
///
/// # Errors
///
/// If the supervisor can't be created, an error is returned.
pub async fn create_control_plane_supervisor(
    config_snapshot: serde_json::Value, ipc_cert_path: PathBuf, data_plane: &DataPlaneConfig,
    component_registry: &ComponentRegistry, health_registry: HealthRegistry, control_surfaces: TopologyControlSurfaces,
    services: Option<RemoteAgentServices>, logging_controller: LoggingOverrideController,
    log_level_handle: Option<ScopedConfigHandle<Option<String>>>,
) -> Result<Supervisor, GenericError> {
    let mut supervisor = Supervisor::new("ctrl-pln")?
        .with_dedicated_runtime(RuntimeConfiguration::single_threaded())
        .with_restart_strategy(RestartStrategy::one_to_one());

    supervisor.add_worker(health_registry.worker());
    supervisor.add_worker(ResourceTelemetryWorker::new(component_registry));
    supervisor.add_worker(InternalTelemetryAPIWorker::new());
    supervisor.add_worker(DynamicLogLevelWorker::new_native(logging_controller, log_level_handle));
    supervisor.add_worker(ConfigWorker::from_value(config_snapshot));

    supervisor.add_worker(DynamicAPIBuilder::new(
        EndpointType::Unprivileged,
        data_plane.api_listen_address.clone(),
    ));
    let tls_config = build_ipc_server_tls_config(ipc_cert_path).await?;

    let mut privileged_api =
        DynamicAPIBuilder::new(EndpointType::Privileged, data_plane.secure_api_listen_address.clone())
            .with_tls_config(tls_config);

    privileged_api = control_surfaces.register_control_surfaces(privileged_api);

    // When connected to the Datadog Agent, expose the Remote Agent gRPC services on the privileged API.
    if let Some(services) = &services {
        privileged_api = privileged_api
            .with_grpc_service(services.create_status_service())
            .with_grpc_service(services.create_flare_service())
            .with_grpc_service(services.create_telemetry_service());
    }

    supervisor.add_worker(privileged_api);

    Ok(supervisor)
}
