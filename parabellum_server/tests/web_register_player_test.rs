mod test_utils;

use axum::http::StatusCode;
use std::collections::HashMap;

use parabellum_core::ApplicationError;
use parabellum_types::tribe::Tribe;

use crate::test_utils::tests::setup_web_app;

#[tokio::test]
async fn test_register_player_happy_path() -> Result<(), ApplicationError> {
    let (client, uow_provider) = setup_web_app().await?;
    let uow = uow_provider.begin().await?;

    let email = "inttest@example.com";
    let username = "IntegrationUser";

    let mut form = HashMap::new();
    form.insert("username", username);
    form.insert("email", email);
    form.insert("password", "Secure123!");
    form.insert("tribe", "Teuton");
    form.insert("quadrant", "SouthWest");

    assert!(uow.users().get_by_email(email.to_string()).await.is_err());

    let res = client
        .post("http://localhost:8088/register")
        .form(&form)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let email = email.to_string();
    let username = username.to_string();
    let user = uow.users().get_by_email(email.clone()).await?;
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
