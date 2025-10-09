// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "tribe_enum"))]
    pub struct TribeEnum;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TribeEnum;

    players (id) {
        id -> Uuid,
        #[max_length = 255]
        username -> Varchar,
        tribe -> TribeEnum,
    }
}

diesel::table! {
    villages (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        player_id -> Uuid,
        loyalty -> Int2,
        is_capital -> Bool,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(villages -> players (player_id));

diesel::allow_tables_to_appear_in_same_query!(players, villages,);
