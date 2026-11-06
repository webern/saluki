//! Plans the request routes used by participant-owned artifact collection.

use std::collections::BTreeMap;

use super::{
    endpoints::ParticipantEndpoints, ports::RunnerHostPorts, routes::plan_runner_host_route, PlannedParticipant,
};
use crate::{
    case::CaseFile,
    endpoint::{EndpointOperation, RunnerHostRoute},
    layout::Layout,
    participant::{find_type, CollectSource, ParticipantType},
};

pub(super) fn plan_collections(
    file: &CaseFile, layout: &Layout, types: &[&dyn ParticipantType],
    participants: &BTreeMap<String, PlannedParticipant>, endpoints: &BTreeMap<String, ParticipantEndpoints>,
    ports: &mut RunnerHostPorts<'_>, routes: &mut BTreeMap<(String, String, EndpointOperation), RunnerHostRoute>,
) -> Result<(), String> {
    for (name, entry) in &file.participants {
        let realization = layout
            .get(name)
            .expect("participants are laid out before collection planning");
        let definition = find_type(types, &entry.participant_type).expect("participant types are registered");
        for artifact in definition.collect(realization) {
            let CollectSource::Operation(operation) = artifact.source else {
                continue;
            };
            let context = format!(
                "participant '{name}', collection '{}' calls operation '{operation}'",
                artifact.name
            );
            let offered = participants[name]
                .operations
                .iter()
                .find(|offered| offered.name() == operation)
                .ok_or_else(|| format!("{context}, which its type does not offer"))?;
            for &endpoint in offered.http_endpoints() {
                let route = plan_runner_host_route(
                    &format!("{context}, which requests endpoint '{endpoint}'"),
                    name,
                    endpoint,
                    EndpointOperation::Request,
                    layout,
                    endpoints,
                    ports,
                )?;
                routes.insert((name.clone(), endpoint.into(), EndpointOperation::Request), route);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use async_trait::async_trait;
    use serde_json::Value;

    use super::*;
    use crate::{
        artifact::ArtifactRole,
        case::{
            fixtures::{CallFactory, STEPS},
            CaseLoader, Registries,
        },
        endpoint::{Endpoint, EndpointKind, Port, ResolvedAddress, RunnerHostPort, Transport},
        handle::HandleError,
        layout::{Os, Realization, RealizationKind},
        participant::{CollectArtifact, CollectDestination, EffectiveConfig, OpContext, Operation},
        plan::{CasePlan, Planner},
        step::StepFactory,
    };

    struct Snapshot;
    struct Status;
    struct Orphan;

    #[async_trait]
    impl Operation for Snapshot {
        fn name(&self) -> &'static str {
            "snapshot"
        }
        fn http_endpoints(&self) -> &'static [&'static str] {
            &["api"]
        }
        async fn run(&self, _: &OpContext<'_>, _: Value) -> Result<Value, HandleError> {
            Ok(Value::Null)
        }
    }

    #[async_trait]
    impl Operation for Status {
        fn name(&self) -> &'static str {
            "status"
        }
        fn http_endpoints(&self) -> &'static [&'static str] {
            &["api"]
        }
        async fn run(&self, _: &OpContext<'_>, _: Value) -> Result<Value, HandleError> {
            Ok(Value::Null)
        }
    }

    /// Requests an endpoint the fixture type never serves.
    #[async_trait]
    impl Operation for Orphan {
        fn name(&self) -> &'static str {
            "orphan"
        }
        fn http_endpoints(&self) -> &'static [&'static str] {
            &["absent"]
        }
        async fn run(&self, _: &OpContext<'_>, _: Value) -> Result<Value, HandleError> {
            Ok(Value::Null)
        }
    }

    /// A participant type that declares end-of-case artifacts and serves one HTTP endpoint.
    struct Collector(&'static [CollectArtifact]);

    impl ParticipantType for Collector {
        fn name(&self) -> &'static str {
            "collector"
        }
        fn supports(&self) -> &'static [(Os, RealizationKind)] {
            &[
                (Os::Linux, RealizationKind::LinuxContainer),
                (Os::Linux, RealizationKind::KubernetesPod),
                (Os::MacOs, RealizationKind::HostProcessGroup),
                (Os::Windows, RealizationKind::WindowsContainer),
            ]
        }
        fn artifacts(&self, _: Realization) -> &'static [ArtifactRole] {
            &[]
        }
        fn endpoints(&self, _: Realization, _: &EffectiveConfig) -> Result<BTreeMap<String, Endpoint>, String> {
            Ok(BTreeMap::from([("api".into(), Endpoint::http(Port::Fixed(8080)))]))
        }
        fn operations(&self, _: Realization) -> &'static [&'static dyn Operation] {
            &[&Snapshot, &Status, &Orphan]
        }
        fn collect(&self, _: Realization) -> &'static [CollectArtifact] {
            self.0
        }
        fn processes(&self) -> &'static [&'static str] {
            &["main"]
        }
    }

    const fn operation(operation: &'static str, name: &'static str) -> CollectArtifact {
        CollectArtifact {
            source: CollectSource::Operation(operation),
            name,
            kind: "snapshot",
            destination: CollectDestination::Participant,
        }
    }

    const FILE: CollectArtifact = CollectArtifact {
        source: CollectSource::File("/logs/output.log"),
        name: "output.log",
        kind: "log",
        destination: CollectDestination::Participant,
    };

    const NOOP: &str = "procedure: [{step: noop}]";
    const CALL: &str = "procedure: [{step: call, on: subject, operation: snapshot}]";

    fn yaml(os: Os, kind: RealizationKind, procedure: &str) -> String {
        format!(
            "timeout: 1s\nsubject_os: [{}]\nparticipants:\n  subject: {{type: collector, subject: true, realization: {{kind: {}}}}}\n{procedure}\n",
            os.name(),
            kind.name(),
        )
    }

    fn compile(
        artifacts: &'static [CollectArtifact], steps: &'static [&'static dyn StepFactory], yaml: &str, os: Os,
    ) -> (Result<CasePlan, String>, Planner<'static>) {
        let collector: &'static Collector = Box::leak(Box::new(Collector(artifacts)));
        let registries = Registries {
            participant_types: Box::leak(Box::new([collector as &dyn ParticipantType])),
            steps,
        };
        let case = CaseLoader::new(registries)
            .load_content("cases", yaml, "alpha", Path::new("cases/alpha/config.yaml"))
            .unwrap();
        let mut planner = Planner::new(registries, Default::default()).unwrap();
        let plan = planner
            .compile(case, os)
            .map(|compiled| compiled.into_plan().unwrap())
            .map_err(|error| error.to_string());
        (plan, planner)
    }

    fn address(url: &str, host: &str, port: u16) -> ResolvedAddress {
        ResolvedAddress {
            kind: EndpointKind::Http,
            url: url.into(),
            host: Some(host.into()),
            port: Some(port),
            path: None,
        }
    }

    const REQUEST: EndpointOperation = EndpointOperation::Request;

    #[test]
    fn a_collection_alone_plans_the_route_its_realization_needs() {
        const COLLECTED: &[CollectArtifact] = &[operation("snapshot", "snapshot.json")];
        let published = RunnerHostPort {
            transport: Transport::Tcp,
            container_port: 8080,
            host_port: 20000,
        };
        for (os, kind) in [
            (Os::Linux, RealizationKind::LinuxContainer),
            (Os::Linux, RealizationKind::KubernetesPod),
            (Os::Windows, RealizationKind::WindowsContainer),
            (Os::MacOs, RealizationKind::HostProcessGroup),
        ] {
            let (plan, mut planner) = compile(COLLECTED, STEPS, &yaml(os, kind, NOOP), os);
            let plan = plan.unwrap();
            let (route, ports, next) = match kind {
                RealizationKind::LinuxContainer | RealizationKind::KubernetesPod => (
                    RunnerHostRoute::Direct(address("http://127.0.0.1:20000", "127.0.0.1", 20000)),
                    vec![published],
                    20001,
                ),
                RealizationKind::WindowsContainer => (
                    RunnerHostRoute::Courier(address("http://subject:8080", "subject", 8080)),
                    vec![],
                    20000,
                ),
                RealizationKind::HostProcessGroup => (
                    RunnerHostRoute::Direct(address("http://127.0.0.1:8080", "127.0.0.1", 8080)),
                    vec![],
                    20000,
                ),
            };
            assert_eq!(
                plan.runner_host_route("subject", "api", REQUEST),
                Some(&route),
                "{kind:?}"
            );
            assert_eq!(plan.participants.planned.runner_host_routes.len(), 1, "{kind:?}");
            assert_eq!(plan.runner_host_ports("subject"), ports, "{kind:?}");
            assert_eq!(planner.ports.allocate(), Ok(next), "{kind:?}");
        }
    }

    #[test]
    fn a_collection_reuses_the_route_a_procedure_step_already_planned() {
        const COLLECTED: &[CollectArtifact] = &[operation("snapshot", "snapshot.json")];
        let (plan, mut planner) = compile(
            COLLECTED,
            &[&CallFactory],
            &yaml(Os::Linux, RealizationKind::LinuxContainer, CALL),
            Os::Linux,
        );
        let plan = plan.unwrap();
        assert_eq!(
            plan.runner_host_route("subject", "api", REQUEST),
            Some(&RunnerHostRoute::Direct(address(
                "http://127.0.0.1:20000",
                "127.0.0.1",
                20000
            )))
        );
        assert_eq!(plan.participants.planned.runner_host_routes.len(), 1);
        assert_eq!(plan.runner_host_ports("subject").len(), 1);
        assert_eq!(planner.ports.allocate(), Ok(20001));
    }

    #[test]
    fn collections_requesting_one_endpoint_share_one_published_port() {
        const COLLECTED: &[CollectArtifact] = &[
            operation("snapshot", "snapshot.json"),
            FILE,
            operation("status", "status.json"),
        ];
        let (plan, mut planner) = compile(
            COLLECTED,
            STEPS,
            &yaml(Os::Linux, RealizationKind::LinuxContainer, NOOP),
            Os::Linux,
        );
        let plan = plan.unwrap();
        assert_eq!(plan.participants.planned.runner_host_routes.len(), 1);
        assert_eq!(plan.runner_host_ports("subject").len(), 1);
        assert_eq!(planner.ports.allocate(), Ok(20001));
    }

    #[test]
    fn file_only_or_absent_collections_plan_no_routes_and_allocate_no_ports() {
        const FILES: &[CollectArtifact] = &[FILE];
        for artifacts in [FILES, &[]] {
            for (os, kind) in [
                (Os::Linux, RealizationKind::LinuxContainer),
                (Os::Linux, RealizationKind::KubernetesPod),
            ] {
                let (plan, mut planner) = compile(artifacts, STEPS, &yaml(os, kind, NOOP), os);
                let plan = plan.unwrap();
                assert!(plan.participants.planned.runner_host_routes.is_empty());
                assert!(plan.runner_host_ports("subject").is_empty());
                assert_eq!(planner.ports.allocate(), Ok(20000));
            }
        }
    }

    #[test]
    fn an_unknown_collection_operation_fails_naming_the_collection_and_participant() {
        const COLLECTED: &[CollectArtifact] = &[operation("unknown", "unknown.json")];
        let (plan, mut planner) = compile(
            COLLECTED,
            STEPS,
            &yaml(Os::Linux, RealizationKind::LinuxContainer, NOOP),
            Os::Linux,
        );
        assert_eq!(
            plan.err().unwrap(),
            "case 'alpha' in cases/alpha/config.yaml: on subject OS 'linux', participant 'subject', collection 'unknown.json' calls operation 'unknown', which its type does not offer"
        );
        assert_eq!(planner.ports.allocate(), Ok(20000));
    }

    #[test]
    fn a_collection_operation_requesting_an_unserved_endpoint_fails_naming_it() {
        const COLLECTED: &[CollectArtifact] = &[operation("orphan", "orphan.json")];
        let (plan, mut planner) = compile(
            COLLECTED,
            STEPS,
            &yaml(Os::Linux, RealizationKind::LinuxContainer, NOOP),
            Os::Linux,
        );
        assert_eq!(
            plan.err().unwrap(),
            "case 'alpha' in cases/alpha/config.yaml: on subject OS 'linux', participant 'subject', collection 'orphan.json' calls operation 'orphan', which requests endpoint 'absent', which it does not serve; it serves api"
        );
        assert_eq!(planner.ports.allocate(), Ok(20000));
    }
}
