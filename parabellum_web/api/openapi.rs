use axum::Json;
use serde_json::{Value, json};

/// Returns a minimal OpenAPI document for `/api/v1`.
///
/// This is intentionally lightweight for the first hardening pass; endpoint
/// coverage can be expanded incrementally without changing the route contract.
pub async fn openapi_spec() -> Json<Value> {
    Json(json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Parabellum API",
            "version": env!("CARGO_PKG_VERSION")
        },
        "servers": [
            { "url": "/api/v1" }
        ],
        "paths": {}
    }))
}
