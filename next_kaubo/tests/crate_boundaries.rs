use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn manifest_dependencies(manifest: &Path) -> BTreeSet<String> {
    let text = fs::read_to_string(manifest)
        .unwrap_or_else(|err| panic!("read {}: {}", manifest.display(), err));
    let mut deps = BTreeSet::new();
    let mut in_dependency_section = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();

        if line.starts_with('[') {
            in_dependency_section = matches!(
                line,
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
            );
            continue;
        }

        if !in_dependency_section || line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((name, _)) = line.split_once('=') {
            deps.insert(name.trim().to_string());
        }
    }

    deps
}

#[test]
fn contract_crates_do_not_depend_on_stage_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage_crates = [
        "kaubo-syntax",
        "kaubo-infer",
        "kaubo-ir",
        "kaubo-vm",
        "kaubo-pipeline",
    ];
    let contract_crates = [
        ("kaubo-ast", "crates/kaubo-ast/Cargo.toml"),
        ("kaubo-token", "crates/kaubo-token/Cargo.toml"),
        ("kaubo-cps", "crates/kaubo-cps/Cargo.toml"),
    ];

    for (crate_name, manifest_path) in contract_crates {
        let deps = manifest_dependencies(&root.join(manifest_path));
        let forbidden: Vec<&str> = stage_crates
            .iter()
            .copied()
            .filter(|name| deps.contains(*name))
            .collect();

        assert!(
            forbidden.is_empty(),
            "{crate_name} must not depend on stage crates; found {forbidden:?}"
        );
    }
}

#[test]
fn stage_crates_do_not_depend_on_other_stage_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage_crates = [
        ("kaubo-syntax", "crates/kaubo-syntax/Cargo.toml"),
        ("kaubo-infer", "crates/kaubo-infer/Cargo.toml"),
        ("kaubo-ir", "crates/kaubo-ir/Cargo.toml"),
        ("kaubo-vm", "crates/kaubo-vm/Cargo.toml"),
        ("kaubo-pipeline", "crates/kaubo-pipeline/Cargo.toml"),
    ];

    for (crate_name, manifest_path) in stage_crates {
        let deps = manifest_dependencies(&root.join(manifest_path));
        let forbidden: Vec<&str> = stage_crates
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| *name != crate_name)
            .filter(|name| deps.contains(*name))
            .collect();

        assert!(
            forbidden.is_empty(),
            "{crate_name} must not depend on other stage crates; found {forbidden:?}"
        );
    }
}
