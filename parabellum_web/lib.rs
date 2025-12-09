pub mod pages;

pub mod components;
pub mod handlers;
pub mod view_helpers;

mod http;

pub use http::*;

#[macro_use]
extern crate rust_i18n;

i18n!("locales", fallback = "en");
