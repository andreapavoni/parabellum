mod test_utils;

use axum::http::StatusCode;

use parabellum_types::tribe::Tribe;
use parabellum_types::{army::TroopSet, errors::ApplicationError};

use crate::test_utils::tests::{
    login_tokens, setup_http_client, setup_player_party, setup_web_app,
};

#[tokio::test]
async fn test_me_session_contract() -> Result<(), ApplicationError> {
    let (uow_provider, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;
    let (player, village, _, _, user) =
        setup_player_party(uow_provider, None, Tribe::Roman, TroopSet::default(), false).await?;

    let tokens = login_tokens(&client, &base_url, &user.email, "parabellum!").await;

    let response = client
        .get(format!("{base_url}/api/v1/me/session"))
        .bearer_auth(tokens.access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["authenticated"], true);
    assert_eq!(body["user"]["playerId"], player.id.to_string());
    assert_eq!(body["user"]["username"], player.username);
    assert_eq!(body["currentVillageId"], village.id);

    Ok(())
}

#[tokio::test]
async fn test_me_context_contract() -> Result<(), ApplicationError> {
    let (uow_provider, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;
    let (player, village, _, _, user) =
        setup_player_party(uow_provider, None, Tribe::Roman, TroopSet::default(), false).await?;

    let tokens = login_tokens(&client, &base_url, &user.email, "parabellum!").await;

    let response = client
        .get(format!("{base_url}/api/v1/me/context"))
        .bearer_auth(tokens.access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert!(body["serverTime"].as_i64().is_some());
    assert!(body["worldSize"].as_i64().is_some());
    assert!(body["serverSpeed"].as_i64().is_some());
    assert_eq!(body["player"]["id"], player.id.to_string());
    assert_eq!(body["currentVillage"]["id"], village.id);

    let villages = body["villages"].as_array().expect("villages must be array");
    assert!(!villages.is_empty());
    assert!(
        villages
            .iter()
            .any(|v| v["id"] == village.id && v["isCurrent"] == true)
    );

    Ok(())
}

#[tokio::test]
async fn test_village_overview_and_resources_owned_and_non_owned() -> Result<(), ApplicationError> {
    let (uow_provider, base_url) = setup_web_app().await?;
    let client = setup_http_client(None, None).await;

    let (_player, village, _, _, user) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;
    let (_other_player, other_village, _, _, _other_user) =
        setup_player_party(uow_provider, None, Tribe::Gaul, TroopSet::default(), false).await?;

    let tokens = login_tokens(&client, &base_url, &user.email, "parabellum!").await;

    let overview = client
        .get(format!(
            "{base_url}/api/v1/villages/{}/overview",
            village.id
        ))
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(overview.status(), StatusCode::OK);
    let overview_body: serde_json::Value =
        serde_json::from_str(&overview.text().await.unwrap()).unwrap();
    assert_eq!(overview_body["village"]["id"], village.id);
    assert!(overview_body["buildingSlots"].as_array().is_some());
    assert!(overview_body["buildingQueue"].as_array().is_some());

    let resources = client
        .get(format!(
            "{base_url}/api/v1/villages/{}/resources",
            village.id
        ))
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resources.status(), StatusCode::OK);
    let resources_body: serde_json::Value =
        serde_json::from_str(&resources.text().await.unwrap()).unwrap();
    assert_eq!(resources_body["village"]["id"], village.id);
    assert!(resources_body["resourceSlots"].as_array().is_some());
    assert!(resources_body["buildingQueue"].as_array().is_some());

    let forbidden_overview = client
        .get(format!(
            "{base_url}/api/v1/villages/{}/overview",
            other_village.id
        ))
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden_overview.status(), StatusCode::NOT_FOUND);
    let forbidden_overview_body: serde_json::Value =
        serde_json::from_str(&forbidden_overview.text().await.unwrap()).unwrap();
    assert_eq!(forbidden_overview_body["code"], "not_found");

    let forbidden_resources = client
        .get(format!(
            "{base_url}/api/v1/villages/{}/resources",
            other_village.id
        ))
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden_resources.status(), StatusCode::NOT_FOUND);
    let forbidden_resources_body: serde_json::Value =
        serde_json::from_str(&forbidden_resources.text().await.unwrap()).unwrap();
    assert_eq!(forbidden_resources_body["code"], "not_found");

    Ok(())
}
