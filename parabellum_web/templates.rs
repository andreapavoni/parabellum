use askama::Template;

#[derive(Template)]
#[template(path = "hello.html")]
pub struct HelloTemplate {}
