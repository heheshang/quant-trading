//! `export_openapi` — emit the live `ApiDoc` as a pretty-printed OpenAPI 3.1
//! JSON document on stdout.
//!
//! Used by:
//!   - `frontend/scripts/gen-types.sh` — `cargo run --bin export_openapi > /tmp/openapi.json`
//!     then `npx openapi-typescript` consumes it to produce
//!     `frontend/src/types/api-generated.ts`.
//!   - `docs/openapi.json` — committed for offline consumption (Postman /
//!     `openapi-typescript` in CI / a static site that hosts the spec).
//!   - CI diff job — if `git diff --exit-code docs/openapi.json` is non-empty,
//!     the spec changed; the workflow fails unless the diff is committed.
//!
//! Usage:
//!     cargo run --bin export_openapi > docs/openapi.json
//!
//! Exit codes:
//!     0  — spec emitted successfully
//!     1  — JSON serialization failed (effectively impossible; ApiDoc is
//!           built from in-memory types, but we keep the `Result` path
//!           for symmetry with `to_json`).

use quant_trading_backend::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let api = <ApiDoc as OpenApi>::openapi();
    let json = api
        .to_pretty_json()
        .expect("OpenAPI doc serializes to JSON (in-memory types only)");
    println!("{json}");
}
