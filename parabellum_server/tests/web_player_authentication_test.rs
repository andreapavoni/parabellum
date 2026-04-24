mod test_utils;

use axum::http::StatusCode;

use parabellum_types::tribe::Tribe;
use parabellum_types::{army::TroopSet, errors::ApplicationError};

use crate::test_utils::tests::{
    login_tokens, setup_http_client, setup_player_party, setup_web_app,
};

#[tokio::test]
async fn test_login_player_happy_path() -> Result<(), ApplicationError> {
    let (uow_provider, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, Some(1)).await;

    let (player, _, _, _, user) =
        setup_player_party(uow_provider, None, Tribe::Roman, TroopSet::default(), false).await?;

    let res = client
        .post(format!("{base_url}/api/v1/auth/token/login"))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "email": user.email,
                "password": "parabellum!",
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().await.unwrap().to_string();
    assert!(body.contains(&player.username));

    Ok(())
}

#[tokio::test]
async fn test_login_player_wrong_password() -> Result<(), ApplicationError> {
    let (uow_provider, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, Some(1)).await;

    let (_, _, _, _, user) =
        setup_player_party(uow_provider, None, Tribe::Roman, TroopSet::default(), false).await?;

    let res = client
        .post(format!("{base_url}/api/v1/auth/token/login"))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "email": user.email,
                "password": "wrong",
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let body = res.text().await.unwrap().to_string();
    assert!(body.contains("Invalid email or password."));

    Ok(())
}

#[tokio::test]
async fn test_logout_success() -> Result<(), ApplicationError> {
    let (uow_provider, base_url) = setup_web_app().await?;

    let (_, _, _, _, user) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;

    let client = setup_http_client(None, None).await;
    let tokens = login_tokens(&client, &base_url, &user.email, "parabellum!").await;

    let res = client
        .post(format!("{base_url}/api/v1/auth/token/logout"))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "refreshToken": tokens.refresh_token,
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_refresh_rotates_and_old_refresh_is_rejected() -> Result<(), ApplicationError> {
    let (uow_provider, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;
    let (_, _, _, _, user) =
        setup_player_party(uow_provider, None, Tribe::Roman, TroopSet::default(), false).await?;

    let tokens = login_tokens(&client, &base_url, &user.email, "parabellum!").await;

    let rotate = client
        .post(format!("{base_url}/api/v1/auth/refresh"))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "refreshToken": tokens.refresh_token,
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(rotate.status(), StatusCode::OK);

    let rotated_body: serde_json::Value =
        serde_json::from_str(&rotate.text().await.unwrap()).unwrap();
    let rotated_refresh = rotated_body["refreshToken"].as_str().unwrap();
    assert_ne!(rotated_refresh, tokens.refresh_token);

    let old_reuse = client
        .post(format!("{base_url}/api/v1/auth/refresh"))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "refreshToken": tokens.refresh_token,
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(old_reuse.status(), StatusCode::UNAUTHORIZED);
    let old_reuse_body: serde_json::Value =
        serde_json::from_str(&old_reuse.text().await.unwrap()).unwrap();
    assert_eq!(old_reuse_body["code"], "session_revoked");

    Ok(())
}
