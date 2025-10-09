use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    serialize::{self, Output, ToSql},
    sql_types::Jsonb,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::Write;

/// Generic wrapper for JSONB
#[derive(Debug, Clone, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
pub struct JsonbWrapper<T: std::fmt::Debug>(pub T);

impl<T> ToSql<Jsonb, Pg> for JsonbWrapper<T>
where
    T: Serialize + std::fmt::Debug,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        // serializza la struct in JSON (bytes)
        let bytes = serde_json::to_vec(&self.0)
            .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;

        out.write_all(&bytes)?;
        Ok(serialize::IsNull::No)
    }
}

impl<T> FromSql<Jsonb, Pg> for JsonbWrapper<T>
where
    T: DeserializeOwned + std::fmt::Debug,
{
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        let v = <serde_json::Value as FromSql<Jsonb, Pg>>::from_sql(bytes)?;
        let inner = serde_json::from_value(v)?;
        Ok(JsonbWrapper(inner))
    }
}

#[macro_export]
macro_rules! impl_jsonb_for {
    ($t:ty) => {
        impl From<$t> for JsonbWrapper<$t> {
            fn from(inner: $t) -> Self {
                JsonbWrapper(inner)
            }
        }
        impl From<JsonbWrapper<$t>> for $t {
            fn from(wrapper: JsonbWrapper<$t>) -> Self {
                wrapper.0
            }
        }
    };
}
