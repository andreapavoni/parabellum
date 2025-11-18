use askama::Template;

#[derive(Debug, Template)]
#[template(path = "hello.html")]
pub struct HelloTemplate {
    pub current_user: bool,
    pub current_user_email: Option<String>,
}

// New template for the login page
#[derive(Debug, Default, Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub current_user: bool,
    pub current_user_email: Option<String>,
    pub email_value: String,   // to pre-fill email input
    pub error: Option<String>, // login error message, if any
}

// New template for the registration page
#[derive(Debug, Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {
    pub current_user: bool,
    pub current_user_email: Option<String>,
    pub username_value: String,    // to pre-fill username on error
    pub email_value: String,       // to pre-fill email on error
    pub selected_tribe: String,    // to retain selected tribe option
    pub selected_quadrant: String, // to retain selected quadrant option
    pub error: Option<String>,     // signup error message, if any
}
