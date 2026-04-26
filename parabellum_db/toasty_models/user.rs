use uuid::Uuid;

use parabellum_types::common::User;

#[derive(Debug, Clone, toasty::Model)]
#[table = "users"]
pub struct UserRecord {
    #[key]
    pub id: Uuid,

    #[index]
    pub email: String,

    pub password_hash: String,
    pub created_at: jiff::Timestamp,
}

impl From<UserRecord> for User {
    fn from(user: UserRecord) -> Self {
        User::new(user.id, user.email, user.password_hash)
    }
}
