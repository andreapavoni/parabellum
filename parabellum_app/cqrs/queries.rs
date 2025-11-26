use parabellum_types::common::User;

use crate::cqrs::Query;

/// Checks if a user is authenticates with email and password.
pub struct AuthenticateUser {
    pub email: String,
    pub password: String,
}

impl Query for AuthenticateUser {
    type Output = User;
}

/// Fetch a user by email without checking password (for authenticated sessions).
pub struct GetUserByEmail {
    pub email: String,
}

impl Query for GetUserByEmail {
    type Output = User;
}
