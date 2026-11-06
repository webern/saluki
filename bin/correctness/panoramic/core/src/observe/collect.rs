//! Collects declared participant artifacts without changing the procedure verdict.

use std::{collections::BTreeMap, fs::File, io::Write as _, path::Path, sync::Mutex, time::Duration};

use tokio_util::sync::CancellationToken;

use crate::{
    endpoint::RunnerHostRoute,
    handle::Primitives,
    participant::{CollectArtifact, CollectDestination, CollectSource, OpContext, Operation},
    result::{Collected, Issue, IssueKind},
};

const COLLECTION_TIME: Duration = Duration::from_secs(5);

#[derive(Default)]
pub(crate) struct CollectionReport {
    pub collected: Vec<Collected>,
    pub issues: Vec<Issue>,
}

pub(crate) fn artifact_file(directory: &Path, name: &str) -> Result<File, String> {
    if name.is_empty() || matches!(name, "." | "..") || name.contains(['/', '\\', ':']) {
        return Err(format!("artifact name '{name}' must be a plain filename"));
    }
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("cannot create artifact directory '{}': {error}", directory.display()))?;
    let path = directory.join(name);
    File::options()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("cannot create artifact '{}': {error}", path.display()))
}

/// The participant and planned routes available to its declared collections.
pub(crate) struct Collector<'a> {
    pub primitives: &'a dyn Primitives,
    pub operations: &'a [&'static dyn Operation],
    pub requests: BTreeMap<&'static str, RunnerHostRoute>,
}

impl Collector<'_> {
    pub async fn collect(
        &self, name: &str, artifacts: &[CollectArtifact], log_dir: &Path, report: &Mutex<CollectionReport>,
        cancel: &CancellationToken,
    ) {
        for artifact in artifacts {
            let directory = match artifact.destination {
                CollectDestination::Participant => log_dir.join(name).join("collected"),
                CollectDestination::Traffic => log_dir.join("traffic"),
                CollectDestination::ParticipantTraffic => log_dir.join("traffic").join(name),
            };
            let copy = async {
                let bytes = match artifact.source {
                    CollectSource::File(path) => self
                        .primitives
                        .read_file(path)
                        .await
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| format!("file '{path}' is missing"))?,
                    CollectSource::Operation(operation) => {
                        let operation = self
                            .operations
                            .iter()
                            .find(|offered| offered.name() == operation)
                            .ok_or_else(|| format!("operation '{operation}' is not offered"))?;
                        let requests = operation
                            .http_endpoints()
                            .iter()
                            .filter_map(|endpoint| self.requests.get(endpoint).cloned().map(|route| (*endpoint, route)))
                            .collect();
                        let value = operation
                            .run(
                                &OpContext {
                                    primitives: self.primitives,
                                    limit: COLLECTION_TIME,
                                    requests,
                                },
                                serde_json::Value::Null,
                            )
                            .await
                            .map_err(|error| error.to_string())?;
                        serde_json::to_vec(&value).map_err(|error| format!("cannot encode collection: {error}"))?
                    }
                };
                let mut output = artifact_file(&directory, artifact.name)?;
                let path = directory.join(artifact.name);
                if let Err(error) = output.write_all(&bytes) {
                    drop(output);
                    let _ = std::fs::remove_file(&path);
                    return Err(format!("cannot write '{}': {error}", path.display()));
                }
                std::path::absolute(&path).map_err(|error| format!("cannot resolve '{}': {error}", path.display()))
            };
            let result = match cancel
                .run_until_cancelled(tokio::time::timeout(COLLECTION_TIME, copy))
                .await
            {
                Some(Ok(result)) => result,
                Some(Err(_)) => Err(format!("collection exceeded {COLLECTION_TIME:?}")),
                None => Err("collection was cancelled".into()),
            };
            let mut report = report.lock().unwrap();
            match result {
                Ok(path) => report.collected.push(Collected {
                    participant: name.into(),
                    kind: artifact.kind.into(),
                    path,
                }),
                Err(error) => report.issues.push(Issue {
                    kind: IssueKind::CollectionFailed,
                    message: format!(
                        "participant '{name}', collection '{}' from {:?}: {error}",
                        artifact.name, artifact.source
                    ),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handle::HandleError, step::test_support::FakeParticipant};

    async fn collect(
        name: &str, primitives: &dyn Primitives, artifacts: &[CollectArtifact], log_dir: &Path,
        report: &Mutex<CollectionReport>, cancel: &CancellationToken,
    ) {
        Collector {
            primitives,
            operations: &[],
            requests: BTreeMap::new(),
        }
        .collect(name, artifacts, log_dir, report, cancel)
        .await;
    }

    #[tokio::test]
    async fn collection_retains_successes_and_reports_missing_files_and_read_and_write_failures() {
        let root = tempfile::tempdir().unwrap();
        let fake = FakeParticipant::new(&[]);
        fake.set_file("/available", Ok(Some(vec![0, 255])));
        fake.set_file("/missing", Ok(None));
        fake.set_file(
            "/unreadable",
            Err(HandleError::Harness {
                message: "read mechanism failed".into(),
            }),
        );
        let files = [
            ("/available", "good"),
            ("/available", "good"),
            ("/missing", "missing"),
            ("/unreadable", "unreadable"),
            ("/available", "../outside"),
        ]
        .map(|(path, name)| CollectArtifact {
            source: CollectSource::File(path),
            name,
            kind: "sample",
            destination: CollectDestination::Traffic,
        });
        let report = Mutex::new(CollectionReport::default());
        collect(
            "subject",
            &fake,
            &files,
            root.path(),
            &report,
            &CancellationToken::new(),
        )
        .await;
        let report = report.into_inner().unwrap();
        assert_eq!(
            report.collected,
            [Collected {
                participant: "subject".into(),
                kind: "sample".into(),
                path: root.path().join("traffic/good"),
            }]
        );
        assert_eq!(std::fs::read(&report.collected[0].path).unwrap(), [0, 255]);
        assert_eq!(report.issues.len(), 4);
        for (issue, reason) in report.issues.iter().zip([
            "cannot create artifact",
            "is missing",
            "read mechanism failed",
            "plain filename",
        ]) {
            assert_eq!(issue.kind, IssueKind::CollectionFailed);
            assert!(issue.message.contains("participant 'subject'"), "{issue:?}");
            assert!(issue.message.contains(reason), "{issue:?}");
        }
        assert!(!root.path().join("outside").exists());
    }

    #[tokio::test]
    async fn cancelled_collection_records_an_issue_without_creating_an_artifact() {
        let root = tempfile::tempdir().unwrap();
        let fake = FakeParticipant::new(&[]);
        fake.set_file("/available", Ok(Some(vec![0, 255])));
        let files = [CollectArtifact {
            source: CollectSource::File("/available"),
            name: "output",
            kind: "sample",
            destination: CollectDestination::Participant,
        }];
        let report = Mutex::new(CollectionReport::default());
        let cancel = CancellationToken::new();
        cancel.cancel();
        collect("subject", &fake, &files, root.path(), &report, &cancel).await;
        let report = report.into_inner().unwrap();
        assert!(report.collected.is_empty());
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].kind, IssueKind::CollectionFailed);
        assert!(report.issues[0].message.contains("cancelled"));
        assert!(!root.path().join("subject/collected/output").exists());
    }

    struct Export;

    #[async_trait::async_trait]
    impl Operation for Export {
        fn name(&self) -> &'static str {
            "export"
        }
        async fn run(&self, cx: &OpContext<'_>, args: serde_json::Value) -> Result<serde_json::Value, HandleError> {
            assert_eq!(args, serde_json::Value::Null);
            let output = cx.exec(vec!["export".into()]).await?;
            Ok(serde_json::json!({"output": String::from_utf8(output.stdout).unwrap()}))
        }
    }

    const EXPORT: CollectArtifact = CollectArtifact {
        source: CollectSource::Operation("export"),
        name: "snapshot.json",
        kind: "snapshot",
        destination: CollectDestination::ParticipantTraffic,
    };

    #[tokio::test]
    async fn operation_collections_preserve_json_and_keep_participants_separate() {
        let root = tempfile::tempdir().unwrap();
        let fake = FakeParticipant::new(&[]);
        fake.push_exec(Ok(crate::handle::ExecOutput {
            exit_code: 0,
            stdout: b"sample".to_vec(),
            stderr: vec![],
        }));
        let report = Mutex::new(CollectionReport::default());
        let collector = Collector {
            primitives: &fake,
            operations: &[&Export],
            requests: BTreeMap::new(),
        };
        for name in ["first", "second"] {
            collector
                .collect(name, &[EXPORT], root.path(), &report, &CancellationToken::new())
                .await;
        }
        let report = report.into_inner().unwrap();
        assert!(report.issues.is_empty(), "{:?}", report.issues);
        assert_eq!(report.collected.len(), 2);
        for (artifact, name) in report.collected.iter().zip(["first", "second"]) {
            assert_eq!(artifact.participant, name);
            assert_eq!(artifact.kind, "snapshot");
            assert_eq!(
                artifact.path,
                root.path().join("traffic").join(name).join("snapshot.json")
            );
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&std::fs::read(&artifact.path).unwrap()).unwrap(),
                serde_json::json!({"output": "sample"})
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn operation_collection_errors_timeouts_and_cancellation_are_issues_not_artifacts() {
        for mode in ["error", "timeout", "cancel", "unknown"] {
            let root = tempfile::tempdir().unwrap();
            let mut fake = FakeParticipant::new(&[]);
            fake.push_exec(Err(HandleError::Product {
                message: "export unavailable".into(),
            }));
            if mode == "timeout" {
                fake.delay_exec(COLLECTION_TIME * 2);
            }
            let cancel = CancellationToken::new();
            if mode == "cancel" {
                cancel.cancel();
            }
            let report = Mutex::new(CollectionReport::default());
            let collector = Collector {
                primitives: &fake,
                operations: if mode == "unknown" { &[] } else { &[&Export] },
                requests: BTreeMap::new(),
            };
            let started = tokio::time::Instant::now();
            collector
                .collect("subject", &[EXPORT], root.path(), &report, &cancel)
                .await;
            let report = report.into_inner().unwrap();
            assert!(report.collected.is_empty());
            assert!(!root.path().join("traffic").exists());
            assert_eq!(report.issues.len(), 1);
            assert_eq!(report.issues[0].kind, IssueKind::CollectionFailed);
            let expected = match mode {
                "error" => "export unavailable",
                "timeout" => "exceeded 5s",
                "cancel" => "cancelled",
                "unknown" => "operation 'export' is not offered",
                _ => unreachable!(),
            };
            assert!(report.issues[0].message.contains(expected), "{:?}", report.issues);
            assert_eq!(
                started.elapsed(),
                if mode == "timeout" {
                    COLLECTION_TIME
                } else {
                    Duration::ZERO
                }
            );
        }
    }

    struct PendingReader;

    #[async_trait::async_trait]
    impl Primitives for PendingReader {
        async fn read_file(&self, _: &str) -> Result<Option<Vec<u8>>, HandleError> {
            std::future::pending().await
        }
        async fn stat(&self, _: &str) -> Result<Option<crate::handle::FileStatus>, HandleError> {
            unreachable!()
        }
        async fn exec(
            &self, _: crate::handle::ExecCommand, _: Duration,
        ) -> Result<crate::handle::ExecOutput, HandleError> {
            unreachable!()
        }
        async fn probe(
            &self, _: &crate::endpoint::ResolvedAddress, _: &crate::probe::AttemptLimits,
        ) -> Result<crate::probe::ProbeAnswer, HandleError> {
            unreachable!()
        }
        async fn send(
            &self, _: &crate::endpoint::ResolvedAddress, _: &[u8], _: &crate::probe::AttemptLimits,
        ) -> Result<String, HandleError> {
            unreachable!()
        }
        async fn http(
            &self, _: &crate::endpoint::ResolvedAddress, _: &str, _: bool, _: &crate::probe::AttemptLimits,
        ) -> Result<crate::handle::HttpResponse, HandleError> {
            unreachable!()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn collection_times_out_a_stalled_read_and_keeps_the_report() {
        let root = tempfile::tempdir().unwrap();
        let report = Mutex::new(CollectionReport::default());
        let started = tokio::time::Instant::now();
        collect(
            "subject",
            &PendingReader,
            &[CollectArtifact {
                source: CollectSource::File("/pending"),
                name: "output",
                kind: "sample",
                destination: CollectDestination::Participant,
            }],
            root.path(),
            &report,
            &CancellationToken::new(),
        )
        .await;
        assert_eq!(started.elapsed(), COLLECTION_TIME);
        let report = report.into_inner().unwrap();
        assert!(report.collected.is_empty());
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].kind, IssueKind::CollectionFailed);
        assert!(report.issues[0].message.contains("exceeded 5s"));
    }
}
