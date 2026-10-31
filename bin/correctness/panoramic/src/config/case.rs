//! A [`CaseConfig`] is one runnable case loaded from a `config.yaml`.
//!
//! It records the name, description, suite, source file, timeout and declared subject operating systems.
//! The parsed integration or correctness data stays private. [`CaseConfig::build_test`] builds runners
//! from copies, so image overrides leave the loaded case unchanged.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::Duration,
};

use saluki_error::{generic_error, ErrorContext as _, GenericError};

use super::{load_case, IntegrationConfig, MatrixVariants, LINUX_RUNTIME, MAC_RUNTIME, WINDOWS_RUNTIME};
use crate::{
    correctness::{case::CorrectnessTestCase, config::Config as CorrectnessConfig},
    image_override::ImageOverrides,
    integration::{IntegrationSettings, IntegrationTestCase},
    test::{Test, TestSuite},
};

/// An operating system a case's processes run on.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Os {
    Linux,
    #[allow(
        clippy::enum_variant_names,
        reason = "macOS is one word; the Os suffix is part of its name."
    )]
    MacOs,
    Windows,
}

impl Os {
    /// Maps an integration runtime identifier to its OS.
    pub(crate) fn from_runtime(runtime: &str) -> Option<Self> {
        match runtime {
            LINUX_RUNTIME => Some(Self::Linux),
            MAC_RUNTIME => Some(Self::MacOs),
            WINDOWS_RUNTIME => Some(Self::Windows),
            _ => None,
        }
    }
}

/// Time budget for correctness cases, which run two isolation groups of several containers each.
pub(crate) const CORRECTNESS_TIMEOUT: Duration = Duration::from_mins(20);

/// Immutable case metadata and the file data used to build its runners.
pub(crate) struct CaseConfig {
    name: String,
    description: Option<String>,
    suite: TestSuite,
    source: PathBuf,
    timeout: Duration,
    subject_os: BTreeSet<Os>,
    disabled: bool,
    schema: CaseSchema,
}

/// Parsed case data, with each correctness matrix variant expanded into its own case.
enum CaseSchema {
    Integration(IntegrationConfig),
    Correctness(CorrectnessConfig),
}

impl CaseConfig {
    /// Loads every case a file declares, including disabled cases and individual matrix variants.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or resolved, contains invalid YAML, omits `type`,
    /// names an unknown type, or fails schema deserialization. Integration runtimes must be nonempty
    /// and recognized; a correctness matrix must have at least one variant.
    pub(crate) fn load_file(config_path: &Path) -> Result<Vec<Self>, GenericError> {
        let content = std::fs::read_to_string(config_path)
            .error_context(format!("Failed to read config file: {}", config_path.display()))?;

        let peek: serde_json::Value = serde_saphyr::from_str(&content).error_context(format!(
            "Failed to parse config file as YAML: {}",
            config_path.display()
        ))?;

        let test_type = peek.get("type").and_then(|v| v.as_str()).ok_or_else(|| {
            generic_error!(
                "Missing required 'type' field (expected 'integration', 'correctness', or 'correctness_matrix')"
            )
        })?;
        let dir_name = config_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");

        match test_type {
            "integration" => {
                let config: IntegrationConfig = load_case(config_path)?;
                if config.runtimes.is_empty() {
                    return Err(generic_error!(
                        "integration test '{}' has empty runtimes list",
                        config.name
                    ));
                }
                let subject_os = config
                    .runtimes
                    .iter()
                    .map(|runtime| {
                        Os::from_runtime(runtime).ok_or_else(|| {
                            generic_error!(
                                "integration test '{}' declares unknown runtime '{}' (expected '{}', '{}', or '{}')",
                                config.name,
                                runtime,
                                LINUX_RUNTIME,
                                MAC_RUNTIME,
                                WINDOWS_RUNTIME
                            )
                        })
                    })
                    .collect::<Result<BTreeSet<_>, _>>()?;
                Ok(vec![Self {
                    name: config.name.clone(),
                    description: config.description.clone(),
                    suite: TestSuite::Integration,
                    source: config.loaded_from.clone(),
                    timeout: config.timeout.0,
                    subject_os,
                    disabled: false,
                    schema: CaseSchema::Integration(config),
                }])
            }
            "correctness" => {
                let config = load_case(config_path)?;
                Ok(vec![Self::from_correctness(dir_name.to_string(), config)])
            }
            "correctness_matrix" => {
                // Parse each schema from YAML text; flattening or a Value round-trip changes scalar coercion.
                let base: CorrectnessConfig = load_case(config_path)?;
                let matrix: MatrixVariants = serde_saphyr::from_str(&content)
                    .error_context(format!("Failed to parse configuration file: {}", config_path.display()))?;
                if matrix.variants.is_empty() {
                    return Err(generic_error!(
                        "correctness_matrix '{}' has no variants defined",
                        dir_name
                    ));
                }
                Ok(matrix
                    .expand(&base, dir_name)
                    .into_iter()
                    .map(|(name, config)| Self::from_correctness(name, config))
                    .collect())
            }
            other => Err(generic_error!(
                "Unknown test type '{}' (expected 'integration', 'correctness', or 'correctness_matrix')",
                other
            )),
        }
    }

    fn from_correctness(name: String, config: CorrectnessConfig) -> Self {
        Self {
            name,
            description: None,
            suite: TestSuite::Correctness,
            source: config.loaded_from.clone(),
            timeout: CORRECTNESS_TIMEOUT,
            subject_os: BTreeSet::from([Os::Linux]),
            disabled: config.disabled,
            schema: CaseSchema::Correctness(config),
        }
    }

    /// Scopes integration cases to their declared OSes; enabled correctness cases accept every runtime.
    pub(super) fn is_eligible_for(&self, runtime: &str) -> bool {
        if self.disabled {
            return false;
        }
        match self.schema {
            CaseSchema::Integration(_) => Os::from_runtime(runtime).is_some_and(|os| self.subject_os.contains(&os)),
            CaseSchema::Correctness(_) => true,
        }
    }

    /// Builds a runner from a copy of the case data, applying image choices to that copy.
    pub(super) fn build_test(&self, settings: &IntegrationSettings, overrides: &ImageOverrides<'_>) -> Box<dyn Test> {
        match &self.schema {
            CaseSchema::Integration(config) => Box::new(IntegrationTestCase::new(config.clone(), settings)),
            CaseSchema::Correctness(config) => {
                let mut config = config.clone();
                overrides.apply_correctness(&mut config);
                Box::new(CorrectnessTestCase::new(self.name.clone(), config))
            }
        }
    }
}

// TODO: remove the dead_code allowance once list and run read case metadata from CaseConfig.
#[allow(dead_code)]
impl CaseConfig {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub(crate) fn suite(&self) -> TestSuite {
        self.suite
    }

    pub(crate) fn source(&self) -> &Path {
        &self.source
    }

    pub(crate) fn case_dir(&self) -> &Path {
        self.source
            .parent()
            .expect("a canonical config file path has a parent directory")
    }

    pub(crate) fn timeout(&self) -> Duration {
        self.timeout
    }

    pub(crate) fn subject_os(&self) -> &BTreeSet<Os> {
        &self.subject_os
    }

    pub(crate) fn disabled(&self) -> bool {
        self.disabled
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::config::default_host_runtime;

    const CORRECTNESS_FIELDS: &str =
        "runtime: docker\nanalysis_mode: metrics\nbaseline: {image: 'base:file'}\ncomparison: {image: 'comp:file'}\n";

    fn write_case(root: &Path, name: &str, yaml: &str) -> PathBuf {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.yaml");
        fs::write(&path, yaml).unwrap();
        path
    }

    #[test]
    fn integration_case_records_its_declared_name_description_timeout_and_subject_os() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_case(
            dir.path(),
            "directory",
            "type: integration\nname: alpha\ndescription: A portable case\ntimeout: 7s\nruntimes: [linux, mac]\nprocedure: []\n",
        );
        let cases = CaseConfig::load_file(&path).unwrap();
        assert_eq!(cases.len(), 1);
        let case = &cases[0];
        assert_eq!(case.name(), "alpha");
        assert_eq!(case.description(), Some("A portable case"));
        assert_eq!(case.timeout(), Duration::from_secs(7));
        assert_eq!(case.suite(), TestSuite::Integration);
        assert_eq!(case.subject_os(), &BTreeSet::from([Os::Linux, Os::MacOs]));
        assert!(!case.disabled());
        let canonical = path.canonicalize().unwrap();
        assert_eq!(case.source(), canonical);
        assert_eq!(case.case_dir(), canonical.parent().unwrap());
    }

    #[test]
    fn correctness_case_takes_its_name_from_the_directory_and_the_fixed_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_case(dir.path(), "beta", &format!("type: correctness\n{CORRECTNESS_FIELDS}"));
        let cases = CaseConfig::load_file(&path).unwrap();
        assert_eq!(cases.len(), 1);
        let case = &cases[0];
        assert_eq!(case.name(), "beta");
        assert_eq!(case.description(), None);
        assert_eq!(case.timeout(), CORRECTNESS_TIMEOUT);
        assert_eq!(case.timeout(), Duration::from_mins(20));
        assert_eq!(case.suite(), TestSuite::Correctness);
        assert_eq!(case.subject_os(), &BTreeSet::from([Os::Linux]));
        assert!(!case.disabled());
    }

    #[test]
    fn matrix_file_yields_one_case_per_variant_in_declaration_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_case(
            dir.path(),
            "matrix",
            &format!(
                "type: correctness_matrix\n{CORRECTNESS_FIELDS}\nvariants:\n  - name: second\n    env: {{VARIANT: second}}\n  - name: first\n    env: {{VARIANT: first}}\n"
            ),
        );
        let cases = CaseConfig::load_file(&path).unwrap();
        assert_eq!(
            cases.iter().map(CaseConfig::name).collect::<Vec<_>>(),
            ["matrix/second", "matrix/first"]
        );
        let source = path.canonicalize().unwrap();
        let overrides = ImageOverrides::new(&[]);
        let settings = IntegrationSettings::new(LINUX_RUNTIME, &overrides);
        for (case, variant) in cases.iter().zip(["second", "first"]) {
            assert_eq!(case.source(), source);
            assert_eq!(case.timeout(), CORRECTNESS_TIMEOUT);
            assert_eq!(case.suite(), TestSuite::Correctness);
            assert_eq!(case.description(), None);
            assert_eq!(case.subject_os(), &BTreeSet::from([Os::Linux]));
            assert!(!case.disabled());
            let runner = case.build_test(&settings, &overrides);
            assert_eq!(runner.name(), format!("matrix/{variant}"));
            assert_eq!(runner.case_path(), source.parent().unwrap());

            // Check the variant data build_test consumes; CorrectnessTestCase exposes its target environment.
            let CaseSchema::Correctness(config) = &case.schema else {
                panic!("expected correctness data");
            };
            let runner = CorrectnessTestCase::new(case.name().to_string(), config.clone());
            for target in [&runner.config.baseline, &runner.config.comparison] {
                let (target, _) = runner.target_config(target).unwrap();
                assert_eq!(target.additional_env_vars, [format!("VARIANT={variant}")]);
            }
        }
    }

    #[test]
    fn disabled_correctness_cases_load_but_are_never_eligible() {
        let dir = tempfile::tempdir().unwrap();
        for (kind, variants, names) in [
            ("correctness", "", vec!["disabled"]),
            (
                "correctness_matrix",
                "variants: [{name: second}, {name: first}]\n",
                vec!["disabled/second", "disabled/first"],
            ),
        ] {
            let path = write_case(
                dir.path(),
                "disabled",
                &format!("type: {kind}\n{CORRECTNESS_FIELDS}disabled: true\n{variants}"),
            );
            let cases = CaseConfig::load_file(&path).unwrap();
            assert_eq!(cases.iter().map(CaseConfig::name).collect::<Vec<_>>(), names);
            for case in cases {
                assert!(case.disabled());
                for runtime in [LINUX_RUNTIME, MAC_RUNTIME, WINDOWS_RUNTIME, "unknown"] {
                    assert!(!case.is_eligible_for(runtime), "{kind} under {runtime}");
                }
            }
        }
    }

    #[test]
    fn eligibility_scopes_integration_cases_to_their_runtimes_and_not_correctness_cases() {
        let dir = tempfile::tempdir().unwrap();
        let integration = write_case(
            dir.path(),
            "portable",
            "type: integration\nname: portable\ntimeout: 1s\nruntimes: [linux, windows]\nprocedure: []\n",
        );
        let correctness = write_case(
            dir.path(),
            "correctness",
            &format!("type: correctness\n{CORRECTNESS_FIELDS}"),
        );
        let integration = CaseConfig::load_file(&integration).unwrap();
        let correctness = CaseConfig::load_file(&correctness).unwrap();
        assert_eq!(integration.len(), 1);
        assert_eq!(correctness.len(), 1);
        assert_eq!(integration[0].subject_os(), &BTreeSet::from([Os::Linux, Os::Windows]));
        for (runtime, eligible) in [
            (LINUX_RUNTIME, true),
            (MAC_RUNTIME, false),
            (WINDOWS_RUNTIME, true),
            ("unknown", false),
        ] {
            assert_eq!(integration[0].is_eligible_for(runtime), eligible, "{runtime}");
            assert!(correctness[0].is_eligible_for(runtime), "{runtime}");
        }
    }

    #[test]
    fn load_file_rejects_missing_and_unknown_types_and_empty_matrices() {
        let dir = tempfile::tempdir().unwrap();
        for (yaml, expected) in [
            ("name: missing\n".to_string(), "Missing required 'type' field"),
            ("type: bogus\n".to_string(), "Unknown test type 'bogus'"),
            (
                format!("type: correctness_matrix\n{CORRECTNESS_FIELDS}variants: []\n"),
                "has no variants defined",
            ),
        ] {
            let path = write_case(dir.path(), "invalid", &yaml);
            let error = CaseConfig::load_file(&path).err().expect("invalid case must fail");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn source_is_canonical_when_the_file_is_named_through_a_non_canonical_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_case(dir.path(), "case", &format!("type: correctness\n{CORRECTNESS_FIELDS}"));
        let cases = CaseConfig::load_file(&dir.path().join("case/../case/config.yaml")).unwrap();
        assert_eq!(cases.len(), 1);
        let case = &cases[0];
        assert_eq!(case.source(), path.canonicalize().unwrap());
        assert_eq!(case.name(), "case");
        let overrides = ImageOverrides::new(&[]);
        let settings = IntegrationSettings::new(LINUX_RUNTIME, &overrides);
        let runner = case.build_test(&settings, &overrides);
        assert_eq!(runner.case_path(), path.canonicalize().unwrap().parent().unwrap());
    }

    #[test]
    fn image_overrides_change_the_built_runner_but_not_the_loaded_case() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_case(dir.path(), "case", &format!("type: correctness\n{CORRECTNESS_FIELDS}"));
        let cases = CaseConfig::load_file(&path).unwrap();
        assert_eq!(cases.len(), 1);
        let case = &cases[0];
        let entries = ["baseline=base:override".parse().unwrap()];
        let overrides = ImageOverrides::new(&entries);
        let settings = IntegrationSettings::new(LINUX_RUNTIME, &overrides);
        let overridden = case.build_test(&settings, &overrides);
        let declared = case.build_test(&settings, &ImageOverrides::new(&[]));
        assert_eq!(overridden.images()["baseline"], "base:override");
        assert_eq!(declared.images()["baseline"], "base:file");
    }

    #[test]
    fn facade_metadata_matches_the_built_runner_for_every_checked_in_case() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let overrides = ImageOverrides::new(&[]);
        for suite in ["integration", "correctness"] {
            let mut loaded = 0;
            let mut built = 0;
            for entry in fs::read_dir(root.join("test").join(suite).join("cases")).unwrap() {
                let dir = entry.unwrap().path();
                let path = dir.join("config.yaml");
                if !dir.is_dir() || !path.is_file() {
                    continue;
                }
                let cases = CaseConfig::load_file(&path).unwrap();
                assert!(!cases.is_empty(), "{}", path.display());
                loaded += cases.len();
                let yaml: serde_json::Value = serde_saphyr::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
                for case in cases {
                    if yaml["type"] == "integration" {
                        let declared_os: BTreeSet<_> = match yaml.get("runtimes") {
                            Some(runtimes) => runtimes
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|runtime| Os::from_runtime(runtime.as_str().unwrap()).unwrap())
                                .collect(),
                            None => BTreeSet::from([Os::from_runtime(default_host_runtime()).unwrap()]),
                        };
                        assert_eq!(case.subject_os(), &declared_os, "{}", path.display());
                    }
                    for runtime in [LINUX_RUNTIME, MAC_RUNTIME, WINDOWS_RUNTIME] {
                        if !case.is_eligible_for(runtime) {
                            continue;
                        }
                        let settings = IntegrationSettings::new(runtime, &overrides);
                        let runner = case.build_test(&settings, &overrides);
                        assert_eq!(case.name(), runner.name(), "{} under {runtime}", path.display());
                        assert_eq!(case.suite(), runner.suite(), "{} under {runtime}", path.display());
                        assert_eq!(
                            case.description(),
                            runner.description().as_deref(),
                            "{} under {runtime}",
                            path.display()
                        );
                        assert_eq!(
                            case.case_dir(),
                            runner.case_path(),
                            "{} under {runtime}",
                            path.display()
                        );
                        assert_eq!(case.timeout(), runner.timeout(), "{} under {runtime}", path.display());
                        built += 1;
                    }
                }
            }
            assert!(loaded > 0, "{suite} must contain cases");
            assert!(built > 0, "{suite} must contain eligible cases");
        }
    }

    #[test]
    fn disabled_correctness_cases_still_reject_invalid_environments() {
        let dir = tempfile::tempdir().unwrap();
        for (kind, fields, variants) in [
            (
                "correctness",
                CORRECTNESS_FIELDS.replace(
                    "baseline: {image: 'base:file'}",
                    "baseline: {image: 'base:file', env: {INVALID: true}}",
                ),
                "",
            ),
            (
                "correctness_matrix",
                CORRECTNESS_FIELDS.to_string(),
                "variants: [{name: invalid, env: {INVALID: true}}]\n",
            ),
        ] {
            let path = write_case(
                dir.path(),
                "invalid",
                &format!("type: {kind}\n{fields}disabled: true\n{variants}"),
            );
            let error = CaseConfig::load_file(&path)
                .err()
                .expect("disabled invalid case must fail");
            assert!(
                format!("{error:?}").contains("environment variable 'INVALID' must be a YAML string"),
                "{error:?}"
            );
        }
    }

    #[test]
    fn load_file_reports_read_yaml_and_schema_failures() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing/config.yaml");
        let error = CaseConfig::load_file(&missing).err().expect("missing file must fail");
        assert_eq!(
            error.to_string(),
            format!("Failed to read config file: {}", missing.display())
        );
        for (yaml, context) in [
            ("type: [", "Failed to parse config file as YAML"),
            ("type: integration\n", "Failed to parse configuration file"),
        ] {
            let path = write_case(dir.path(), "invalid", yaml).canonicalize().unwrap();
            let error = CaseConfig::load_file(&path).err().expect("invalid file must fail");
            assert_eq!(error.to_string(), format!("{context}: {}", path.display()));
        }
    }

    #[cfg(unix)]
    #[test]
    fn correctness_names_use_the_supplied_directory_even_when_it_is_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_case(
            dir.path(),
            "original",
            &format!("type: correctness\n{CORRECTNESS_FIELDS}"),
        );
        std::os::unix::fs::symlink(dir.path().join("original"), dir.path().join("alias")).unwrap();
        let cases = CaseConfig::load_file(&dir.path().join("alias/config.yaml")).unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].name(), "alias");
        assert_eq!(cases[0].source(), path.canonicalize().unwrap());
    }
}
