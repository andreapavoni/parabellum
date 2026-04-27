use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::{VillageInfo, VillageRepository};
use parabellum_game::models::village::Village;
use parabellum_types::{
    errors::{ApplicationError, DbError},
    map::Position,
};

use crate::{
    mapping::{VillageAggregate, tribe_to_db_code},
    models as db_models,
    toasty_models::{
        army::{ArmyDbRow, into_db_army},
        hero::HeroDbRow,
        job::JobRecord,
        map_field::MapFieldDbRow,
        marketplace::MarketplaceOfferDbRow,
        player::PlayerRecord,
        village::VillageDbRow,
    },
};

pub struct ToastyVillageRepository {
    db: Arc<Mutex<toasty::Db>>,
}

impl ToastyVillageRepository {
    pub fn new(db: Arc<Mutex<toasty::Db>>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl VillageRepository for ToastyVillageRepository {
    async fn get_by_id(&self, village_id_u32: u32) -> Result<Village, ApplicationError> {
        let village_id = village_id_u32 as i32;
        let mut tx_guard = self.db.lock().await;

        let village = VillageDbRow::get_by_id(&mut *tx_guard, village_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(village_id_u32)))?;

        let player = PlayerRecord::get_by_id(&mut *tx_guard, village.player_id)
            .await
            .map_err(map_toasty_error)?;

        let armies = load_armies_for_village(&mut tx_guard, village_id).await?;
        let oases = MapFieldDbRow::filter_by_village_id(Some(village_id))
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?
            .into_iter()
            .map(db_models::MapField::from)
            .collect::<Vec<_>>();
        let busy_merchants = busy_merchants(&mut tx_guard, village_id).await?;

        let aggregate = VillageAggregate {
            village: village.try_into()?,
            player: to_db_player(player),
            armies,
            oases,
        };

        let mut game_village = Village::try_from(aggregate)?;
        game_village.busy_merchants = busy_merchants;
        Ok(game_village)
    }

    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Village>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;

        let player = PlayerRecord::get_by_id(&mut *tx_guard, player_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(player_id)))?;
        let db_player = to_db_player(player);

        let villages = VillageDbRow::filter_by_player_id(player_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        if villages.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(villages.len());
        for village in villages {
            let village_id = village.id;
            let armies = load_armies_for_village(&mut tx_guard, village_id).await?;
            let oases = MapFieldDbRow::filter_by_village_id(Some(village_id))
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?
                .into_iter()
                .map(db_models::MapField::from)
                .collect::<Vec<_>>();
            let merchants = busy_merchants(&mut tx_guard, village_id).await?;

            let aggregate = VillageAggregate {
                village: village.try_into()?,
                player: db_player.clone(),
                armies,
                oases,
            };

            let mut game_village = Village::try_from(aggregate)?;
            game_village.busy_merchants = merchants;
            result.push(game_village);
        }

        Ok(result)
    }

    async fn save(&self, village: &Village) -> Result<(), ApplicationError> {
        let record = VillageDbRow::try_from(village)?;
        let village_id = record.id;
        let mut tx_guard = self.db.lock().await;

        let mut rows = toasty::query!(VillageDbRow filter .id == #village_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        if let Some(mut existing) = rows.pop() {
            existing
                .update()
                .name(record.name)
                .buildings(record.buildings)
                .production(record.production)
                .stocks(record.stocks)
                .smithy_upgrades(record.smithy_upgrades)
                .academy_research(record.academy_research)
                .population(record.population)
                .loyalty(record.loyalty)
                .culture_points(record.culture_points)
                .culture_points_production(record.culture_points_production)
                .parent_village_id(record.parent_village_id)
                .updated_at(jiff::Timestamp::now())
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?;
        } else {
            toasty::create!(VillageDbRow {
                id: record.id,
                player_id: record.player_id,
                name: record.name,
                position: record.position,
                buildings: record.buildings,
                production: record.production,
                stocks: record.stocks,
                smithy_upgrades: record.smithy_upgrades,
                academy_research: record.academy_research,
                population: record.population,
                loyalty: record.loyalty,
                is_capital: record.is_capital,
                culture_points: record.culture_points,
                culture_points_production: record.culture_points_production,
                created_at: jiff::Timestamp::now(),
                updated_at: jiff::Timestamp::now(),
                parent_village_id: record.parent_village_id,
            })
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        }

        // Keep map ownership in sync with village position.
        let all_fields = MapFieldDbRow::all()
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        for mut field in all_fields {
            let Ok(position) = serde_json::from_value::<Position>(field.position.clone()) else {
                continue;
            };
            if position.x == village.position.x && position.y == village.position.y {
                field
                    .update()
                    .village_id(Some(village.id as i32))
                    .player_id(Some(village.player_id))
                    .exec(&mut *tx_guard)
                    .await
                    .map_err(map_toasty_error)?;
                break;
            }
        }

        Ok(())
    }

    async fn get_info_by_ids(&self, village_ids: &[u32]) -> Result<HashMap<u32, VillageInfo>, ApplicationError> {
        if village_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut tx_guard = self.db.lock().await;
        let mut result = HashMap::new();
        for village_id in village_ids {
            let Ok(village) = VillageDbRow::get_by_id(&mut *tx_guard, *village_id as i32).await else {
                continue;
            };
            let position: Position = serde_json::from_value(village.position).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid village position payload for {}: {}",
                    village.id, e
                )))
            })?;
            result.insert(
                *village_id,
                VillageInfo {
                    id: *village_id,
                    name: village.name,
                    position,
                },
            );
        }

        Ok(result)
    }
}

async fn load_armies_for_village(
    tx: &mut toasty::Db,
    village_id: i32,
) -> Result<Vec<db_models::Army>, ApplicationError> {
    let mut rows = ArmyDbRow::filter_by_village_id(village_id)
        .exec(tx)
        .await
        .map_err(map_toasty_error)?;

    let mut deployed = toasty::query!(ArmyDbRow filter .current_map_field_id == #(Some(village_id)))
        .exec(tx)
        .await
        .map_err(map_toasty_error)?;

    rows.append(&mut deployed);

    let mut dedup = HashMap::new();
    for row in rows {
        dedup.entry(row.id).or_insert(row);
    }

    let mut db_armies = Vec::with_capacity(dedup.len());
    for row in dedup.into_values() {
        let hero_row = match row.hero_id {
            Some(hero_id) => HeroDbRow::get_by_id(tx, hero_id).await.ok(),
            None => None,
        };
        db_armies.push(into_db_army(row, hero_row)?);
    }

    Ok(db_armies)
}

async fn busy_merchants(
    tx: &mut toasty::Db,
    village_id: i32,
) -> Result<u8, ApplicationError> {
    let jobs = JobRecord::filter_by_village_id(village_id)
        .exec(tx)
        .await
        .map_err(map_toasty_error)?;

    let jobs_total = jobs
        .iter()
        .filter(|job| job.status == "Pending" || job.status == "Processing")
        .filter(|job| {
            job.task.task_type == "MerchantGoing" || job.task.task_type == "MerchantReturn"
        })
        .filter_map(|job| job.task.data.get("merchants_used"))
        .filter_map(|value| value.as_u64())
        .sum::<u64>();

    let offers = MarketplaceOfferDbRow::filter_by_village_id(village_id)
        .exec(tx)
        .await
        .map_err(map_toasty_error)?;
    let offers_total = offers
        .iter()
        .map(|offer| i64::from(offer.merchants_required))
        .sum::<i64>()
        .max(0) as u64;

    let total = jobs_total.saturating_add(offers_total);
    Ok(total.min(u8::MAX as u64) as u8)
}

fn to_db_player(player: PlayerRecord) -> db_models::Player {
    let tribe = tribe_to_db_code(&player.tribe.into());
    db_models::Player {
        id: player.id,
        username: player.username,
        tribe,
        user_id: player.user_id,
        culture_points: player.culture_points,
    }
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}
