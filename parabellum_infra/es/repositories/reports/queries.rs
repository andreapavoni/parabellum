//! Query builders for report projections.

use parabellum_app::villages::projection_repositories::ReportFilter;
use sqlx::{Postgres, QueryBuilder};

pub(super) fn report_list_query(filter: ReportFilter) -> QueryBuilder<'static, Postgres> {
    let offset = filter.offset;
    let limit = filter.limit;
    let mut query = report_select_query();

    push_report_filter(&mut query, filter);
    query.push(" ORDER BY r.created_at DESC, r.id DESC");

    if let Some(offset) = offset {
        query.push(" OFFSET ");
        query.push_bind(offset);
    }

    if let Some(limit) = limit {
        query.push(" LIMIT ");
        query.push_bind(limit);
    }

    query
}

pub(super) fn report_count_query(filter: ReportFilter) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(
        r#"
        SELECT COUNT(*)
        FROM rm_reports r
        JOIN rm_report_reads rr ON rr.report_id = r.id
        "#,
    );
    push_report_filter(&mut query, filter);
    query
}

fn report_select_query() -> QueryBuilder<'static, Postgres> {
    QueryBuilder::new(
        r#"
        SELECT
          r.id,
          r.report_type,
          r.payload,
          r.actor_player_id,
          r.actor_village_id,
          r.target_player_id,
          r.target_village_id,
          r.created_at,
          rr.read_at
        FROM rm_reports r
        JOIN rm_report_reads rr ON rr.report_id = r.id
        "#,
    )
}

fn push_report_filter(query: &mut QueryBuilder<'static, Postgres>, filter: ReportFilter) {
    query.push(" WHERE rr.player_id = ");
    query.push_bind(filter.player_id);

    if let Some(report_id) = filter.report_id {
        query.push(" AND r.id = ");
        query.push_bind(report_id);
    }

    if filter.unread_only {
        query.push(" AND rr.read_at IS NULL");
    }

    if !filter.kinds.is_empty() {
        query.push(" AND r.report_type IN (");
        let mut separated = query.separated(", ");
        for kind in filter.kinds {
            separated.push_bind(kind.as_str().to_string());
        }
        separated.push_unseparated(")");
    }
}
