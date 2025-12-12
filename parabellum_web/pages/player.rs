use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerVillageRow {
    pub village_id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub population: i32,
}

#[component]
pub fn PlayerProfilePage(username: String, villages: Vec<PlayerVillageRow>) -> Element {
    rsx! {
        div { class: "max-w-4xl mx-auto space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-semibold text-gray-800", "{username}" }
                div { class: "text-sm text-gray-600", "{villages.len()} villages" }
            }

            div { class: "overflow-hidden border rounded-md bg-white shadow-sm",
                table { class: "min-w-full text-sm",
                    thead { class: "bg-gray-100 text-left text-gray-600 uppercase text-xs tracking-wide",
                        tr {
                            th { class: "px-4 py-3", "Village" }
                            th { class: "px-4 py-3", "Coordinates" }
                            th { class: "px-4 py-3 text-right", "Population" }
                        }
                    }
                    if villages.is_empty() {
                        tbody {
                            tr {
                                td { class: "px-4 py-4 text-center text-gray-500 text-sm", colspan: "3",
                                    "No villages yet."
                                }
                            }
                        }
                    } else {
                        tbody { class: "divide-y divide-gray-200",
                            for village in villages {
                                tr { class: "hover:bg-gray-50",
                                    td { class: "px-4 py-3 font-semibold text-gray-800",
                                        a { class: "text-green-700 hover:underline", href: format!("/map/{}", village.village_id), "{village.name}" }
                                    }
                                    td { class: "px-4 py-3 text-gray-700", "({village.x}|{village.y})" }
                                    td { class: "px-4 py-3 text-right text-gray-900 font-semibold", "{village.population}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
