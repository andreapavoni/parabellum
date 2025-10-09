use super::schema::players;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Debug)]
pub enum TribeEnum {
    Roman,
    Gaul,
    Teuton,
    Natar,
    Nature,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = players)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: TribeEnum,
}

#[derive(Insertable)]
#[diesel(table_name = players)]
pub struct NewPlayer<'a> {
    pub id: Uuid,
    pub username: &'a str,
    pub tribe: TribeEnum,
}
