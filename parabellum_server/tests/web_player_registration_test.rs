mod test_utils;

use axum::http::StatusCode;

use parabellum_types::tribe::Tribe;
use parabellum_types::{army::TroopSet, errors::ApplicationError};

use crate::test_utils::tests::{
    login_tokens, setup_http_client, setup_player_party, setup_web_app,
};

#[tokio::test]
async fn test_register_player_happy_path() -> Result<(), ApplicationError> {
    let uow_provider = setup_web_app().await?;
    let uow = uow_provider.tx().await?;
    let client = setup_http_client(None, None).await;

    let email = "inttest@example.com";
    let username = "IntegrationUser";

    assert!(uow.users().get_by_email(email).await.is_err());

    let res = client
        .post("http://localhost:8088/api/v1/auth/token/register")
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "username": username,
                "email": email,
                "password": "Secure123!",
                "tribe": "Teuton",
                "quadrant": "SouthWest",
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let username = username.to_string();
    let user = uow.users().get_by_email(email).await?;
    assert_eq!(user.email, email, "User should have inserted email");

    let player = uow.players().get_by_user_id(user.id).await?;
    assert_eq!(
        player.tribe,
        Tribe::Teuton,
        "Player should have selected tribe"
    );
    assert_eq!(
        player.username, username,
        "Player should have inserted username"
    );

    let mut villages = uow.villages().list_by_player_id(player.id).await?;
    assert_eq!(villages.len(), 1, "Should create a village");

    let village = villages.pop().unwrap();
    let x = village.position.x;
    let y = village.position.y;

    assert!(x < 0 && y < 0, "Village should be on the selected quadrant");
    assert_eq!(
        village.population, 2,
        "Freshly created village should start with population 2"
    );

    uow.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_register_player_wrong_form() -> Result<(), ApplicationError> {
    let uow_provider = setup_web_app().await?;
    let client = setup_http_client(None, None).await;
    let uow = uow_provider.tx().await?;

    let email = "inttest@example.com";
    let username = "IntegrationUser";

    assert!(uow.users().get_by_email(email).await.is_err());

    let res = client
        .post("http://localhost:8088/api/v1/auth/token/register")
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "username": username,
                "tribe": "Teuton",
                "quadrant": "SouthWest",
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    uow.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_register_authenticated_player() -> Result<(), ApplicationError> {
    let uow_provider = setup_web_app().await?;

    let (_, _, _, _, user) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;

    let client = setup_http_client(None, None).await;
    let tokens = login_tokens(&client, &user.email, "parabellum!").await;

    let res = client
        .get("http://localhost:8088/api/v1/auth/token/session")
        .bearer_auth(tokens.access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    Ok(())
}
