use chrono::{DateTime, Utc};

use parabellum_types::errors::{ApplicationError, DbError};

pub fn chrono_to_jiff_utc(value: DateTime<Utc>) -> Result<jiff::Timestamp, ApplicationError> {
    jiff::Timestamp::from_second(value.timestamp())
        .and_then(|ts| {
            ts.checked_add(jiff::SignedDuration::new(
                0,
                value.timestamp_subsec_nanos() as i32,
            ))
        })
        .map_err(|err| {
            ApplicationError::Db(DbError::Transaction(format!(
                "could not convert chrono datetime to jiff timestamp: {err}"
            )))
        })
}

pub fn jiff_to_chrono_utc(value: jiff::Timestamp) -> Result<DateTime<Utc>, ApplicationError> {
    let nanos_i128 = value.as_nanosecond();
    let nanos_i64 = i64::try_from(nanos_i128).map_err(|_| {
        ApplicationError::Db(DbError::Transaction(
            "jiff timestamp is outside chrono nanosecond range".to_string(),
        ))
    })?;

    Ok(DateTime::<Utc>::from_timestamp_nanos(nanos_i64))
}
