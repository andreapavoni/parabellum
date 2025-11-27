mod test_utils;

use axum::http::StatusCode;
use std::collections::HashMap;

use parabellum_types::errors::ApplicationError;
use parabellum_types::tribe::Tribe;

use crate::test_utils::tests::{
    fetch_csrf_token, setup_http_client, setup_player_party, setup_user_cookie, setup_web_app,
};

#[tokio::test]
async fn test_login_player_happy_path() -> Result<(), ApplicationError> {
    let uow_provider = setup_web_app().await?;
    let client = setup_http_client(None, Some(1)).await;

    let res = client
        .get("http://localhost:8088/login")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let (_player, _, _, _, user) =
        setup_player_party(uow_provider, None, Tribe::Roman, [0; 10], false).await?;
    let csrf_token = fetch_csrf_token(&client, "http://localhost:8088/login").await?;

    let mut form = HashMap::new();
    form.insert("email", user.email.as_str());
    form.insert("password", "parabellum!");
    form.insert("csrf_token", csrf_token.as_str());

    let res = client
        .post("http://localhost:8088/login")
        .form(&form)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().await.unwrap().to_string();
    // println!("==== body {} =======", body);
    assert!(body.contains("Username"));
    // assert!(body.contains(&format!("{}", player.username)));

    Ok(())
}

#[tokio::test]
async fn test_login_player_wrong_password() -> Result<(), ApplicationError> {
    let uow_provider = setup_web_app().await?;
    let client = setup_http_client(None, Some(1)).await;

    let (_, _, _, _, user) =
        setup_player_party(uow_provider, None, Tribe::Roman, [0; 10], false).await?;

    let csrf_token = fetch_csrf_token(&client, "http://localhost:8088/login").await?;

    let mut form = HashMap::new();
    form.insert("email", user.email.as_str());
    form.insert("password", "wrong");
    form.insert("csrf_token", csrf_token.as_str());

    let res = client
        .post("http://localhost:8088/login")
        .form(&form)
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
    let uow_provider = setup_web_app().await?;

    let (_, _, _, _, user) =
        setup_player_party(uow_provider.clone(), None, Tribe::Roman, [0; 10], false).await?;

    let cookie = setup_user_cookie(user).await;

    let client = setup_http_client(Some(cookie), None).await;
    let res = client
        .get("http://localhost:8088/logout")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);

    let location = res.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(location, "/");

    Ok(())
}
