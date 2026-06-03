#!/usr/bin/env bash
# gen-types.sh — regenerate `frontend/src/types/api-generated.ts` from the
# backend's live OpenAPI 3.1 spec.
#
# Strategy (in order):
#   1. If `docs/openapi.json` exists in the repo root, use it (fast, no Rust).
#   2. Otherwise, run `cargo run --bin export_openapi` and pipe stdout to
#      a temp file, then consume that.
#
# This is wrapped by the `gen:api` npm script. CI uses the same script.

set -euo pipefail

# Resolve the script's directory (frontend/) and the repo root (one level up).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$FRONTEND_DIR/.." && pwd)"
BACKEND_DIR="$REPO_ROOT/backend"
SPEC_PATH="${REPO_ROOT}/docs/openapi.json"
TMP_SPEC="$(mktemp -t openapi-XXXXXX.json)"
trap 'rm -f "$TMP_SPEC"' EXIT

cd "$FRONTEND_DIR"

# Pick the spec source: committed file → cargo export → fail.
if [[ -f "$SPEC_PATH" ]]; then
  echo "→ using committed spec at $SPEC_PATH"
  cp "$SPEC_PATH" "$TMP_SPEC"
else
  echo "→ no committed spec, exporting via cargo (this may take a minute)..."
  (cd "$BACKEND_DIR" && cargo run --quiet --bin export_openapi) > "$TMP_SPEC"
fi

# Sanity check — empty file would mean cargo failed silently.
if [[ ! -s "$TMP_SPEC" ]]; then
  echo "ERROR: generated spec is empty — cargo run --bin export_openapi failed" >&2
  exit 1
fi

# Run openapi-typescript. Writes to src/types/api-generated.ts.
mkdir -p src/types
npx --yes openapi-typescript "$TMP_SPEC" -o src/types/api-generated.ts
echo "✓ wrote src/types/api-generated.ts"
