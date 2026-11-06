//! Boundaries between the panoramic crates.
//!
//! Panoramic's core knows no product, its steps reach participants through handles, and only the participants and
//! the binary know products. Dependency guards keep the crates pointing the right way. Term guards keep the sources
//! of core and steps free of names that belong to participants.

use std::path::Path;

use crate::{
    graph::{DependencyKind, MemberDependency, Workspace},
    terms::{read_crate, relative_to, SourceText, Term, TermRule},
};

const PERMITTED: &[(&str, &[&str])] = &[
    (
        "panoramic",
        &["panoramic-core", "panoramic-participants", "panoramic-steps"],
    ),
    ("panoramic-core", &["panoramic-courier"]),
    ("panoramic-courier", &[]),
    ("panoramic-participants", &["panoramic-core"]),
    ("panoramic-steps", &["panoramic-core"]),
];

fn workspace() -> Workspace {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    Workspace::load(&manifest).expect("load the workspace package graph")
}

fn is_panoramic(name: &str) -> bool {
    name == "panoramic" || name.starts_with("panoramic-")
}

/// Keeps core and steps independent of product definitions by checking panoramic dependency directions.
/// Third-party dependencies are outside this check.
fn dependency_violations(members: &[String], dependencies: &[MemberDependency]) -> Vec<String> {
    let mut names: Vec<&str> = members.iter().map(String::as_str).collect();
    names.extend(PERMITTED.iter().map(|(name, _)| *name));
    names.sort_unstable();
    names.dedup();

    let mut problems = Vec::new();
    for name in names {
        if !members.iter().any(|member| member == name) {
            problems.push(format!("{name} is listed in PERMITTED but missing from the workspace"));
            continue;
        }
        let permitted = PERMITTED.iter().find(|(allowed_name, _)| *allowed_name == name);
        if is_panoramic(name) && permitted.is_none() {
            problems.push(format!("{name} is missing from PERMITTED"));
        }
        let allowed = permitted.map_or(&[][..], |(_, dependencies)| *dependencies);
        for MemberDependency { to, kind, .. } in dependencies.iter().filter(|dependency| dependency.from == name) {
            if !is_panoramic(to) {
                continue;
            }
            if !is_panoramic(name) {
                problems.push(format!(
                    "{name} has a {kind} dependency on {to}; only panoramic workspace crates may depend on panoramic crates"
                ));
            } else if !allowed.contains(&to.as_str()) {
                problems.push(format!(
                    "{name} has a {kind} dependency on {to}; permitted panoramic dependencies: {allowed:?}"
                ));
            }
        }
    }
    problems
}

struct Boundary {
    package: &'static str,
    rule: TermRule,
    guidance: &'static str,
}

const CORE: Boundary = Boundary {
    package: "panoramic-core",
    rule: TermRule {
        terms: &[
            Term::Text("agent"),
            Term::Text("adp"),
            Term::Text("datadog"),
            Term::Text("dogstatsd"),
            Term::Text("intake"),
            Term::Text("millstone"),
            Term::Text("otlp"),
            Term::Text("s6"),
            Term::Number("2049"),
            Term::Number("55001"),
        ],
        exact_exemptions: &[],
    },
    guidance: "Product knowledge belongs in panoramic-participants.",
};

const STEPS: Boundary = Boundary {
    package: "panoramic-steps",
    rule: TermRule {
        terms: &[
            Term::Text("docker"),
            Term::Text("pwsh"),
            Term::Text("powershell"),
            Term::Text("kube"),
            Term::Text("container"),
            Term::Text("host process"),
            Term::Text("host_process"),
            Term::Text("hostprocess"),
            Term::Text("/opt/"),
            Term::Text("/etc/"),
            Term::Text("/usr/"),
            Term::Text("/var/"),
            Term::Text("c:\\"),
            Term::CfgIdent("windows"),
            Term::CfgIdent("unix"),
            Term::CfgIdent("target_os"),
            Term::CfgIdent("target_family"),
            Term::Ident("Os"),
            Term::Ident("Realization"),
            Term::Ident("RealizationKind"),
            Term::Ident("Pod"),
            Term::Ident("pod"),
        ],
        exact_exemptions: STEPS_METRIC_PREFIX_EXEMPTIONS,
    },
    guidance:
        "Steps reach participants through handles and must not name realizations, host mechanisms or product paths.",
};

const STEPS_METRIC_PREFIX_EXEMPTIONS: &[(&str, &str)] = &[
    (
        "bin/correctness/panoramic/steps/src/compare/analysis/metrics/mod.rs",
        "|| name.starts_with(\"docker.\")",
    ),
    (
        "bin/correctness/panoramic/steps/src/compare/analysis/metrics/mod.rs",
        "|| name.starts_with(\"container.\")",
    ),
];

impl Boundary {
    #[track_caller]
    fn assert_sources(&self) {
        let workspace = workspace();
        let manifest = workspace
            .member(self.package)
            .unwrap_or_else(|error| panic!("{error}\n{}", self.guidance))
            .manifest_path()
            .parent()
            .expect("manifest must have a directory")
            .as_std_path()
            .to_path_buf();
        let crate_dir = relative_to(workspace.root(), &manifest);
        let files =
            read_crate(workspace.root(), &crate_dir).unwrap_or_else(|error| panic!("{error}\n{}", self.guidance));
        let problems = self.rule.violations(&files);
        assert!(problems.is_empty(), "{}\n{}", problems.join("\n"), self.guidance);
    }
}

#[test]
fn panoramic_crates_depend_only_in_permitted_directions() {
    let workspace = workspace();
    let problems = dependency_violations(&workspace.member_names(), &workspace.direct_member_dependencies());
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn panoramic_core_source_names_no_product() {
    CORE.assert_sources();
}

#[test]
fn panoramic_steps_source_names_no_realization_host_mechanism_or_product_path() {
    STEPS.assert_sources();
}

mod dependency_directions {
    use super::*;

    fn dependency(from: &str, to: &str, kind: DependencyKind) -> MemberDependency {
        MemberDependency {
            from: from.into(),
            to: to.into(),
            kind,
        }
    }

    fn members(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn workspace_members() -> Vec<String> {
        members(&[
            "airlock",
            "panoramic",
            "panoramic-core",
            "panoramic-courier",
            "panoramic-participants",
            "panoramic-steps",
        ])
    }

    fn workspace_dependencies() -> Vec<MemberDependency> {
        use DependencyKind::{Dev, Normal};
        vec![
            dependency("panoramic", "panoramic-core", Normal),
            dependency("panoramic", "panoramic-participants", Normal),
            dependency("panoramic", "panoramic-steps", Normal),
            dependency("panoramic", "panoramic-steps", Dev),
            dependency("panoramic-core", "airlock", Normal),
            dependency("panoramic-core", "panoramic-courier", Normal),
            dependency("panoramic-participants", "panoramic-core", Normal),
            dependency("panoramic-steps", "panoramic-core", Normal),
        ]
    }

    #[test]
    fn the_permitted_workspace_has_no_violations() {
        assert!(dependency_violations(&workspace_members(), &workspace_dependencies()).is_empty());
    }

    #[test]
    fn a_forbidden_panoramic_dependency_is_reported_for_every_dependency_kind() {
        use DependencyKind::{Build, Dev, Normal};
        let mut dependencies = workspace_dependencies();
        dependencies.extend([
            dependency("panoramic-steps", "panoramic-participants", Dev),
            dependency("panoramic-core", "panoramic-steps", Normal),
            dependency("panoramic-participants", "panoramic-steps", Build),
            dependency("panoramic-courier", "panoramic-core", Normal),
        ]);
        let mut members = workspace_members();
        members.reverse();
        dependencies.reverse();
        assert_eq!(
            dependency_violations(&members, &dependencies),
            [
                "panoramic-core has a normal dependency on panoramic-steps; permitted panoramic dependencies: [\"panoramic-courier\"]",
                "panoramic-courier has a normal dependency on panoramic-core; permitted panoramic dependencies: []",
                "panoramic-participants has a build dependency on panoramic-steps; permitted panoramic dependencies: [\"panoramic-core\"]",
                "panoramic-steps has a dev dependency on panoramic-participants; permitted panoramic dependencies: [\"panoramic-core\"]",
            ]
        );
    }

    #[test]
    fn a_workspace_crate_outside_panoramic_may_not_depend_on_panoramic_crates() {
        let mut dependencies = workspace_dependencies();
        dependencies.push(dependency("airlock", "panoramic-core", DependencyKind::Normal));
        assert_eq!(
            dependency_violations(&workspace_members(), &dependencies),
            ["airlock has a normal dependency on panoramic-core; only panoramic workspace crates may depend on panoramic crates"]
        );
    }

    #[test]
    fn every_panoramic_workspace_crate_must_have_a_permitted_entry() {
        let mut members = workspace_members();
        members.retain(|member| member != "panoramic-steps");
        members.push("panoramic-extra".into());
        assert_eq!(
            dependency_violations(&members, &workspace_dependencies()),
            [
                "panoramic-extra is missing from PERMITTED",
                "panoramic-steps is listed in PERMITTED but missing from the workspace",
            ]
        );
    }
}

mod source_terms {
    use std::fs;

    use super::*;

    fn violations(boundary: &Boundary, files: &[(&str, &str)]) -> Vec<String> {
        let files: Vec<_> = files.iter().map(|(path, text)| SourceText::new(*path, *text)).collect();
        boundary.rule.violations(&files)
    }

    #[test]
    fn a_core_term_is_reported_case_insensitively_with_its_file_and_line() {
        for newline in ["\n", "\r\n"] {
            let text = ["AgentAdp", "DATADOG", "ADP agent adp"].join(newline);
            assert_eq!(
                violations(&CORE, &[("core/src/example.rs", &text)]),
                [
                    "core/src/example.rs:1: names agent, adp",
                    "core/src/example.rs:2: names datadog",
                    "core/src/example.rs:3: names agent, adp",
                ]
            );
        }
    }

    #[test]
    fn a_comment_on_a_core_line_does_not_exempt_its_terms() {
        assert_eq!(
            violations(&CORE, &[("core/src/example.rs", "adp // required by an external API")]),
            ["core/src/example.rs:1: names adp"]
        );
    }

    #[test]
    fn numbers_and_identifiers_match_only_whole_tokens() {
        assert_eq!(
            violations(
                &CORE,
                &[(
                    "core/src/ports.rs",
                    "2049\n12049 20490\n55001\n155001 550010\nport2049_name\n"
                )]
            ),
            [
                "core/src/ports.rs:1: names 2049",
                "core/src/ports.rs:3: names 55001",
                "core/src/ports.rs:5: names 2049",
            ]
        );
        assert_eq!(
            violations(
                &STEPS,
                &[(
                    "steps/src/example.rs",
                    concat!(
                        "Os::Linux\n",
                        "use x::Os;\n",
                        "Cost OsString my_Os Os_name 1Os Os1 os OS\n",
                        "slice.windows(2)\n",
                        "#[cfg(windows)]\n",
                        "cfg!(windows)\n",
                        "#[cfg(not(windows))]\n",
                        "RealizationKind Realization Pod pod\n",
                        "RealizationKindExtra RealizationExtra Pods tripod pod_name\n",
                        "éOsé\n",
                    ),
                )]
            ),
            [
                "steps/src/example.rs:1: names Os",
                "steps/src/example.rs:2: names Os",
                "steps/src/example.rs:5: names windows",
                "steps/src/example.rs:6: names windows",
                "steps/src/example.rs:7: names windows",
                "steps/src/example.rs:8: names Realization, RealizationKind, Pod, pod",
                "steps/src/example.rs:10: names Os",
            ]
        );
    }

    #[test]
    fn host_os_cfg_identifiers_match_within_nested_conditions_and_macros() {
        // Literal parenthesized names miss nested cfg conditions; match identifiers across the line.
        for (line, expected) in [
            ("#[cfg(any(unix, windows))]", "windows, unix"),
            ("#[cfg(all(windows, test))]", "windows"),
            ("cfg!(windows)", "windows"),
            ("cfg! ( unix )", "unix"),
            ("#[cfg(not(windows))]", "windows"),
            ("#[cfg(target_os = \"linux\")]", "target_os"),
            ("#[cfg_attr(target_family = \"unix\", test)]", "unix, target_family"),
            ("#[cfg(feature = \"x\")] fn windows() {}", "windows"),
            ("#[cfg(windows_extra, my_unix, target_os2, my_target_family)]", ""),
            ("#[cfg(my_windows, unix2, my_target_os, target_family_extra)]", ""),
            ("slice.windows(2); let unix = target_os + target_family;", ""),
        ] {
            let expected = if expected.is_empty() {
                Vec::new()
            } else {
                vec![format!("steps/src/example.rs:1: names {expected}")]
            };
            assert_eq!(
                violations(&STEPS, &[("steps/src/example.rs", line)]),
                expected,
                "{line}"
            );
        }
    }

    #[test]
    fn violations_are_sorted_by_path_then_line() {
        assert_eq!(
            violations(
                &CORE,
                &[
                    ("core/src/z.rs", "adp\n"),
                    ("core/src/a.rs", "\nagent\n\n\n\n\n\n\n\n\nintake\n"),
                ]
            ),
            [
                "core/src/a.rs:2: names agent",
                "core/src/a.rs:11: names intake",
                "core/src/z.rs:1: names adp",
            ]
        );
    }

    #[test]
    fn crate_files_include_the_manifest_and_nested_assets_in_path_order() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("core/src/assets/nested")).unwrap();
        fs::write(dir.path().join("core/Cargo.toml"), "agent").unwrap();
        fs::write(dir.path().join("core/src/lib.rs"), "").unwrap();
        fs::write(dir.path().join("core/src/assets/nested/start.sh"), "adp").unwrap();
        fs::write(dir.path().join("core/src/assets/settings"), "otlp").unwrap();

        let files = read_crate(dir.path(), "core").unwrap();
        assert_eq!(
            files,
            [
                SourceText::new("core/Cargo.toml", "agent"),
                SourceText::new("core/src/assets/nested/start.sh", "adp"),
                SourceText::new("core/src/assets/settings", "otlp"),
                SourceText::new("core/src/lib.rs", ""),
            ]
        );
        assert_eq!(
            CORE.rule.violations(&files),
            [
                "core/Cargo.toml:1: names agent",
                "core/src/assets/nested/start.sh:1: names adp",
                "core/src/assets/settings:1: names otlp",
            ]
        );
    }

    #[test]
    fn crate_files_cover_the_whole_crate_except_target_directories() {
        // Starting at the crate root catches product and host names in every Cargo target and asset.
        for (relative, scanned) in [
            ("tests/nested/example.rs", true),
            ("benches/example.rs", true),
            ("examples/example.rs", true),
            ("build.rs", true),
            ("assets/settings", true),
            (".hidden/settings", true),
            ("target/debug/generated.rs", false),
            ("examples/target/generated.rs", false),
        ] {
            for (boundary, crate_dir, term) in [(&CORE, "core", "agent"), (&STEPS, "steps", "docker")] {
                let dir = tempfile::tempdir().unwrap();
                let root = dir.path().join(crate_dir);
                fs::create_dir_all(root.join("src")).unwrap();
                fs::write(root.join("Cargo.toml"), "").unwrap();
                fs::write(root.join("src/lib.rs"), "").unwrap();
                let path = root.join(relative);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, term).unwrap();

                let files = read_crate(dir.path(), crate_dir).unwrap();
                let expected = if scanned {
                    vec![format!("{crate_dir}/{relative}:1: names {term}")]
                } else {
                    Vec::new()
                };
                assert_eq!(boundary.rule.violations(&files), expected, "{crate_dir}: {relative}");
                assert_eq!(files.len(), if scanned { 3 } else { 2 }, "{relative}");

                if !scanned {
                    fs::write(&path, [0xff]).unwrap();
                    assert_eq!(read_crate(dir.path(), crate_dir).unwrap(), files);
                }
            }
        }
    }

    #[test]
    fn a_crate_directory_without_sources_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let expected = "core/src/lib.rs: required crate source file is missing or not a regular file";
        assert_eq!(read_crate(dir.path(), "core").unwrap_err().to_string(), expected);
        fs::create_dir_all(dir.path().join("core/src")).unwrap();
        fs::write(dir.path().join("core/Cargo.toml"), "").unwrap();
        assert_eq!(read_crate(dir.path(), "core").unwrap_err().to_string(), expected);
    }

    #[test]
    fn a_missing_manifest_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("core/src")).unwrap();
        fs::write(dir.path().join("core/src/lib.rs"), "").unwrap();
        let error = read_crate(dir.path(), "core").unwrap_err().to_string();
        assert!(error.starts_with("core/Cargo.toml: "), "{error}");
    }

    #[test]
    fn a_non_utf8_source_file_is_an_error() {
        for relative in ["src/assets/data.bin", "tests/data.bin", "build.rs"] {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("core");
            fs::create_dir_all(root.join("src")).unwrap();
            fs::write(root.join("Cargo.toml"), "").unwrap();
            fs::write(root.join("src/lib.rs"), "").unwrap();
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, [0xff]).unwrap();
            let error = read_crate(dir.path(), "core").unwrap_err().to_string();
            assert!(error.starts_with(&format!("core/{relative}: ")), "{error}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_and_special_files_are_errors() {
        use std::os::unix::{fs::symlink, net::UnixListener};

        for relative in ["src/linked", "tests", "target"] {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("core");
            fs::create_dir_all(root.join("src")).unwrap();
            fs::write(root.join("Cargo.toml"), "").unwrap();
            fs::write(root.join("src/lib.rs"), "").unwrap();
            let path = root.join(relative);
            let expected = format!("core/{relative}: expected a regular file or directory");

            symlink(root.join("src"), &path).unwrap();
            assert_eq!(read_crate(dir.path(), "core").unwrap_err().to_string(), expected);
            fs::remove_file(&path).unwrap();

            let _socket = UnixListener::bind(&path).unwrap();
            assert_eq!(read_crate(dir.path(), "core").unwrap_err().to_string(), expected);
        }
    }

    #[test]
    fn only_exact_metric_filter_prefix_lines_are_exempt() {
        let path = "bin/correctness/panoramic/steps/src/compare/analysis/metrics/mod.rs";
        for line in [
            r#"|| name.starts_with("docker.")"#,
            r#"|| name.starts_with("container.")"#,
        ] {
            assert!(violations(&STEPS, &[(path, line)]).is_empty());
            assert!(!violations(&STEPS, &[("steps/src/other.rs", line)]).is_empty());
            for changed in [
                format!("{line} // container"),
                line.replace("name", "path"),
                line.replace("docker.", "docker/").replace("container.", "container/"),
            ] {
                assert!(!violations(&STEPS, &[(path, &changed)]).is_empty());
            }
        }
    }

    #[test]
    fn metric_prefix_exemptions_require_both_the_exact_path_and_line() {
        for &(path, line) in STEPS_METRIC_PREFIX_EXEMPTIONS {
            assert!(violations(&STEPS, &[(path, line)]).is_empty());
            assert_eq!(violations(&STEPS, &[("steps/src/other.rs", line)]).len(), 1);
            assert_eq!(
                violations(&STEPS, &[(path, &format!("{line} // another use"))]).len(),
                1
            );
        }
    }

    #[test]
    fn relative_paths_use_forward_slashes() {
        let root = Path::new("root");
        assert_eq!(relative_to(root, &root.join("a").join("b")), "a/b");
    }
}
