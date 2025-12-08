use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapPageData {
    pub center_x: i32,
    pub center_y: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub home_village_id: u32,
    pub world_size: i32,
}

#[component]
pub fn MapPage(data: MapPageData) -> Element {
    rsx! {
        div {
            id: "map-page",
            class: "container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col md:flex-row justify-center items-center md:items-start gap-8 pb-12",
            "data-center-x": "{data.center_x}",
            "data-center-y": "{data.center_y}",
            "data-home-x": "{data.home_x}",
            "data-home-y": "{data.home_y}",
            "data-home-village-id": "{data.home_village_id}",
            "data-world-size": "{data.world_size}",

            div { class: "map-container-main relative w-full md:w-auto",
                div { class: "flex flex-col md:flex-row justify-between items-center w-full max-w-[560px] mb-4 px-2 md:pl-4",
                    h1 { class: "text-xl font-bold text-left w-full md:w-auto",
                        "Map "
                        span {
                            id: "header-coords",
                            class: "text-gray-700",
                            "({data.center_x}|{data.center_y})"
                        }
                    }
                }

                div { class: "large-map-wrapper",
                    div { class: "map-grid-container",
                        div {
                            class: "nav-arrow arrow-n",
                            "onclick": "moveMap(0, 1)",
                            title: "Nord (Y+)"
                        }
                        div {
                            class: "nav-arrow arrow-s",
                            "onclick": "moveMap(0, -1)",
                            title: "Sud (Y-)"
                        }
                        div {
                            class: "nav-arrow arrow-w",
                            "onclick": "moveMap(-1, 0)",
                            title: "Ovest (X-)"
                        }
                        div {
                            class: "nav-arrow arrow-e",
                            "onclick": "moveMap(1, 0)",
                            title: "Est (X+)"
                        }

                        // Y Axis
                        div { id: "y-axis-container", class: "y-axis" }

                        // 15x15 Grid (populated by JavaScript)
                        div { id: "map-grid", class: "map-grid" }

                        // X Axis
                        div { id: "x-axis-container", class: "x-axis" }
                    }
                }

                div { class: "coords-input-container z-20",
                    span { class: "font-bold text-sm text-gray-700", "x" }
                    input {
                        r#type: "text",
                        id: "input-x",
                        value: "{data.center_x}",
                        class: "w-12 p-1.5 border border-gray-300 rounded text-center text-sm outline-none focus:border-green-500 font-semibold"
                    }
                    span { class: "font-bold text-sm text-gray-700", "y" }
                    input {
                        r#type: "text",
                        id: "input-y",
                        value: "{data.center_y}",
                        class: "w-12 p-1.5 border border-gray-300 rounded text-center text-sm outline-none focus:border-green-500 font-semibold"
                    }
                    button {
                        "onclick": "goToCoords()",
                        class: "bg-gray-100 hover:bg-gray-200 border border-gray-300 px-4 py-1.5 rounded text-xs font-bold text-green-700 ml-3 cursor-pointer shadow-sm transition-colors",
                        "OK"
                    }
                }

                div {
                    id: "details-panel-container",
                    class: "details-panel hidden",
                    div { id: "details-panel", class: "text-xs text-gray-700" }
                }
            }
        }
    }
}
