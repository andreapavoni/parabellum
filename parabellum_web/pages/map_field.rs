use dioxus::prelude::*;
use parabellum_game::models::village::Village;
use parabellum_types::map::{Position, ValleyTopology};
use rust_i18n::t;

/// Page showing information about an existing village on the map
#[component]
pub fn MapFieldVillagePage(
    village: Village,
    current_village_id: u32,
    csrf_token: String,
) -> Element {
    let is_own_village = village.id == current_village_id;

    rsx! {
        div { class: "container mx-auto px-4 py-6 max-w-4xl",
            h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                "{village.name}"
            }
            p { class: "text-gray-600 mb-6",
                "({village.position.x}|{village.position.y})"
            }

            div { class: "border rounded-md p-4 bg-white space-y-4",
                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "{t!(\"game.map.village_name\")}:" }
                    span { "{village.name}" }
                }

                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "{t!(\"game.map.coordinates\")}:" }
                    span { "({village.position.x}|{village.position.y})" }
                }

                div { class: "flex items-center gap-2 text-sm text-gray-700",
                    span { class: "font-semibold", "{t!(\"game.map.population\")}:" }
                    span { "{village.population}" }
                }

                if !is_own_village {
                    div { class: "mt-6 flex gap-3",
                        a {
                            href: "/build/39?target_x={village.position.x}&target_y={village.position.y}",
                            class: "bg-red-600 hover:bg-red-700 text-white font-semibold px-4 py-2 rounded",
                            "{t!(\"game.map.send_troops\")}"
                        }
                    }
                }
            }

            div { class: "mt-4",
                a {
                    href: "/map",
                    class: "text-blue-600 hover:text-blue-800 text-sm",
                    "← {t!(\"game.map.back_to_map\")}"
                }
            }
        }
    }
}

/// Page showing information about an empty valley (no village)
#[component]
pub fn MapFieldValleyPage(
    field_id: u32,
    position: Position,
    valley: ValleyTopology,
    can_found_village: bool,
    current_village_id: u32,
    current_village_name: String,
    csrf_token: String,
) -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-6 max-w-4xl",
            h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                "{t!(\"game.map.empty_valley\")}"
            }
            p { class: "text-gray-600 mb-6",
                "({position.x}|{position.y})"
            }

            div { class: "border rounded-md p-4 bg-white space-y-4",
                h2 { class: "text-lg font-semibold text-gray-800 mb-3",
                    "{t!(\"game.map.valley_resources\")}"
                }

                div { class: "grid grid-cols-2 sm:grid-cols-4 gap-4",
                    div { class: "text-center",
                        div { class: "text-2xl font-bold text-green-700", "{valley.0}" }
                        div { class: "text-xs text-gray-600", "{t!(\"game.resources.lumber\")}" }
                    }
                    div { class: "text-center",
                        div { class: "text-2xl font-bold text-orange-700", "{valley.1}" }
                        div { class: "text-xs text-gray-600", "{t!(\"game.resources.clay\")}" }
                    }
                    div { class: "text-center",
                        div { class: "text-2xl font-bold text-gray-700", "{valley.2}" }
                        div { class: "text-xs text-gray-600", "{t!(\"game.resources.iron\")}" }
                    }
                    div { class: "text-center",
                        div { class: "text-2xl font-bold text-yellow-600", "{valley.3}" }
                        div { class: "text-xs text-gray-600", "{t!(\"game.resources.crop\")}" }
                    }
                }

                if can_found_village {
                    div { class: "mt-6 p-4 bg-amber-50 border border-amber-200 rounded-md",
                        h3 { class: "text-sm font-semibold text-amber-900 mb-2",
                            "{t!(\"game.expansion.found_new_village\")}"
                        }
                        p { class: "text-sm text-amber-800 mb-3",
                            "{t!(\"game.expansion.found_village_description\")}"
                        }

                        form {
                            action: "/map/field/{field_id}/found/confirm",
                            method: "post",

                            input { r#type: "hidden", name: "field_id", value: "{field_id}" }
                            input { r#type: "hidden", name: "target_x", value: "{position.x}" }
                            input { r#type: "hidden", name: "target_y", value: "{position.y}" }
                            input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                            button {
                                r#type: "submit",
                                class: "bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-4 py-2 rounded",
                                "{t!(\"game.expansion.found_village\")}"
                            }
                        }
                    }
                } else {
                    div { class: "mt-6 p-4 bg-gray-50 border border-gray-200 rounded-md",
                        p { class: "text-sm text-gray-600",
                            "{t!(\"game.expansion.cannot_found_village\")}"
                        }
                        p { class: "text-xs text-gray-500 mt-1",
                            "{t!(\"game.expansion.requires_culture_points_and_settlers\")}"
                        }
                    }
                }
            }

            div { class: "mt-4",
                a {
                    href: "/map",
                    class: "text-blue-600 hover:text-blue-800 text-sm",
                    "← {t!(\"game.map.back_to_map\")}"
                }
            }
        }
    }
}

/// Page showing information about an oasis
#[component]
pub fn MapFieldOasisPage(field_id: u32, position: Position, csrf_token: String) -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-6 max-w-4xl",
            h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                "{t!(\"game.map.oasis\")}"
            }
            p { class: "text-gray-600 mb-6",
                "({position.x}|{position.y})"
            }

            div { class: "border rounded-md p-4 bg-white space-y-4",
                p { class: "text-sm text-gray-700",
                    "{t!(\"game.map.oasis_description\")}"
                }

                // TODO: Add oasis-specific information and actions
            }

            div { class: "mt-4",
                a {
                    href: "/map",
                    class: "text-blue-600 hover:text-blue-800 text-sm",
                    "← {t!(\"game.map.back_to_map\")}"
                }
            }
        }
    }
}
