//! Golden tests over the `examples/` corpus.
//!
//! For *every* example file this test asserts that the tree-sitter → `Program<Parsed>` →
//! `Program<Validated>` pipeline runs with no parse warnings or errors and yields `Some`.
//!
//! For a curated subset of small examples (see `GOLDEN_SET`) it additionally checks the serialized
//! `Program<Validated>` against a committed golden JSON file, and round-trips that golden back
//! through `serde` to confirm `Program` is fully (de)serializable. Golden files for the whole
//! corpus would be gigabytes, so only the small ones are committed; regenerate them with
//! `REGEN_GOLDEN=1 cargo test -p knitout-weak-ast --test examples_golden`.

use std::path::{Path, PathBuf};

use knitout_ast::{Program, Validated};
use knitout_weak_ast::{cst_to_program, parsed_to_valid};

/// Example files (by file name) for which a golden JSON is committed and compared.
const GOLDEN_SET: &[&str] = &[
    "used-to-crash.k",
    "xfer-kick.k",
    "twist1x1.k",
    "small_square_jersey.k",
    "small_square_rib.k",
    "k.k",
    "small_square_jersey_bindoff.k",
    "small_square_stockinette.k",
    "rib2x2.k",
];

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Runs the full pipeline for one example and, if it is in `GOLDEN_SET`, compares against / round
/// trips its committed golden.
fn golden_test(path: &Path) -> datatest_stable::Result<()> {
    let source = std::fs::read(path)?;

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_knitout::LANGUAGE.into())?;
    let tree = parser
        .parse(&source, None)
        .ok_or("tree-sitter returned no tree")?;

    let (program, warnings, errors) = cst_to_program(&source, &tree);
    if !warnings.is_empty() {
        return Err(format!("expected no parse warnings, got {warnings:?}").into());
    }
    if !errors.is_empty() {
        return Err(format!("expected no parse errors, got {errors:?}").into());
    }

    let validated = parsed_to_valid(program).ok_or("program did not validate to Some")?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("example path has no file name")?;
    if !GOLDEN_SET.contains(&file_name) {
        // Pipeline-only coverage for the (many, large) non-curated examples.
        return Ok(());
    }

    let json = format!("{}\n", serde_json::to_string_pretty(&validated)?);
    let golden_path = golden_dir().join(format!("{file_name}.json"));

    if std::env::var_os("REGEN_GOLDEN").is_some() {
        std::fs::create_dir_all(golden_dir())?;
        std::fs::write(&golden_path, &json)?;
        return Ok(());
    }

    let golden = std::fs::read_to_string(&golden_path).map_err(|e| {
        format!("could not read golden {golden_path:?} ({e}); run with REGEN_GOLDEN=1 to create it")
    })?;

    if json != golden {
        return Err(format!(
            "serialized program does not match golden for {file_name}; run REGEN_GOLDEN=1 if intended"
        )
        .into());
    }

    // Confirm the golden deserializes back into a `Program<Validated>` and re-serializes identically.
    let restored: Program<Validated> = serde_json::from_str(&golden)?;
    if format!("{}\n", serde_json::to_string_pretty(&restored)?) != golden {
        return Err(format!(
            "golden for {file_name} is not stable under a (de)serialize round-trip"
        )
        .into());
    }

    Ok(())
}

datatest_stable::harness! {
    { test = golden_test, root = "../../examples", pattern = r".*\.k$" },
}
