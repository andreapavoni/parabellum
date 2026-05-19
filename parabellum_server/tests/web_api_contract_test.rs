mod test_utils;

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

use parabellum_types::errors::ApplicationError;

use crate::test_utils::tests::{setup_http_client, setup_web_app, setup_web_app_with_seeded_user};

fn unique_identity() -> (String, String, String) {
    let id = Uuid::new_v4().simple().to_string();
    let short = &id[..10];
    let username = format!("user{short}");
    let email = format!("user{short}@example.com");
    let password = "Password123!".to_string();
    (username, email, password)
}

async fn login(
    client: &reqwest::Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> reqwest::Response {
    let payload = json!({
        "email": email,
        "password": password
    })
    .to_string();
    client
        .post(format!("{base_url}/api/v1/auth/token/login"))
        .header("content-type", "application/json")
        .body(payload)
        .send()
        .await
        .unwrap()
}

fn str_field<'a>(body: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| body.get(*key)?.as_str())
}

#[tokio::test]
async fn auth_login_happy_path_returns_tokens_and_me_context() -> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();

    let login_response = login(&client, &base_url, &seeded.email, "Password123!").await;
    let login_status = login_response.status();
    let login_text = login_response.text().await.unwrap();
    assert_eq!(login_status, StatusCode::OK, "login failed: {login_text}");
    let login_body: Value = serde_json::from_str(&login_text).unwrap();
    let access_token = str_field(&login_body, &["access_token", "accessToken"])
        .unwrap()
        .to_string();
    let refresh_token = str_field(&login_body, &["refresh_token", "refreshToken"])
        .unwrap()
        .to_string();
    assert!(!access_token.is_empty());
    assert!(!refresh_token.is_empty());
    assert_eq!(login_body["user"]["email"], seeded.email);

    let me_response = client
        .get(format!("{base_url}/api/v1/me/context"))
        .bearer_auth(access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(me_response.status(), StatusCode::OK);
    let me_body: Value = serde_json::from_str(&me_response.text().await.unwrap()).unwrap();
    assert_eq!(me_body["player"]["username"], seeded.username);
    let current_village = me_body
        .get("current_village")
        .or_else(|| me_body.get("currentVillage"))
        .unwrap();
    assert!(current_village["id"].is_number());
    Ok(())
}

#[tokio::test]
async fn auth_refresh_happy_path_rotates_refresh_token() -> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();

    let login_response = login(&client, &base_url, &seeded.email, "Password123!").await;
    let login_status = login_response.status();
    let login_text = login_response.text().await.unwrap();
    assert_eq!(login_status, StatusCode::OK, "login failed: {login_text}");
    let login_body: Value = serde_json::from_str(&login_text).unwrap();
    let refresh_token = str_field(&login_body, &["refresh_token", "refreshToken"])
        .unwrap()
        .to_string();

    let refresh_response = client
        .post(format!("{base_url}/api/v1/auth/refresh"))
        .header("content-type", "application/json")
        .body(json!({ "refreshToken": refresh_token }).to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_body: Value =
        serde_json::from_str(&refresh_response.text().await.unwrap()).unwrap();
    assert!(str_field(&refresh_body, &["access_token", "accessToken"]).is_some());
    assert!(str_field(&refresh_body, &["refresh_token", "refreshToken"]).is_some());
    assert_ne!(
        str_field(&refresh_body, &["refresh_token", "refreshToken"]),
        str_field(&login_body, &["refresh_token", "refreshToken"])
    );
    assert_eq!(refresh_body["user"]["email"], seeded.email);
    Ok(())
}

#[tokio::test]
async fn auth_login_validation_error_for_missing_email() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let response = client
        .post(format!("{base_url}/api/v1/auth/token/login"))
        .header("content-type", "application/json")
        .body(
            json!({
                "email": "",
                "password": "x"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unexpected login validation response: {text}"
    );
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["code"], "validation_error");
    assert_eq!(body["field_errors"]["email"], "Email is required");

    Ok(())
}

#[tokio::test]
async fn auth_refresh_validation_error_for_missing_refresh_token() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let response = client
        .post(format!("{base_url}/api/v1/auth/refresh"))
        .header("content-type", "application/json")
        .body(json!({ "refreshToken": "" }).to_string())
        .send()
        .await
        .unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unexpected refresh validation response: {text}"
    );
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["code"], "validation_error");
    assert_eq!(
        body["field_errors"]["refresh_token"],
        "Refresh token is required"
    );

    Ok(())
}

#[tokio::test]
async fn auth_logout_validation_error_for_missing_refresh_token() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let response = client
        .post(format!("{base_url}/api/v1/auth/token/logout"))
        .header("content-type", "application/json")
        .body(
            json!({
                "refreshToken": "",
                "allSessions": false
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unexpected logout validation response: {text}"
    );
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["code"], "validation_error");
    assert_eq!(
        body["field_errors"]["refresh_token"],
        "Refresh token is required"
    );

    Ok(())
}

#[tokio::test]
async fn protected_village_overview_requires_bearer_token() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let response = client
        .get(format!("{base_url}/api/v1/villages/1/overview"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let text = response.text().await.unwrap();
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["code"], "unauthorized");

    Ok(())
}

#[tokio::test]
async fn protected_me_context_rejects_invalid_bearer_token() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let response = client
        .get(format!("{base_url}/api/v1/me/context"))
        .bearer_auth("invalid-token")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let text = response.text().await.unwrap();
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["code"], "unauthorized");

    Ok(())
}

#[tokio::test]
async fn auth_refresh_invalid_token_is_unauthorized() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let refresh_response = client
        .post(format!("{base_url}/api/v1/auth/refresh"))
        .header("content-type", "application/json")
        .body(json!({ "refreshToken": "invalid-refresh-token" }).to_string())
        .send()
        .await
        .unwrap();
    let refresh_status = refresh_response.status();
    let refresh_text = refresh_response.text().await.unwrap();
    assert_eq!(
        refresh_status,
        StatusCode::UNAUTHORIZED,
        "refresh failed: {refresh_text}"
    );
    let body: Value = serde_json::from_str(&refresh_text).unwrap();
    assert!(body["code"].is_string());

    Ok(())
}

#[tokio::test]
async fn auth_login_invalid_credentials_is_unauthorized() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let (_, email, _) = unique_identity();
    let login_response = login(&client, &base_url, &email, "wrong-password").await;
    let login_status = login_response.status();
    let login_text = login_response.text().await.unwrap();
    assert_eq!(
        login_status,
        StatusCode::UNAUTHORIZED,
        "login failed: {login_text}"
    );
    let body: Value = serde_json::from_str(&login_text).unwrap();
    assert_eq!(body["code"], "unauthorized");
    assert_ne!(body["message"], "");
    Ok(())
}
