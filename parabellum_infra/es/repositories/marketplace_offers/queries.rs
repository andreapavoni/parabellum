//! Query builders for marketplace offer projections.

use parabellum_app::villages::projection_repositories::MarketplaceOfferListFilter;
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use super::rows::DbMarketplaceOfferStatus;

pub(super) fn marketplace_offer_query(
    filter: MarketplaceOfferListFilter,
) -> QueryBuilder<'static, Postgres> {
    let mut query = marketplace_offer_select_query();
    let mut has_where = false;

    if let Some(owner_village_id) = filter.owner_village_id {
        push_filter(&mut query, &mut has_where);
        query.push("owner_village_id = ");
        query.push_bind(owner_village_id as i32);
    }

    if let Some(exclude_owner_village_id) = filter.exclude_owner_village_id {
        push_filter(&mut query, &mut has_where);
        query.push("owner_village_id <> ");
        query.push_bind(exclude_owner_village_id as i32);
    }

    if let Some(status) = filter.status {
        push_filter(&mut query, &mut has_where);
        query.push("status = ");
        query.push_bind(DbMarketplaceOfferStatus::from(status));
    }

    query.push(" ORDER BY created_at DESC");
    query
}

pub(super) fn marketplace_offer_by_id_query(offer_id: Uuid) -> QueryBuilder<'static, Postgres> {
    let mut query = marketplace_offer_select_query();
    query.push(" WHERE offer_id = ");
    query.push_bind(offer_id);
    query
}

pub(super) fn push_marketplace_offer_returning(query: &mut QueryBuilder<'static, Postgres>) {
    query.push(" RETURNING ");
    query.push(marketplace_offer_columns_sql());
}

fn marketplace_offer_select_query() -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new("SELECT ");
    query.push(marketplace_offer_columns_sql());
    query.push(" FROM rm_marketplace_offers");
    query
}

fn marketplace_offer_columns_sql() -> &'static str {
    r#"
    offer_id, owner_player_id, owner_village_id, offer_resources, seek_resources,
    merchants_reserved, status, accepted_by_player_id, accepted_by_village_id,
    created_at, accepted_at, canceled_at
    "#
}

fn push_filter(query: &mut QueryBuilder<'static, Postgres>, has_where: &mut bool) {
    if *has_where {
        query.push(" AND ");
    } else {
        query.push(" WHERE ");
        *has_where = true;
    }
}
