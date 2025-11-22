mod test_utils;

use axum::http::StatusCode;
use std::collections::HashMap;

use parabellum_core::ApplicationError;
use parabellum_types::tribe::Tribe;

use crate::test_utils::tests::{
    setup_http_client, setup_player_party, setup_user_cookie, setup_web_app,
};

#[tokio::test]
async fn test_register_player_happy_path() -> Result<(), ApplicationError> {
    let uow_provider = setup_web_app().await?;
    let uow = uow_provider.tx().await?;
    let client = setup_http_client(None, None).await;

    let email = "inttest@example.com";
    let username = "IntegrationUser";

    let mut form = HashMap::new();
    form.insert("username", username);
    form.insert("email", email);
    form.insert("password", "Secure123!");
    form.insert("tribe", "Teuton");
    form.insert("quadrant", "SouthWest");

    assert!(uow.users().get_by_email(&email.to_string()).await.is_err());

    let res = client
        .post("http://localhost:8088/register")
        .form(&form)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::SEE_OTHER);

    let email = email;
    let username = username.to_string();
    let user = uow.users().get_by_email(&email.to_string()).await?;
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

    let mut form = HashMap::new();
    form.insert("username", username);
    form.insert("tribe", "Teuton");
    form.insert("quadrant", "SouthWest");

    assert!(uow.users().get_by_email(&email.to_string()).await.is_err());

    let res = client
        .post("http://localhost:8088/register")
        .form(&form)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = res.text().await.unwrap();
    assert_eq!(
        body.to_string(),
        "Failed to deserialize form body: missing field `email`",
    );

    uow.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_register_authenticated_player() -> Result<(), ApplicationError> {
    let uow_provider = setup_web_app().await?;

    let (_, _, _, _, user) =
        setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

    let cookie = setup_user_cookie(user).await;
    let client = setup_http_client(Some(cookie), None).await;

    let res = client
        .get("http://localhost:8088/register")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);

    let location = res.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(location, "/");

    Ok(())
}
