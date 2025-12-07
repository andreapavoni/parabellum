use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageResources {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceProduction {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageCapacity {
    pub warehouse: u32,
    pub granary: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VillageHeaderData {
    pub resources: VillageResources,
    pub production: ResourceProduction,
    pub capacity: VillageCapacity,
    pub population: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutData {
    pub user: Option<UserInfo>,
    pub village: Option<VillageHeaderData>,
    pub server_time: i64,
    pub nav_active: String,
}

/// Main body layout component (to be wrapped in HTML shell)
#[component]
pub fn LayoutBody(data: LayoutData, children: Element) -> Element {
    rsx! {
        Header { data: data.clone() }
        main { class: "flex-grow container mx-auto",
            {children}
        }
        Footer {}
    }
}

#[component]
fn Header(data: LayoutData) -> Element {
    rsx! {
        header { class: "bg-white border-b border-gray-300 shadow-sm",
            if let Some(user) = &data.user {
                // Authenticated user header
                div { class: "flex justify-between items-center px-4 py-1 bg-gray-200 border-b border-gray-300 text-xs",
                    div { class: "font-serif font-bold text-lg text-gray-700 tracking-wide",
                        a { class: "cursor-pointer", href: "/", "PARABELLUM" }
                    }

                    div { class: "flex items-center gap-3 text-gray-600",
                        span { class: "font-bold text-gray-800", "{user.username}" }
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
fn NavBar(active: String) -> Element {
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
fn ResourceBar(village: VillageHeaderData) -> Element {
    rsx! {
        div { class: "flex justify-center items-center py-2 bg-white flex-wrap px-2",
            ResourceDisplay {
                icon: "🌲",
                amount: village.resources.lumber,
                capacity: village.capacity.warehouse,
                prod_per_hour: village.production.lumber,
                resource_type: "lumber"
            }
            ResourceDisplay {
                icon: "🧱",
                amount: village.resources.clay,
                capacity: village.capacity.warehouse,
                prod_per_hour: village.production.clay,
                resource_type: "clay"
            }
            ResourceDisplay {
                icon: "⛏️",
                amount: village.resources.iron,
                capacity: village.capacity.warehouse,
                prod_per_hour: village.production.iron,
                resource_type: "iron"
            }
            ResourceDisplay {
                icon: "🌾",
                amount: village.resources.crop,
                capacity: village.capacity.granary,
                prod_per_hour: village.production.crop,
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
fn ResourceDisplay(
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
fn Footer() -> Element {
    rsx! {
        footer { class: "bg-gray-100 border-t border-gray-300 py-4 mt-auto",
            div { class: "container mx-auto text-center text-sm text-gray-600",
                "PARABELLUM - A Travian 3.x inspired game"
            }
        }
    }
}

fn format_server_time(timestamp: i64) -> String {
    use chrono::prelude::*;
    let dt = DateTime::from_timestamp(timestamp, 0).unwrap_or_default();
    format!("{:02}:{:02}:{:02}", dt.hour(), dt.minute(), dt.second())
}

/// Generate the complete HTML document with embedded scripts
pub fn wrap_in_html_shell(body_content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PARABELLUM</title>
    <link rel="stylesheet" href="/assets/tailwind.css">
    <link rel="stylesheet" href="/assets/index.css">
</head>
<body class="flex flex-col min-h-screen">
{}
<script src="/assets/index.js" type="application/javascript"></script>
<script>
// Server clock ticker
window.__serverClock = window.__serverClock || (function() {{
    const pad = (num) => num.toString().padStart(2, '0');
    const element = document.getElementById('server-time');
    if (!element) return true;

    let timestamp = parseInt(element.dataset.timestamp || '0', 10);
    if (!Number.isFinite(timestamp) || timestamp <= 0) return true;

    const tick = () => {{
        timestamp += 1;
        const date = new Date(timestamp * 1000);
        const hours = pad(date.getUTCHours());
        const minutes = pad(date.getUTCMinutes());
        const seconds = pad(date.getUTCSeconds());
        element.textContent = `${{hours}}:${{minutes}}:${{seconds}}`;
        element.dataset.timestamp = timestamp;
    }};

    setInterval(tick, 1000);
    return true;
}})();

// Resource ticker
window.__resourceTicker = window.__resourceTicker || (function() {{
    const parseNumber = (value) => {{
        const parsed = parseFloat(value);
        return Number.isFinite(parsed) ? parsed : 0;
    }};

    const resources = Array.from(document.querySelectorAll('.res-value[data-prod-per-hour]')).map((el) => {{
        const amount = parseNumber(el.dataset.amount);
        const capacity = parseNumber(el.dataset.capacity);
        const prodPerHour = parseNumber(el.dataset.prodPerHour);
        return {{
            el,
            amount,
            capacity,
            capacityDisplay: Math.floor(capacity),
            perSecond: prodPerHour / 3600,
        }};
    }});

    if (!resources.length) return true;

    const render = (resource) => {{
        const amountInt = Math.max(0, Math.floor(resource.amount));
        resource.el.textContent = `${{amountInt}}/${{resource.capacityDisplay}}`;
    }};

    let lastTick = Date.now();
    const tick = () => {{
        const now = Date.now();
        const deltaSeconds = Math.max(0, (now - lastTick) / 1000);
        lastTick = now;

        resources.forEach((resource) => {{
            if (resource.perSecond === 0) return;
            resource.amount += resource.perSecond * deltaSeconds;
            resource.amount = Math.min(resource.capacity, Math.max(0, resource.amount));
            render(resource);
        }});
    }};

    resources.forEach(render);
    setInterval(tick, 1000);
    return true;
}})();
</script>
</body>
</html>"#,
        body_content
    )
}
