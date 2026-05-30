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
    username: &str,
    password: &str,
) -> reqwest::Response {
    let payload = json!({
        "username": username,
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

async fn login_access_token(
    client: &reqwest::Client,
    base_url: &str,
    username: &str,
    password: &str,
) -> String {
    let resp = login(client, base_url, username, password).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_str(&resp.text().await.unwrap()).unwrap();
    str_field(&body, &["access_token", "accessToken"])
        .unwrap()
        .to_string()
}

fn str_field<'a>(body: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| body.get(*key)?.as_str())
}

fn find_offer_id(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(id) = map
                .get("offerId")
                .or_else(|| map.get("offer_id"))
                .and_then(Value::as_str)
            {
                return Some(id.to_string());
            }
            map.values().find_map(find_offer_id)
        }
        Value::Array(items) => items.iter().find_map(find_offer_id),
        _ => None,
    }
}

async fn assert_error_code(response: reqwest::Response, status: StatusCode, code: &str) {
    let actual = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(actual, status, "unexpected status body: {text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["code"], code);
    assert!(body["message"].is_string());
}

async fn assert_unauthorized(response: reqwest::Response) {
    assert_error_code(response, StatusCode::UNAUTHORIZED, "unauthorized").await;
}

#[tokio::test]
async fn auth_login_happy_path_returns_tokens_and_me_context() -> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();

    let login_response = login(&client, &base_url, &seeded.username, &seeded.password).await;
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

    let login_response = login(&client, &base_url, &seeded.username, &seeded.password).await;
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
async fn auth_login_validation_error_for_missing_username() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let response = client
        .post(format!("{base_url}/api/v1/auth/token/login"))
        .header("content-type", "application/json")
        .body(
            json!({
                "username": "",
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
    assert_eq!(body["field_errors"]["username"], "Username is required");

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

    let (username, _, _) = unique_identity();
    let login_response = login(&client, &base_url, &username, "wrong-password").await;
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

#[tokio::test]
async fn openapi_contract_is_available() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base_url}/api/v1/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["openapi"], "3.1.0");
    assert!(body["paths"].is_object());
    Ok(())
}

#[tokio::test]
async fn protected_matrix_endpoints_require_bearer_token() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = reqwest::Client::new();

    let checks: Vec<(&str, String, Option<Value>)> = vec![
        ("GET", "/api/v1/me/session".to_string(), None),
        ("GET", "/api/v1/stats".to_string(), None),
        ("GET", "/api/v1/villages/1/resources".to_string(), None),
        ("GET", "/api/v1/buildings/1".to_string(), None),
        ("GET", "/api/v1/map/region".to_string(), None),
        ("GET", "/api/v1/map/fields/1".to_string(), None),
        ("GET", "/api/v1/reports".to_string(), None),
        ("GET", format!("/api/v1/reports/{}", Uuid::new_v4()), None),
        (
            "GET",
            "/api/v1/players/00000000-0000-0000-0000-000000000001".to_string(),
            None,
        ),
    ];

    for (method, path, body) in checks {
        let url = format!("{base_url}{path}");
        let req = match method {
            "GET" => client.get(url),
            "POST" => client.post(url).header("content-type", "application/json"),
            _ => unreachable!(),
        };
        let req = if let Some(payload) = body {
            req.body(payload.to_string())
        } else {
            req
        };
        assert_unauthorized(req.send().await.unwrap()).await;
    }

    Ok(())
}

#[tokio::test]
async fn contract_gap_register_invalid_password_should_be_422() -> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = reqwest::Client::new();
    let (username, email, _) = unique_identity();

    let response = client
        .post(format!("{base_url}/api/v1/auth/token/register"))
        .header("content-type", "application/json")
        .body(
            json!({
                "username": username,
                "email": email,
                "password": "abc",
                "tribe": "Teuton",
                "quadrant": "NorthEast"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_error_code(
        response,
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_error",
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn contract_gap_register_duplicate_email_should_be_409() -> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let (username, _, password) = unique_identity();

    let second = client
        .post(format!("{base_url}/api/v1/auth/token/register"))
        .header("content-type", "application/json")
        .body(
            json!({
                "username": username,
                "email": seeded.email,
                "password": password,
                "tribe": "Roman",
                "quadrant": "SouthWest"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_error_code(second, StatusCode::CONFLICT, "conflict").await;
    Ok(())
}

#[tokio::test]
async fn reports_detail_unknown_id_returns_not_found_with_valid_auth()
-> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let token = login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;

    let response = client
        .get(format!("{base_url}/api/v1/reports/{}", Uuid::new_v4()))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_error_code(response, StatusCode::NOT_FOUND, "not_found").await;
    Ok(())
}

#[tokio::test]
async fn reports_detail_malformed_id_returns_bad_request_before_auth()
-> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base_url}/api/v1/reports/not-a-uuid"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn map_field_unknown_id_returns_not_found_with_valid_auth() -> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let token = login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;

    let response = client
        .get(format!("{base_url}/api/v1/map/fields/999999999"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_error_code(response, StatusCode::NOT_FOUND, "not_found").await;
    Ok(())
}

#[tokio::test]
async fn map_region_partial_coordinates_returns_bad_request() -> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let token = login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;

    let response = client
        .get(format!("{base_url}/api/v1/map/region?x=10"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_error_code(response, StatusCode::BAD_REQUEST, "bad_request").await;
    Ok(())
}

#[tokio::test]
async fn army_preview_returns_canonical_arrives_at_timestamp() -> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let access_token =
        login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;

    let response = client
        .post(format!("{base_url}/api/v1/army/preview"))
        .bearer_auth(access_token)
        .header("content-type", "application/json")
        .body(
            json!({
                "targetX": 1,
                "targetY": 1,
                "movement": "attack",
                "units": [1,0,0,0,0,0,0,0,0,0]
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert!(body["arrivesAt"].as_str().is_some());
    assert!(body.get("travelTimeSecs").is_none());
    assert!(body.get("arrivesAtUnix").is_none());
    Ok(())
}

#[tokio::test]
async fn village_overview_unknown_id_returns_not_found_with_valid_auth()
-> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let token = login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;

    let response = client
        .get(format!("{base_url}/api/v1/villages/999999999/overview"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_error_code(response, StatusCode::NOT_FOUND, "not_found").await;
    Ok(())
}

#[tokio::test]
async fn village_resources_unknown_id_returns_not_found_with_valid_auth()
-> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let token = login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;

    let response = client
        .get(format!("{base_url}/api/v1/villages/999999999/resources"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_error_code(response, StatusCode::NOT_FOUND, "not_found").await;
    Ok(())
}

#[tokio::test]
async fn marketplace_accept_unknown_offer_returns_not_found_with_valid_auth()
-> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let token = login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;

    let response = client
        .post(format!(
            "{base_url}/api/v1/marketplace/offers/{}/accept",
            Uuid::new_v4()
        ))
        .bearer_auth(token)
        .header("content-type", "application/json")
        .body(json!({ "slotId": 28 }).to_string())
        .send()
        .await
        .unwrap();
    assert_error_code(response, StatusCode::NOT_FOUND, "not_found").await;
    Ok(())
}

#[tokio::test]
async fn marketplace_cancel_unknown_offer_returns_not_found_with_valid_auth()
-> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let token = login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;

    let response = client
        .post(format!(
            "{base_url}/api/v1/marketplace/offers/{}/cancel",
            Uuid::new_v4()
        ))
        .bearer_auth(token)
        .header("content-type", "application/json")
        .body(json!({ "slotId": 28 }).to_string())
        .send()
        .await
        .unwrap();
    assert_error_code(response, StatusCode::NOT_FOUND, "not_found").await;
    Ok(())
}

#[tokio::test]
async fn train_units_zero_quantity_returns_validation_error() -> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let token = login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;

    let response = client
        .post(format!("{base_url}/api/v1/army/train"))
        .bearer_auth(token)
        .header("content-type", "application/json")
        .body(
            json!({
                "slotId": 1,
                "unitIdx": 0,
                "quantity": 0,
                "buildingName": "Barracks"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_error_code(
        response,
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_error",
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn marketplace_offer_owner_accept_returns_conflict() -> Result<(), ApplicationError> {
    let (_schema, base_url, seeded) = setup_web_app_with_seeded_user().await?;
    let client = reqwest::Client::new();
    let owner_token =
        login_access_token(&client, &base_url, &seeded.username, &seeded.password).await;
    let owner_market_slot = 28;

    let create_offer = client
        .post(format!("{base_url}/api/v1/marketplace/offers"))
        .bearer_auth(&owner_token)
        .header("content-type", "application/json")
        .body(
            json!({
                "slotId": owner_market_slot,
                "offerLumber": 100,
                "offerClay": 0,
                "offerIron": 0,
                "offerCrop": 0,
                "seekLumber": 0,
                "seekClay": 0,
                "seekIron": 90,
                "seekCrop": 0
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    let create_offer_status = create_offer.status();
    let create_offer_text = create_offer.text().await.unwrap();
    assert_eq!(
        create_offer_status,
        StatusCode::OK,
        "create offer failed: {create_offer_text}"
    );

    let owner_market = client
        .get(format!("{base_url}/api/v1/buildings/{owner_market_slot}"))
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(owner_market.status(), StatusCode::OK);
    let owner_market_body: Value =
        serde_json::from_str(&owner_market.text().await.unwrap()).unwrap();
    let offer_id =
        find_offer_id(&owner_market_body).expect("offer id should exist after offer creation");

    let owner_accept = client
        .post(format!(
            "{base_url}/api/v1/marketplace/offers/{offer_id}/accept"
        ))
        .bearer_auth(owner_token)
        .header("content-type", "application/json")
        .body(json!({ "slotId": owner_market_slot }).to_string())
        .send()
        .await
        .unwrap();
    assert_error_code(owner_accept, StatusCode::CONFLICT, "conflict").await;
    Ok(())
}

#[tokio::test]
async fn extractor_first_precedence_returns_422_before_auth_for_invalid_body()
-> Result<(), ApplicationError> {
    let (_schema, base_url) = setup_web_app().await?;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/api/v1/buildings/upgrade"))
        .header("content-type", "application/json")
        .body(json!({ "notSlotId": 26 }).to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    Ok(())
}
