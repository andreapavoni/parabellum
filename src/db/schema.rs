// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "tribe"))]
    pub struct Tribe;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Tribe;

    armies (id) {
        id -> Uuid,
        village_id -> Int4,
        current_map_field_id -> Int4,
        hero_id -> Nullable<Uuid>,
        units -> Jsonb,
        smithy -> Jsonb,
        tribe -> Tribe,
        player_id -> Uuid,
    }
}

diesel::table! {
    heroes (id) {
        id -> Uuid,
        player_id -> Uuid,
        health -> Int2,
        experience -> Int4,
        attack_points -> Int4,
        defense_points -> Int4,
        off_bonus -> Int2,
        def_bonus -> Int2,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Tribe;

    players (id) {
        id -> Uuid,
        #[max_length = 255]
        username -> Varchar,
        tribe -> Tribe,
    }
}

diesel::table! {
    villages (id) {
        id -> Int4,
        player_id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        pos_x -> Int4,
        pos_y -> Int4,
        buildings -> Jsonb,
        production -> Jsonb,
        stocks -> Jsonb,
        smithy_upgrades -> Jsonb,
        population -> Int4,
        loyalty -> Int2,
        is_capital -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(armies -> heroes (hero_id));
diesel::joinable!(armies -> players (player_id));
diesel::joinable!(armies -> villages (village_id));
diesel::joinable!(heroes -> players (player_id));
diesel::joinable!(villages -> players (player_id));

diesel::allow_tables_to_appear_in_same_query!(armies, heroes, players, villages,);
