//! Golden tests over the `examples/` corpus.
//!
//! For every example file this test:
//!   1. runs the tree-sitter → `Program<Parsed>` → `Program<Validated>` pipeline, asserting no
//!      parse warnings or errors and that validation yields `Some`;
//!   2. serializes the `Program<Validated>` to JSON and compares it to a committed golden;
//!   3. deserializes the golden back into a `Program<Validated>` and re-serializes it, confirming
//!      `Program` is fully (de)serializable and stable.
//!
//! Goldens are stored as zstd-compressed JSON (`<name>.json.zst`) — the uncompressed corpus would
//! be gigabytes. Decompression is cheap, so it happens on every test run; compression only happens
//! during regeneration, and `REGEN_GOLDEN` first decompresses the existing golden and skips the
//! (slow) re-compression when the JSON is unchanged. Regenerate with
//! `REGEN_GOLDEN=1 cargo test -p knitout-weak-ast --test examples_golden`.

use std::path::{Path, PathBuf};

use knitout_ast::{Program, Validated};
use knitout_weak_ast::{cst_to_program, parsed_to_valid};

/// zstd level used when (re-)writing goldens. Only affects committed file size and regeneration
/// time, never the comparison (which decompresses first). Kept low so regeneration stays fast; the
/// corpus is repetitive enough that even a low level compresses it dramatically.
const ZSTD_LEVEL: i32 = 3;

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

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
    let json = serde_json::to_vec(&validated)?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("example path has no file name")?;
    let golden_path = golden_dir().join(format!("{file_name}.json.zst"));

    // The current golden's decompressed JSON, if the golden exists.
    let existing = match std::fs::read(&golden_path) {
        Ok(compressed) => Some(zstd::decode_all(&compressed[..])?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };

    if std::env::var_os("REGEN_GOLDEN").is_some() {
        // Only pay for (slow) compression when the JSON actually changed.
        if existing.as_deref() != Some(json.as_slice()) {
            std::fs::create_dir_all(golden_dir())?;
            std::fs::write(&golden_path, zstd::encode_all(&json[..], ZSTD_LEVEL)?)?;
        }
        return Ok(());
    }

    let golden = existing.ok_or_else(|| {
        format!("missing golden {golden_path:?}; run with REGEN_GOLDEN=1 to create it")
    })?;

    if json != golden {
        return Err(format!(
            "serialized program does not match golden for {file_name}; run REGEN_GOLDEN=1 if intended"
        )
        .into());
    }

    // Confirm the golden deserializes back into a `Program<Validated>` and re-serializes identically.
    let restored: Program<Validated> = serde_json::from_slice(&golden)?;
    if serde_json::to_vec(&restored)? != golden {
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
