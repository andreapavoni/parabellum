use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::common::Player;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutData {
    pub player: Option<Player>,
    pub village: Option<Village>,
    pub server_time: i64,
    pub nav_active: String,
}

/// Page body layout (will be wrapped in HTML shell)
#[component]
pub fn PageLayout(data: LayoutData, children: Element) -> Element {
    rsx! {
        Header { data: data.clone() }
        main { class: "flex-grow container mx-auto",
            {children}
        }
        Footer {}
    }
}

/// Wrap rendered body content in full HTML document shell
pub fn wrap_in_html(body_content: &str) -> String {
    static TEMPLATE: &str = include_str!("../templates/base.html");
    TEMPLATE.replace("{{BODY}}", body_content)
}

#[component]
pub fn Header(data: LayoutData) -> Element {
    rsx! {
        header { class: "bg-white border-b border-gray-300 shadow-sm",
            if let Some(player) = &data.player {
                // Authenticated user header
                div { class: "flex justify-between items-center px-4 py-1 bg-gray-200 border-b border-gray-300 text-xs",
                    div { class: "font-serif font-bold text-lg text-gray-700 tracking-wide",
                        a { class: "cursor-pointer", href: "/", "PARABELLUM" }
                    }

                    div { class: "flex items-center gap-3 text-gray-600",
                        span { class: "font-bold text-gray-800", "{player.username}" }
                        span { class: "cursor-pointer font-bold hover:text-green-600 text-green-700 hover:underline",
                            a { href: "/logout", "Logout" }
                        }
                        span {
                            id: "server-time",
                            class: "sm:inline text-[12px] text-gray-600 font-mono",
                            "data-timestamp": "{data.server_time}",
                            {format_server_time(data.server_time)}
                        }
                    }
                }

                // Navigation
                NavBar { active: data.nav_active.clone() }

                // Resource display
                if let Some(village) = &data.village {
                    ResourceBar { village: village.clone() }
                }
            } else {
                // Public header
                div { class: "container mx-auto flex justify-between items-center",
                    div { class: "font-serif font-bold text-2xl text-gray-700 tracking-wide",
                        a { href: "/", "PARABELLUM" }
                    }
                    div { class: "space-x-4 text-sm font-bold text-gray-600",
                        a { href: "/login", class: "hover:text-green-600 transition", "Login" }
                        a { href: "/register", class: "text-green-700 hover:underline", "Register" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn NavBar(active: String) -> Element {
    let nav_class = |page: &str| -> String {
        if active == page {
            "nav-icon nav-active".to_string()
        } else {
            "nav-icon".to_string()
        }
    };

    rsx! {
        div { class: "flex justify-center space-x-2 md:space-x-3 py-3 bg-gray-100 border-b border-gray-300 px-2 overflow-x-auto scrollbar-hide",
            div { class: "{nav_class(\"resources\")}", title: "Fields",
                a { href: "/resources", "🌾" }
            }
            div { class: "{nav_class(\"village\")}", title: "Village Center",
                a { href: "/village", "🏠" }
            }
            div { class: "{nav_class(\"map\")}", title: "Map",
                a { href: "/map", "🗺️" }
            }
            div { class: "nav-icon", title: "Stats", "📊" }
            div { class: "{nav_class(\"reports\")}", title: "Reports",
                a { href: "/reports", "📜" }
            }
            div { class: "nav-icon", title: "Messages", "✉️" }
        }
    }
}

#[component]
pub fn ResourceBar(village: Village) -> Element {
    rsx! {
        div { class: "flex justify-center items-center py-2 bg-white flex-wrap px-2",
            ResourceDisplay {
                icon: "🌲",
                amount: village.stored_resources().lumber(),
                capacity: village.warehouse_capacity(),
                prod_per_hour: village.production.effective.lumber,
                resource_type: "lumber"
            }
            ResourceDisplay {
                icon: "🧱",
                amount: village.stored_resources().clay(),
                capacity: village.warehouse_capacity(),
                prod_per_hour: village.production.effective.clay,
                resource_type: "clay"
            }
            ResourceDisplay {
                icon: "⛏️",
                amount: village.stored_resources().iron(),
                capacity: village.warehouse_capacity(),
                prod_per_hour: village.production.effective.iron,
                resource_type: "iron"
            }
            ResourceDisplay {
                icon: "🌾",
                amount: village.stored_resources().crop(),
                capacity: village.granary_capacity(),
                prod_per_hour: village.production.effective.crop as u32,
                resource_type: "crop"
            }
            div { class: "res-item",
                span { class: "mr-1", "👤" }
                span { "{village.population}" }
            }
        }
    }
}

#[component]
pub fn ResourceDisplay(
    icon: String,
    amount: u32,
    capacity: u32,
    prod_per_hour: u32,
    resource_type: String,
) -> Element {
    rsx! {
        div { class: "res-item",
            span { class: "mr-1", "{icon}" }
            span {
                class: "res-value",
                "data-resource": "{resource_type}",
                "data-amount": "{amount}",
                "data-capacity": "{capacity}",
                "data-prod-per-hour": "{prod_per_hour}",
                "{amount}/{capacity}"
            }
        }
    }
}

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer { class: "bg-white border-t border-gray-300 py-4 text-center text-xs text-gray-400",
                p {
                    "A "
                    a { class: "hover:underline", href: "https://pavonz.com", "pavonz" }
                    " joint | © 2025 | "
                    a {
                        class: "hover:underline",
                        href: "https://github.com/andreapavoni/parabellum",
                        "Github"
                    }
                }
                div { class: "mt-2 space-x-3",
                    span { "{t!(\"footer.not_affiliated\", name = \"Travian Games GmbH\")}" }
                }
            }
    }
}

fn format_server_time(timestamp: i64) -> String {
    use chrono::prelude::*;
    let dt = DateTime::from_timestamp(timestamp, 0).unwrap_or_default();
    format!("{:02}:{:02}:{:02}", dt.hour(), dt.minute(), dt.second())
}
