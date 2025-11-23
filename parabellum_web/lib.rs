pub mod handlers;
mod http;
mod templates;

pub use http::*;

#[macro_use]
extern crate rust_i18n;

i18n!("locales", fallback = "en");
