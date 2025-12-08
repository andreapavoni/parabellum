use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use parabellum_app::{
    command_handlers::{
        AttackVillageCommandHandler, ReinforceVillageCommandHandler, TrainUnitsCommandHandler,
    },
    cqrs::{
        commands::{AttackVillage, ReinforceVillage, TrainUnits},
        queries::GetVillageById,
    },
    queries_handlers::GetVillageByIdHandler,
};
use parabellum_game::battle::AttackType;
use parabellum_types::{buildings::BuildingName, map::Position};

use crate::{
    handlers::{CsrfForm, CurrentUser, HasCsrfToken},
    http::AppState,
};

use super::building_handler::{MAX_SLOT_ID, render_with_error};
use rust_i18n::t;

#[derive(Debug, Deserialize)]
pub struct TrainUnitsForm {
    pub slot_id: u8,
    pub unit_idx: u8,
    pub quantity: i32,
    pub building_name: BuildingName,
    pub csrf_token: String,
}

impl HasCsrfToken for TrainUnitsForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

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

pub async fn train_units(
    State(state): State<AppState>,
    user: CurrentUser,
    CsrfForm { jar, form }: CsrfForm<TrainUnitsForm>,
) -> Response {
    if form.quantity <= 0 {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.building.invalid_training_quantity").to_string(),
        )
        .await;
    }

    if !(1..=MAX_SLOT_ID).contains(&form.slot_id) {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.building.invalid_training_building").to_string(),
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
                t!("game.building.invalid_training_building").to_string(),
            )
            .await;
        }
    };

    if slot_building.building.name != form.building_name {
        return render_with_error(
            &state,
            jar,
            user,
            form.slot_id,
            t!("game.building.invalid_training_building").to_string(),
        )
        .await;
    }

    let result = state
        .app_bus
        .execute(
            TrainUnits {
                player_id: user.player.id,
                village_id: user.village.id,
                unit_idx: form.unit_idx,
                quantity: form.quantity,
                building_name: form.building_name.clone(),
            },
            TrainUnitsCommandHandler::new(),
        )
        .await;

    match result {
        Ok(()) => Redirect::to(&format!("/build/{}", form.slot_id)).into_response(),
        Err(err) => render_with_error(&state, jar, user, form.slot_id, err.to_string()).await,
    }
}

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

    let army_id = home_army.id;

    let result = match form.movement {
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
                        catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
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
                        catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
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
