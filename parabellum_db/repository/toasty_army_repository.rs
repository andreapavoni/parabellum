use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::ArmyRepository;
use parabellum_game::models::army::Army;
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::{
    toasty_models::{
        army::{ArmyDbRow, into_db_army},
        hero::HeroDbRow,
    },
};

pub struct ToastyArmyRepository<'a> {
    tx: Arc<Mutex<toasty::Transaction<'a>>>,
}

impl<'a> ToastyArmyRepository<'a> {
    pub fn new(tx: Arc<Mutex<toasty::Transaction<'a>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> ArmyRepository for ToastyArmyRepository<'a> {
    async fn get_by_id(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let row = ArmyDbRow::get_by_id(&mut *tx_guard, army_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::ArmyNotFound(army_id)))?;
        to_game_army(&mut tx_guard, row).await
    }

    async fn get_by_hero_id(&self, hero_id: Uuid) -> Result<Army, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut rows = toasty::query!(ArmyDbRow filter .hero_id == #(Some(hero_id)))
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        let row = rows
            .pop()
            .ok_or_else(|| ApplicationError::Db(DbError::HeroWithoutArmy(hero_id)))?;
        to_game_army(&mut tx_guard, row).await
    }

    async fn set_hero(&self, army_id: Uuid, hero_id: Option<Uuid>) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut row = ArmyDbRow::get_by_id(&mut *tx_guard, army_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::ArmyNotFound(army_id)))?;
        row.update()
            .hero_id(hero_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        Ok(())
    }

    async fn save(&self, army: &Army) -> Result<(), ApplicationError> {
        let record = ArmyDbRow::try_from(army)?;
        let army_id = record.id;
        let mut tx_guard = self.tx.lock().await;

        let mut rows = toasty::query!(ArmyDbRow filter .id == #army_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        if let Some(mut existing) = rows.pop() {
            existing
                .update()
                .village_id(record.village_id)
                .current_map_field_id(record.current_map_field_id)
                .hero_id(record.hero_id)
                .units(record.units)
                .smithy(record.smithy)
                .tribe(record.tribe)
                .player_id(record.player_id)
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?;
        } else {
            toasty::create!(ArmyDbRow {
                id: record.id,
                village_id: record.village_id,
                player_id: record.player_id,
                current_map_field_id: record.current_map_field_id,
                tribe: record.tribe,
                units: record.units,
                smithy: record.smithy,
                hero_id: record.hero_id,
            })
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        }

        Ok(())
    }

    async fn remove(&self, army_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        if let Ok(row) = ArmyDbRow::get_by_id(&mut *tx_guard, army_id).await {
            row.delete()
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?;
        }
        Ok(())
    }
}

async fn to_game_army(
    tx: &mut toasty::Transaction<'_>,
    row: ArmyDbRow,
) -> Result<Army, ApplicationError> {
    let hero_row = match row.hero_id {
        Some(hero_id) => HeroDbRow::get_by_id(tx, hero_id).await.ok(),
        None => None,
    };
    let db_army = into_db_army(row, hero_row)?;
    Ok(db_army.into())
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}
