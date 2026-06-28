//! SQL query text for active merchant movement reads.

pub(super) fn active_movements_sql() -> &'static str {
    r#"
    SELECT *
    FROM (
        SELECT
            id,
            'outgoing' AS direction,
            'going' AS kind,
            (payload->'workflow'->>'source_village_id')::int AS origin_village_id,
            (payload->'workflow'->>'target_village_id')::int AS destination_village_id,
            payload->'workflow'->'resources' AS resources,
            (payload->'workflow'->>'merchants_used')::smallint AS merchants_used,
            (payload->'workflow'->>'arrives_at')::timestamptz AS arrives_at
        FROM rm_scheduled_actions
        WHERE action_type = 'MerchantsArrival'
          AND status IN ('pending', 'processing')
          AND (payload->'workflow'->>'village_id')::int = $1

        UNION ALL

        SELECT
            id,
            'incoming' AS direction,
            'going' AS kind,
            (payload->'workflow'->>'source_village_id')::int AS origin_village_id,
            (payload->'workflow'->>'target_village_id')::int AS destination_village_id,
            payload->'workflow'->'resources' AS resources,
            (payload->'workflow'->>'merchants_used')::smallint AS merchants_used,
            (payload->'workflow'->>'arrives_at')::timestamptz AS arrives_at
        FROM rm_scheduled_actions
        WHERE action_type = 'MerchantsArrival'
          AND status IN ('pending', 'processing')
          AND (payload->'workflow'->>'target_village_id')::int = $1

        UNION ALL

        SELECT
            return_action.id,
            'outgoing' AS direction,
            'return' AS kind,
            COALESCE(
                (return_action.payload->'workflow'->>'target_village_id')::int,
                (return_action.payload->'workflow'->>'village_id')::int
            ) AS origin_village_id,
            (return_action.payload->'workflow'->>'source_village_id')::int AS destination_village_id,
            '[0,0,0,0]'::jsonb AS resources,
            (return_action.payload->'workflow'->>'merchants_used')::smallint AS merchants_used,
            (return_action.payload->'workflow'->>'returns_at')::timestamptz AS arrives_at
        FROM rm_scheduled_actions return_action
        WHERE return_action.action_type = 'MerchantsReturn'
          AND return_action.status IN ('pending', 'processing')
          AND (return_action.payload->'workflow'->>'village_id')::int = $1
          AND NOT EXISTS (
              SELECT 1
              FROM rm_scheduled_actions going_action
              WHERE going_action.action_type = 'MerchantsArrival'
                AND going_action.status IN ('pending', 'processing')
                AND (going_action.payload->'workflow'->>'source_village_id')::int =
                    (return_action.payload->'workflow'->>'source_village_id')::int
                AND (going_action.payload->'workflow'->>'target_village_id')::int =
                    COALESCE(
                        (return_action.payload->'workflow'->>'target_village_id')::int,
                        (return_action.payload->'workflow'->>'village_id')::int
                    )
                AND (going_action.payload->'workflow'->>'merchants_used')::smallint =
                    (return_action.payload->'workflow'->>'merchants_used')::smallint
                AND (going_action.payload->'workflow'->>'arrives_at')::timestamptz <=
                    (return_action.payload->'workflow'->>'returns_at')::timestamptz
          )
    ) movements
    ORDER BY arrives_at ASC
    "#
}
