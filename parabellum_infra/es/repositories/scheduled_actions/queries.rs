//! Scheduled-action query construction.

use parabellum_app::villages::projection_repositories::{
    ScheduledActionFilter, ScheduledActionOrder, ScheduledActionWorkflowFilter,
};
use sqlx::{Postgres, QueryBuilder};

use super::rows::{DbScheduledActionStatus, DbScheduledActionType};

pub(crate) fn scheduled_action_row_query(
    filter: ScheduledActionFilter,
) -> QueryBuilder<'static, Postgres> {
    scheduled_action_query(scheduled_action_select_sql(), filter)
}

pub(crate) fn scheduled_action_query(
    select_sql: &'static str,
    filter: ScheduledActionFilter,
) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(select_sql);
    apply_scheduled_action_filter(&mut query, &filter);
    push_order(&mut query, filter.order);
    if let Some(limit) = filter.limit {
        query.push(" LIMIT ");
        query.push_bind(limit);
    }
    query
}

pub(crate) fn scheduled_action_aggregate_query(
    select_sql: &'static str,
    filter: ScheduledActionFilter,
) -> QueryBuilder<'static, Postgres> {
    let mut query = QueryBuilder::new(select_sql);
    apply_scheduled_action_filter(&mut query, &filter);
    query
}

fn apply_scheduled_action_filter(
    query: &mut QueryBuilder<'static, Postgres>,
    filter: &ScheduledActionFilter,
) {
    let mut has_where = false;

    if let Some(action_types) = &filter.action_types {
        push_filter(query, &mut has_where);
        if action_types.is_empty() {
            query.push("FALSE");
        } else if action_types.len() == 1 {
            query.push("action_type = ");
            query.push_bind(DbScheduledActionType::from(action_types[0]));
        } else {
            query.push("action_type IN (");
            let mut separated = query.separated(", ");
            for action_type in action_types {
                separated.push_bind(DbScheduledActionType::from(*action_type));
            }
            separated.push_unseparated(")");
        }
    }

    if let Some(statuses) = &filter.statuses {
        push_filter(query, &mut has_where);
        if statuses.is_empty() {
            query.push("FALSE");
        } else {
            query.push("status IN (");
            let mut separated = query.separated(", ");
            for status in statuses {
                separated.push_bind(DbScheduledActionStatus::from(status.clone()));
            }
            separated.push_unseparated(")");
        }
    }

    for workflow_filter in &filter.workflow_filters {
        push_filter(query, &mut has_where);
        push_workflow_filter(query, *workflow_filter);
    }
}

fn scheduled_action_select_sql() -> &'static str {
    r#"
    SELECT id, action_type, execute_at, payload, status, created_at
    FROM rm_scheduled_actions
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

fn push_workflow_filter(
    query: &mut QueryBuilder<'static, Postgres>,
    filter: ScheduledActionWorkflowFilter,
) {
    match filter {
        ScheduledActionWorkflowFilter::Village(village_id) => {
            query.push("(payload->'workflow'->>'village_id')::int = ");
            query.push_bind(village_id as i32);
        }
        ScheduledActionWorkflowFilter::SourceVillage(village_id) => {
            query.push("(payload->'workflow'->>'source_village_id')::int = ");
            query.push_bind(village_id as i32);
        }
        ScheduledActionWorkflowFilter::TargetVillage(village_id) => {
            query.push("(payload->'workflow'->>'target_village_id')::int = ");
            query.push_bind(village_id as i32);
        }
        ScheduledActionWorkflowFilter::SourceOrVillage(village_id) => {
            query.push("((payload->'workflow'->>'source_village_id')::int = ");
            query.push_bind(village_id as i32);
            query.push(" OR (payload->'workflow'->>'village_id')::int = ");
            query.push_bind(village_id as i32);
            query.push(")");
        }
        ScheduledActionWorkflowFilter::Player(player_id) => {
            query.push("payload->'workflow'->>'player_id' = ");
            query.push_bind(player_id.to_string());
        }
        ScheduledActionWorkflowFilter::Movement(movement_id) => {
            query.push("payload->'workflow'->>'movement_id' = ");
            query.push_bind(movement_id.to_string());
        }
    }
}

fn push_order(query: &mut QueryBuilder<'static, Postgres>, order: ScheduledActionOrder) {
    match order {
        ScheduledActionOrder::ExecuteAtAsc => {
            query.push(" ORDER BY execute_at ASC, created_at ASC");
        }
        ScheduledActionOrder::CreatedAtAsc => {
            query.push(" ORDER BY created_at ASC");
        }
        ScheduledActionOrder::CreatedAtDesc => {
            query.push(" ORDER BY created_at DESC");
        }
    }
}
