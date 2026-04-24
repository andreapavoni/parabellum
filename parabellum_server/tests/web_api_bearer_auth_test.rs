mod test_utils;

use axum::http::StatusCode;

use parabellum_types::errors::ApplicationError;

use crate::test_utils::tests::{setup_http_client, setup_web_app};

#[tokio::test]
async fn test_protected_endpoint_requires_bearer_token() -> Result<(), ApplicationError> {
    let (_, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let response = client
        .get(format!("{base_url}/api/v1/village"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["code"], "unauthorized");

    Ok(())
}

#[tokio::test]
async fn test_protected_endpoint_rejects_invalid_bearer_token() -> Result<(), ApplicationError> {
    let (_, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let response = client
        .get(format!("{base_url}/api/v1/village"))
        .bearer_auth("invalid-token")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["code"], "unauthorized");

    Ok(())
}
