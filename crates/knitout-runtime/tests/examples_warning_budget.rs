//! Warning-budget check over the `examples/` corpus.
//!
//! Parses every example `.k` file, validates it, runs the runtime analysis, and asserts there are
//! **no** runtime errors and at most `MAX_WARNINGS` runtime warnings across the whole corpus. The
//! point is to keep the diagnostics from being noisy on real programs; when it trips, the failure
//! message lists the count of each warning/error kind so it is clear what is firing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use knitout_runtime::analyze;
use knitout_weak_ast::{cst_to_program, parsed_to_valid};

/// The most runtime warnings the corpus is allowed to produce in total.
const MAX_WARNINGS: usize = 0;

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// The variant name of a `Debug`-formatted diagnostic (everything before the first ` ` or `{`).
fn kind(debug: &str) -> String {
    debug.split([' ', '{']).next().unwrap_or(debug).to_string()
}

// Ignored by default: it walks the whole 335-file corpus (hundreds of seconds) and currently
// trips far above the budget. Run it on demand to see the per-kind counts:
//   cargo test -p knitout-runtime --test examples_warning_budget -- --ignored --nocapture
#[test]
#[ignore = "slow corpus diagnostic; run with --ignored to see the warning counts"]
fn examples_stay_within_the_warning_budget() {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_knitout::LANGUAGE.into())
        .expect("load knitout grammar");

    let mut files = 0usize;
    let mut unvalidated = 0usize;
    let mut total_warnings = 0usize;
    let mut total_errors = 0usize;
    let mut warning_kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut error_kinds: BTreeMap<String, usize> = BTreeMap::new();

    let mut paths: Vec<PathBuf> = std::fs::read_dir(examples_dir())
        .expect("read examples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("k"))
        .collect();
    paths.sort();

    for path in &paths {
        let source = std::fs::read(path).expect("read example");
        let tree = parser.parse(&source, None).expect("parse example");
        let (program, _warnings, _errors) = cst_to_program(&source, &tree);
        let Some(valid) = parsed_to_valid(program) else {
            unvalidated += 1;
            continue;
        };
        files += 1;
        let diagnostics = analyze(&valid);
        total_warnings += diagnostics.warnings.len();
        total_errors += diagnostics.errors.len();
        for w in &diagnostics.warnings {
            *warning_kinds.entry(kind(&format!("{w:?}"))).or_default() += 1;
        }
        for e in &diagnostics.errors {
            *error_kinds.entry(kind(&format!("{e:?}"))).or_default() += 1;
        }
    }

    eprintln!("examples analyzed: {files} ({unvalidated} did not validate)");
    eprintln!("runtime errors:   {total_errors} {error_kinds:?}");
    eprintln!("runtime warnings: {total_warnings} {warning_kinds:?}");

    assert_eq!(
        total_errors, 0,
        "expected no runtime errors across the corpus, got {total_errors}: {error_kinds:?}"
    );
    // `MAX_WARNINGS` is a knob meant to be raised/lowered; the `<=` is intentional even at 0.
    #[allow(clippy::absurd_extreme_comparisons)]
    let within_budget = total_warnings <= MAX_WARNINGS;
    assert!(
        within_budget,
        "expected <= {MAX_WARNINGS} runtime warnings across the corpus, got {total_warnings}: {warning_kinds:?}"
    );
}
