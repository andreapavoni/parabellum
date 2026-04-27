use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::HeroRepository;
use parabellum_game::models::hero::Hero;
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::{
    models as db_models,
    toasty_models::hero::HeroDbRow,
};

pub struct ToastyHeroRepository {
    db: Arc<Mutex<toasty::Db>>,
}

impl ToastyHeroRepository {
    pub fn new(db: Arc<Mutex<toasty::Db>>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl HeroRepository for ToastyHeroRepository {
    async fn save(&self, hero: &Hero) -> Result<(), ApplicationError> {
        let record = HeroDbRow::try_from(hero)?;
        let hero_id = record.id;
        let mut tx_guard = self.db.lock().await;

        let mut rows = toasty::query!(HeroDbRow filter .id == #hero_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        if let Some(mut existing) = rows.pop() {
            existing
                .update()
                .village_id(record.village_id)
                .level(record.level)
                .health(record.health)
                .experience(record.experience)
                .strength_points(record.strength_points)
                .regeneration_points(record.regeneration_points)
                .off_bonus_points(record.off_bonus_points)
                .def_bonus_points(record.def_bonus_points)
                .resources_points(record.resources_points)
                .unassigned_points(record.unassigned_points)
                .resource_focus(record.resource_focus)
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?;
        } else {
            toasty::create!(HeroDbRow {
                id: record.id,
                player_id: record.player_id,
                village_id: record.village_id,
                tribe: record.tribe,
                level: record.level,
                health: record.health,
                experience: record.experience,
                resource_focus: record.resource_focus,
                strength_points: record.strength_points,
                off_bonus_points: record.off_bonus_points,
                def_bonus_points: record.def_bonus_points,
                regeneration_points: record.regeneration_points,
                resources_points: record.resources_points,
                unassigned_points: record.unassigned_points,
            })
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        }

        Ok(())
    }

    async fn get_by_id(&self, hero_id: Uuid) -> Result<Hero, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let row = HeroDbRow::get_by_id(&mut *tx_guard, hero_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::HeroNotFound(hero_id)))?;
        let db_hero: db_models::Hero = row.into();
        Ok(db_hero.into())
    }
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}
