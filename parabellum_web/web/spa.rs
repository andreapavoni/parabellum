use axum::{
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::{collections::HashMap, fs};

const SPA_SHELL: &str = include_str!("../templates/spa_shell.html");
const MANIFEST_PATH: &str = "frontend/dist/.vite/manifest.json";
const ENTRYPOINT: &str = "frontend/src/main.tsx";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    file: String,
    #[serde(default)]
    css: Vec<String>,
}

pub async fn spa_shell() -> Response {
    let html = SPA_SHELL.replace("{{ assets }}", &asset_tags());
    let mut response = Html(html).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}

pub async fn spa_fallback(uri: axum::http::Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/assets/") || path.starts_with("/static/") || path_has_extension(path) {
        return StatusCode::NOT_FOUND.into_response();
    }
    spa_shell().await
}

fn asset_tags() -> String {
    let Ok(manifest_json) = fs::read_to_string(MANIFEST_PATH) else {
        return "<!-- frontend manifest not found; run bun run build:release -->".to_string();
    };
    let Ok(manifest) = serde_json::from_str::<HashMap<String, ManifestEntry>>(&manifest_json)
    else {
        return "<!-- frontend manifest is invalid -->".to_string();
    };
    let Some(entry) = manifest.get(ENTRYPOINT) else {
        return "<!-- frontend entrypoint missing from manifest -->".to_string();
    };

    let mut tags = String::new();
    for css in &entry.css {
        tags.push_str(&format!("    <link rel=\"stylesheet\" href=\"/{css}\">\n"));
    }
    tags.push_str(&format!(
        "    <script src=\"/{}\" type=\"module\"></script>",
        entry.file
    ));
    tags
}

fn path_has_extension(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
}
