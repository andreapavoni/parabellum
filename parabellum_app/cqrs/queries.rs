use parabellum_types::common::User;

use crate::cqrs::Query;

pub struct AuthenticateUser {
    pub email: String,
    pub password: String,
}

impl Query for AuthenticateUser {
    type Output = User;
}
