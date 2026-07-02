#!/bin/bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

outdir="crates/tree-sitter-knitout/test/corpus/examples"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

# Run the real generator script but redirected into a scratch dir,
# so we don't clobber the committed expected-output sections.
sed "s#^outdir=.*#outdir=\"$tmpdir\"#" scripts/populate-tree-sitter-example-tests.sh > "$tmpdir/gen.sh"
bash "$tmpdir/gen.sh"

fail=0

# Everything through the first dashed separator (title + source),
# excluding whatever expected S-expression follows it.
header_block() {
  awk '
    /^--------------------------------------------------------------------------------$/ { print; exit }
    { print }
  ' "$1"
}

shopt -s nullglob

for genfile in "$tmpdir"/*.txt; do
  name="$(basename "$genfile")"
  existing="$outdir/$name"
  if [ ! -f "$existing" ]; then
    echo "::error::Missing corpus test '$name' — a new example was added but has no corpus test. Run scripts/populate-tree-sitter-example-tests.sh and add expected output."
    fail=1
    continue
  fi
  if ! diff -u <(header_block "$existing") <(header_block "$genfile") > "$tmpdir/diff.out"; then
    echo "::error::Corpus test '$name' has a stale title or source vs examples/ — regenerate it."
    cat "$tmpdir/diff.out"
    fail=1
  fi
done

for existingfile in "$outdir"/*.txt; do
  name="$(basename "$existingfile")"
  if [ ! -f "$tmpdir/$name" ]; then
    echo "::error::Corpus test '$name' has no matching file in examples/ — looks stale, remove it or restore the example."
    fail=1
  fi
done

exit "$fail"
