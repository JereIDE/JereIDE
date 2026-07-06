#!/usr/bin/env bash
# Rebuild the static documentation site from docs_src/ into docs/.
#
# Usage: ./scripts/build-docs.sh
#
# Requires mkdocs and the material theme:
#   pip install --user mkdocs mkdocs-material
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v mkdocs >/dev/null 2>&1; then
    echo "error: mkdocs is not installed. Install with: pip install --user mkdocs mkdocs-material" >&2
    exit 1
fi

mkdocs build --clean

echo "Built docs into: $ROOT_DIR/docs/"
