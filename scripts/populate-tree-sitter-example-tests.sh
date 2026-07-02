#!/bin/bash

# Make one tree-sitter parse test per example file.

outdir="crates/tree-sitter-knitout/test/corpus/examples"
mkdir -p "$outdir"

find examples -type f | sort | while read -r f; do
  name="$(basename "$f")"
  name="${name%.*}"

  {
    printf "================================================================================\n"
    printf "examples-%s\n" "$name"
    printf "================================================================================\n"

    cat "$f"

    printf "\n--------------------------------------------------------------------------------\n"
  } > "$outdir/${name}.txt"
done
