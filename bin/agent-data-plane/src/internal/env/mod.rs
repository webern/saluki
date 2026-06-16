use std::future::Future;

use agent_data_plane_config::SalukiConfiguration;
use agent_data_plane_config_system::StartedAttachments;
use datadog_agent_commons::ipc::client::RemoteAgentClient;
use resource_accounting::ComponentRegistry;
use saluki_core::health::HealthRegistry;
use saluki_core::runtime::Supervisor;
use saluki_env::{
    autodiscovery::providers::BoxedAutodiscoveryProvider,
    host::providers::{BoxedHostProvider, FixedHostProvider},
    EnvironmentProvider,
};
use saluki_error::GenericError;
use tracing::warn;

mod autodiscovery;
pub use self::autodiscovery::RemoteAgentAutodiscoveryProvider;

mod host;
pub use self::host::RemoteAgentHostProvider;

mod workload;
pub use self::workload::RemoteAgentWorkloadProvider;

/// Agent Data Plane-specific environment provider.
///
/// This environment provider is designed for ADP's normal deployment environment, which is running alongside the
/// Datadog Agent. The underlying providers will communicate directly with the Datadog Agent to receive information such
/// as the hostname, entity tags, workload metadata events, and more.
///
/// # Opting out for testing/benchmarking
///
/// In order to facilitate testing/benchmarking where running the Datadog Agent isn't desirable, the underlying
/// providers can be effectively disabled by setting the `adp.use_fixed_host_provider` configuration value to `true`.
///
/// This will effectively disable origin enrichment (no entity tags) and cause metrics to be tagged with a fixed
/// hostname based on the configuration value of `hostname`.
#[derive(Clone)]
pub struct ADPEnvironmentProvider {
    host_provider: BoxedHostProvider,
    datadog_agent_client: Option<RemoteAgentClient>,
    workload_provider: Option<RemoteAgentWorkloadProvider>,
    autodiscovery_provider: Option<BoxedAutodiscoveryProvider>,
    health_registry: HealthRegistry,
}

impl ADPEnvironmentProvider {
    /// Creates an `ADPEnvironmentProvider` from native ADP configuration and configuration-system attachments.
    pub async fn from_saluki_configuration(
        config: &SalukiConfiguration, attachments: &StartedAttachments, component_registry: &ComponentRegistry,
        health_registry: &HealthRegistry,
    ) -> Result<(Self, Option<Supervisor>), GenericError> {
        let StartedAttachments::DatadogAgentConfigStream { connection, .. } = attachments else {
            warn!("Running without a Datadog Agent attachment. Origin detection/enrichment and other features dependent upon the Datadog Agent will not be available.");

            let env = Self {
                host_provider: BoxedHostProvider::from_provider(FixedHostProvider::new(
                    config.environment.hostname().to_string(),
                )),
                datadog_agent_client: None,
                workload_provider: None,
                autodiscovery_provider: None,
                health_registry: health_registry.clone(),
            };
            return Ok((env, None));
        };

        let client = connection.client();
        let mut provider_component = component_registry.get_or_create("env_provider");
        let mut env_supervisor = Supervisor::new("env-provider")?;

        let host_provider = RemoteAgentHostProvider::from_client(client.clone());
        provider_component
            .bounds_builder()
            .with_subcomponent("host", &host_provider);

        let (workload_provider, workload_supervisor) = RemoteAgentWorkloadProvider::from_datadog_client(
            client.clone(),
            component_registry.get_or_create("workload"),
            health_registry,
        )
        .await?;
        env_supervisor.add_worker(workload_supervisor);

        let (autodiscovery_provider, autodiscovery_supervisor) =
            RemoteAgentAutodiscoveryProvider::from_client(client.clone())?;
        env_supervisor.add_worker(autodiscovery_supervisor);

        let env = Self {
            host_provider: BoxedHostProvider::from_provider(host_provider),
            datadog_agent_client: Some(client),
            workload_provider: Some(workload_provider),
            autodiscovery_provider: Some(BoxedAutodiscoveryProvider::from_provider(autodiscovery_provider)),
            health_registry: health_registry.clone(),
        };

        Ok((env, Some(env_supervisor)))
    }

    /// Returns the Datadog Agent IPC client, if this environment is attached to the Datadog Agent.
    pub fn datadog_agent_client(&self) -> Option<RemoteAgentClient> {
        self.datadog_agent_client.clone()
    }

    /// Returns a future that resolves once the environment provider's background subsystems are ready.
    ///
    /// Specifically, this waits for the workload provider's metadata aggregator and collectors to become ready, which
    /// ensures that origin detection and entity tagging are operational before the caller begins processing data. In
    /// standalone mode -- where there is no workload provider -- the returned future resolves immediately.
    pub fn wait_for_ready(&self) -> impl Future<Output = ()> + Send + 'static {
        let health_registry = self.health_registry.clone();
        let has_workload_provider = self.workload_provider.is_some();
        async move {
            if has_workload_provider {
                health_registry
                    .all_ready_matching(|name| name.starts_with(workload::WORKLOAD_HEALTH_PREFIX))
                    .await;
            }
        }
    }
}

impl EnvironmentProvider for ADPEnvironmentProvider {
    type Host = BoxedHostProvider;
    type Workload = Option<RemoteAgentWorkloadProvider>;
    type AutodiscoveryProvider = Option<BoxedAutodiscoveryProvider>;

    fn host(&self) -> &Self::Host {
        &self.host_provider
    }

    fn workload(&self) -> &Self::Workload {
        &self.workload_provider
    }

    fn autodiscovery(&self) -> &Self::AutodiscoveryProvider {
        &self.autodiscovery_provider
    }
}
