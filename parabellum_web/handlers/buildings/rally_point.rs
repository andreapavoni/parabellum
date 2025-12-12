use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use dioxus::prelude::*;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

use parabellum_app::{
    command_handlers::{
        AttackVillageCommandHandler, RecallTroopsCommandHandler, ReinforceVillageCommandHandler,
        ReleaseReinforcementsCommandHandler, ScoutVillageCommandHandler,
    },
    cqrs::{
        commands::{
            AttackVillage, RecallTroops, ReinforceVillage, ReleaseReinforcements, ScoutVillage,
        },
        queries::GetVillageById,
    },
    queries_handlers::GetVillageByIdHandler,
};
use parabellum_game::models::army::Army;
use parabellum_types::battle::{AttackType, ScoutingTarget};
use parabellum_types::{buildings::BuildingName, map::Position};

use crate::{
    components::{PageLayout, wrap_in_html},
    handlers::{
        building::{MAX_SLOT_ID, render_with_error},
        helpers::{CsrfForm, CurrentUser, HasCsrfToken, create_layout_data, generate_csrf},
    },
    http::AppState,
    pages::buildings::{ConfirmationType, SendTroopsConfirmationPage},
};

use rust_i18n::t;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SendMovementKind {
    Attack,
    Raid,
    Reinforcement,
}

#[derive(Debug)]
pub struct SendTroopsForm {
    pub slot_id: u8,
    pub target_x: i32,
    pub target_y: i32,
    pub movement: SendMovementKind,
    pub units: Vec<i32>,
    pub csrf_token: String,
}

impl<'de> Deserialize<'de> for SendTroopsForm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier)]
        enum Field {
            #[serde(rename = "slot_id")]
            SlotId,
            #[serde(rename = "target_x")]
            TargetX,
            #[serde(rename = "target_y")]
            TargetY,
            #[serde(rename = "movement")]
            Movement,
            #[serde(rename = "units[]")]
            Units,
            #[serde(rename = "csrf_token")]
            CsrfToken,
        }

        struct FormVisitor;

        impl<'de> Visitor<'de> for FormVisitor {
            type Value = SendTroopsForm;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("send troops form data")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut slot_id = None;
                let mut target_x = None;
                let mut target_y = None;
                let mut movement = None;
                let mut units = Vec::new();
                let mut csrf_token = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::SlotId => {
                            if slot_id.is_some() {
                                return Err(de::Error::duplicate_field("slot_id"));
                            }
                            slot_id = Some(map.next_value()?);
                        }
                        Field::TargetX => {
                            if target_x.is_some() {
                                return Err(de::Error::duplicate_field("target_x"));
                            }
                            target_x = Some(map.next_value()?);
                        }
                        Field::TargetY => {
                            if target_y.is_some() {
                                return Err(de::Error::duplicate_field("target_y"));
                            }
                            target_y = Some(map.next_value()?);
                        }
                        Field::Movement => {
                            if movement.is_some() {
                                return Err(de::Error::duplicate_field("movement"));
                            }
                            movement = Some(map.next_value()?);
                        }
                        Field::Units => {
                            let value: i32 = map.next_value()?;
                            units.push(value);
                        }
                        Field::CsrfToken => {
                            if csrf_token.is_some() {
                                return Err(de::Error::duplicate_field("csrf_token"));
                            }
                            csrf_token = Some(map.next_value()?);
                        }
                    }
                }

                let slot_id = slot_id.ok_or_else(|| de::Error::missing_field("slot_id"))?;
                let target_x = target_x.ok_or_else(|| de::Error::missing_field("target_x"))?;
                let target_y = target_y.ok_or_else(|| de::Error::missing_field("target_y"))?;
                let movement = movement.ok_or_else(|| de::Error::missing_field("movement"))?;
                let csrf_token =
                    csrf_token.ok_or_else(|| de::Error::missing_field("csrf_token"))?;

                Ok(SendTroopsForm {
                    slot_id,
                    target_x,
                    target_y,
                    movement,
                    units,
                    csrf_token,
                })
            }
        }

        deserializer.deserialize_map(FormVisitor)
    }
}

impl HasCsrfToken for SendTroopsForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// POST /army/send - Validate troops and render confirmation page
pub async fn send_troops(
    State(state): State<AppState>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<SendTroopsForm>,
) -> Response {
    if !(1..=MAX_SLOT_ID).contains(&form.slot_id) {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.rally_point.invalid_building").to_string(),
        )
        .await;
    }

    let slot_building = match user.village.get_building_by_slot_id(form.slot_id) {
        Some(slot) => slot,
        None => {
            return render_with_error(
                &state,
                jar,
                user,
                form.slot_id,
                t!("game.rally_point.invalid_building").to_string(),
            )
            .await;
        }
    };

    if slot_building.building.name != BuildingName::RallyPoint {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.rally_point.invalid_building").to_string(),
        )
        .await;
    }

    let troop_set = match parse_troop_set(&form.units) {
        Some(set) => set,
        None => {
            return render_with_error(
                &state,
                jar,
                user,
                form.slot_id,
                t!("game.rally_point.invalid_units").to_string(),
            )
            .await;
        }
    };

    if troop_set.iter().all(|amount| *amount == 0) {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.rally_point.invalid_units").to_string(),
        )
        .await;
    }

    let position = Position {
        x: form.target_x,
        y: form.target_y,
    };
    let target_village_id = position.to_id(state.world_size);

    // Validate target village exists
    if state
        .app_bus
        .query(
            GetVillageById {
                id: target_village_id,
            },
            GetVillageByIdHandler::new(),
        )
        .await
        .is_err()
    {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.rally_point.invalid_target").to_string(),
        )
        .await;
    }

    // Create a temporary army to analyze what confirmation type we need
    let temp_army = Army::new(
        None,
        user.village.id,
        Some(user.village.id),
        user.player.id,
        user.village.tribe.clone(),
        &troop_set,
        user.village.smithy(),
        None,
    );

    // Determine confirmation type based on army composition and movement type
    let confirmation_type = match form.movement {
        SendMovementKind::Reinforcement => {
            // Reinforcements always use simple confirmation
            ConfirmationType::Simple
        }
        SendMovementKind::Raid => {
            // Raids with only scouts get scouting choice
            if temp_army.is_only_scouts() {
                ConfirmationType::ScoutingChoice
            } else {
                ConfirmationType::Simple
            }
        }
        SendMovementKind::Attack => {
            // Normal attacks with only scouts get scouting choice
            if temp_army.is_only_scouts() {
                ConfirmationType::ScoutingChoice
            } else if temp_army.has_catapults() {
                // Normal attacks with catapults get catapult target selection
                // Get all buildings from target village (in real implementation, query this)
                // For now, provide common buildings as options
                ConfirmationType::CatapultTargets {
                    available_buildings: vec![
                        BuildingName::MainBuilding,
                        BuildingName::Warehouse,
                        BuildingName::Granary,
                        BuildingName::Barracks,
                        BuildingName::Stable,
                        BuildingName::Workshop,
                        BuildingName::Academy,
                        BuildingName::Smithy,
                        BuildingName::RallyPoint,
                        BuildingName::Marketplace,
                    ],
                }
            } else {
                ConfirmationType::Simple
            }
        }
    };

    let movement_type_str = match form.movement {
        SendMovementKind::Attack => t!("game.rally_point.movement.attack").to_string(),
        SendMovementKind::Raid => t!("game.rally_point.movement.raid").to_string(),
        SendMovementKind::Reinforcement => {
            t!("game.rally_point.movement.reinforcement").to_string()
        }
    };

    let movement_type_value = match form.movement {
        SendMovementKind::Attack => "attack",
        SendMovementKind::Raid => "raid",
        SendMovementKind::Reinforcement => "reinforcement",
    };

    // Generate new CSRF token for the confirmation form
    let (jar, csrf_token) = generate_csrf(jar);
    let layout_data = create_layout_data(&user, "village");

    // Render confirmation page
    let html = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            SendTroopsConfirmationPage {
                village_id: user.village.id,
                village_name: user.village.name.clone(),
                village_position: user.village.position.clone(),
                target_position: position,
                movement_type: movement_type_str,
                movement_type_value: movement_type_value.to_string(),
                tribe: user.village.tribe.clone(),
                troops: troop_set,
                confirmation_type: confirmation_type,
                csrf_token: csrf_token,
                slot_id: form.slot_id,
            }
        }
    });

    (jar, Html(wrap_in_html(&html))).into_response()
}

/// Form data for final confirmation
#[derive(Debug)]
pub struct ConfirmSendTroopsForm {
    pub village_id: u32,
    pub slot_id: u8,
    pub target_x: i32,
    pub target_y: i32,
    pub movement: SendMovementKind,
    pub units: Vec<i32>,
    pub scouting_target: Option<String>,
    pub catapult_target_1: Option<String>,
    pub catapult_target_2: Option<String>,
    pub csrf_token: String,
}

impl<'de> Deserialize<'de> for ConfirmSendTroopsForm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier)]
        enum Field {
            #[serde(rename = "village_id")]
            VillageId,
            #[serde(rename = "slot_id")]
            SlotId,
            #[serde(rename = "target_x")]
            TargetX,
            #[serde(rename = "target_y")]
            TargetY,
            #[serde(rename = "movement")]
            Movement,
            #[serde(rename = "units[]")]
            Units,
            #[serde(rename = "scouting_target")]
            ScoutingTarget,
            #[serde(rename = "catapult_target_1")]
            CatapultTarget1,
            #[serde(rename = "catapult_target_2")]
            CatapultTarget2,
            #[serde(rename = "csrf_token")]
            CsrfToken,
        }

        struct FormVisitor;

        impl<'de> Visitor<'de> for FormVisitor {
            type Value = ConfirmSendTroopsForm;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("confirm send troops form data")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut village_id = None;
                let mut slot_id = None;
                let mut target_x = None;
                let mut target_y = None;
                let mut movement = None;
                let mut units = Vec::new();
                let mut scouting_target = None;
                let mut catapult_target_1 = None;
                let mut catapult_target_2 = None;
                let mut csrf_token = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::VillageId => {
                            if village_id.is_some() {
                                return Err(de::Error::duplicate_field("village_id"));
                            }
                            village_id = Some(map.next_value()?);
                        }
                        Field::SlotId => {
                            if slot_id.is_some() {
                                return Err(de::Error::duplicate_field("slot_id"));
                            }
                            slot_id = Some(map.next_value()?);
                        }
                        Field::TargetX => {
                            if target_x.is_some() {
                                return Err(de::Error::duplicate_field("target_x"));
                            }
                            target_x = Some(map.next_value()?);
                        }
                        Field::TargetY => {
                            if target_y.is_some() {
                                return Err(de::Error::duplicate_field("target_y"));
                            }
                            target_y = Some(map.next_value()?);
                        }
                        Field::Movement => {
                            if movement.is_some() {
                                return Err(de::Error::duplicate_field("movement"));
                            }
                            movement = Some(map.next_value()?);
                        }
                        Field::Units => {
                            let value: i32 = map.next_value()?;
                            units.push(value);
                        }
                        Field::ScoutingTarget => {
                            scouting_target = Some(map.next_value()?);
                        }
                        Field::CatapultTarget1 => {
                            catapult_target_1 = Some(map.next_value()?);
                        }
                        Field::CatapultTarget2 => {
                            catapult_target_2 = Some(map.next_value()?);
                        }
                        Field::CsrfToken => {
                            if csrf_token.is_some() {
                                return Err(de::Error::duplicate_field("csrf_token"));
                            }
                            csrf_token = Some(map.next_value()?);
                        }
                    }
                }

                let village_id =
                    village_id.ok_or_else(|| de::Error::missing_field("village_id"))?;
                let slot_id = slot_id.ok_or_else(|| de::Error::missing_field("slot_id"))?;
                let target_x = target_x.ok_or_else(|| de::Error::missing_field("target_x"))?;
                let target_y = target_y.ok_or_else(|| de::Error::missing_field("target_y"))?;
                let movement = movement.ok_or_else(|| de::Error::missing_field("movement"))?;
                let csrf_token =
                    csrf_token.ok_or_else(|| de::Error::missing_field("csrf_token"))?;

                Ok(ConfirmSendTroopsForm {
                    village_id,
                    slot_id,
                    target_x,
                    target_y,
                    movement,
                    units,
                    scouting_target,
                    catapult_target_1,
                    catapult_target_2,
                    csrf_token,
                })
            }
        }

        deserializer.deserialize_map(FormVisitor)
    }
}

impl HasCsrfToken for ConfirmSendTroopsForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// POST /army/send/confirm - Execute the actual troop movement after confirmation
pub async fn confirm_send_troops(
    State(state): State<AppState>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<ConfirmSendTroopsForm>,
) -> Response {
    let home_army = match user.village.army() {
        Some(army) => army,
        None => {
            return render_with_error(
                &state,
                jar,
                user,
                form.slot_id,
                t!("game.rally_point.army_missing").to_string(),
            )
            .await;
        }
    };

    let troop_set = match parse_troop_set(&form.units) {
        Some(set) => set,
        None => {
            return render_with_error(
                &state,
                jar,
                user,
                form.slot_id,
                t!("game.rally_point.invalid_units").to_string(),
            )
            .await;
        }
    };

    let position = Position {
        x: form.target_x,
        y: form.target_y,
    };
    let target_village_id = position.to_id(state.world_size);

    let army_id = home_army.id;

    // Parse catapult targets if provided
    let catapult_targets =
        if let (Some(t1), Some(t2)) = (&form.catapult_target_1, &form.catapult_target_2) {
            [
                parse_building_name(t1).unwrap_or(BuildingName::MainBuilding),
                parse_building_name(t2).unwrap_or(BuildingName::Warehouse),
            ]
        } else {
            [BuildingName::MainBuilding, BuildingName::Warehouse]
        };

    // Check if this is a scouting mission based on the form
    // Only route to ScoutVillage if scouting_target is present AND non-empty
    let is_scouting = form
        .scouting_target
        .as_ref()
        .map_or(false, |s| !s.is_empty());

    let result = if is_scouting {
        // This is a scouting mission - use ScoutVillage command
        let scouting_target = if form.scouting_target.as_ref().unwrap() == "defenses" {
            ScoutingTarget::Defenses
        } else {
            ScoutingTarget::Resources
        };
        let attack_type = match form.movement {
            SendMovementKind::Attack => AttackType::Normal,
            SendMovementKind::Raid => AttackType::Raid,
            SendMovementKind::Reinforcement => {
                return render_with_error(
                    &state,
                    jar,
                    user,
                    form.slot_id,
                    t!("game.rally_point.scout_requires_attack").to_string(),
                )
                .await;
            }
        };

        state
            .app_bus
            .execute(
                ScoutVillage {
                    player_id: user.player.id,
                    village_id: user.village.id,
                    army_id,
                    units: troop_set,
                    target_village_id,
                    target: scouting_target,
                    attack_type,
                },
                ScoutVillageCommandHandler::new(),
            )
            .await
    } else {
        // Regular attack or reinforcement
        match form.movement {
            SendMovementKind::Attack => {
                state
                    .app_bus
                    .execute(
                        AttackVillage {
                            player_id: user.player.id,
                            village_id: user.village.id,
                            army_id,
                            units: troop_set,
                            target_village_id,
                            catapult_targets,
                            hero_id: None,
                            attack_type: AttackType::Normal,
                        },
                        AttackVillageCommandHandler::new(),
                    )
                    .await
            }
            SendMovementKind::Raid => {
                state
                    .app_bus
                    .execute(
                        AttackVillage {
                            player_id: user.player.id,
                            village_id: user.village.id,
                            army_id,
                            units: troop_set,
                            target_village_id,
                            catapult_targets,
                            hero_id: None,
                            attack_type: AttackType::Raid,
                        },
                        AttackVillageCommandHandler::new(),
                    )
                    .await
            }
            SendMovementKind::Reinforcement => {
                state
                    .app_bus
                    .execute(
                        ReinforceVillage {
                            player_id: user.player.id,
                            village_id: user.village.id,
                            army_id,
                            units: troop_set,
                            target_village_id,
                            hero_id: None,
                        },
                        ReinforceVillageCommandHandler::new(),
                    )
                    .await
            }
        }
    };

    match result {
        Ok(()) => Redirect::to(&format!("/build/{}", form.slot_id)).into_response(),
        Err(err) => render_with_error(&state, jar, user, form.slot_id, err.to_string()).await,
    }
}

fn parse_troop_set(values: &[i32]) -> Option<[u32; 10]> {
    let mut troops = [0u32; 10];
    for idx in 0..troops.len() {
        let amount = *values.get(idx).unwrap_or(&0);
        if amount < 0 {
            return None;
        }
        troops[idx] = amount as u32;
    }
    Some(troops)
}

fn parse_building_name(s: &str) -> Option<BuildingName> {
    match s {
        "Woodcutter" => Some(BuildingName::Woodcutter),
        "ClayPit" => Some(BuildingName::ClayPit),
        "IronMine" => Some(BuildingName::IronMine),
        "Cropland" => Some(BuildingName::Cropland),
        "Sawmill" => Some(BuildingName::Sawmill),
        "Brickyard" => Some(BuildingName::Brickyard),
        "IronFoundry" => Some(BuildingName::IronFoundry),
        "GrainMill" => Some(BuildingName::GrainMill),
        "Bakery" => Some(BuildingName::Bakery),
        "Warehouse" => Some(BuildingName::Warehouse),
        "Granary" => Some(BuildingName::Granary),
        "Smithy" => Some(BuildingName::Smithy),
        "TournamentSquare" => Some(BuildingName::TournamentSquare),
        "MainBuilding" => Some(BuildingName::MainBuilding),
        "RallyPoint" => Some(BuildingName::RallyPoint),
        "Marketplace" => Some(BuildingName::Marketplace),
        "Embassy" => Some(BuildingName::Embassy),
        "Barracks" => Some(BuildingName::Barracks),
        "Stable" => Some(BuildingName::Stable),
        "Workshop" => Some(BuildingName::Workshop),
        "Academy" => Some(BuildingName::Academy),
        "Cranny" => Some(BuildingName::Cranny),
        "TownHall" => Some(BuildingName::TownHall),
        "Residence" => Some(BuildingName::Residence),
        "Palace" => Some(BuildingName::Palace),
        "Treasury" => Some(BuildingName::Treasury),
        "TradeOffice" => Some(BuildingName::TradeOffice),
        "GreatBarracks" => Some(BuildingName::GreatBarracks),
        "GreatStable" => Some(BuildingName::GreatStable),
        _ => None,
    }
}

const RALLY_POINT_SLOT: u8 = 39;

#[derive(Debug, Deserialize)]
pub struct RecallForm {
    pub movement_id: String,
    pub csrf_token: String,
}

impl HasCsrfToken for RecallForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

#[derive(Debug, Deserialize)]
pub struct ReleaseForm {
    pub source_village_id: u32,
    pub csrf_token: String,
}

impl HasCsrfToken for ReleaseForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// POST /army/recall - Recall deployed troops back to home village
pub async fn recall_troops(
    State(state): State<AppState>,
    user: CurrentUser,
    CsrfForm { jar: _jar, form }: CsrfForm<RecallForm>,
) -> Response {
    // Parse movement_id as army UUID
    let army_id = match Uuid::parse_str(&form.movement_id) {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid movement ID format").into_response();
        }
    };

    let command = RecallTroops {
        player_id: user.player.id,
        village_id: user.village.id,
        army_id,
    };

    match state
        .app_bus
        .execute(command, RecallTroopsCommandHandler::new())
        .await
    {
        Ok(()) => Redirect::to(&format!("/build/{}", RALLY_POINT_SLOT)).into_response(),
        Err(e) => {
            tracing::error!("Failed to recall troops: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

/// POST /army/release - Release reinforcements back to their home village
pub async fn release_reinforcements(
    State(state): State<AppState>,
    user: CurrentUser,
    CsrfForm { jar: _jar, form }: CsrfForm<ReleaseForm>,
) -> Response {
    let command = ReleaseReinforcements {
        player_id: user.player.id,
        village_id: user.village.id,
        source_village_id: form.source_village_id,
    };

    match state
        .app_bus
        .execute(command, ReleaseReinforcementsCommandHandler::new())
        .await
    {
        Ok(()) => Redirect::to(&format!("/build/{}", RALLY_POINT_SLOT)).into_response(),
        Err(e) => {
            tracing::error!("Failed to release reinforcements: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}
