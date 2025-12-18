use dioxus::prelude::*;
use parabellum_game::models::village::Village;

#[component]
pub fn MapPage(village: Village, world_size: i32) -> Element {
    rsx! {
        div {
            id: "map-page",
            class: "container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col md:flex-row justify-center items-center md:items-start gap-8 pb-12",
            "data-center-x": "{village.position.x}",
            "data-center-y": "{village.position.y}",
            "data-home-x": "{village.position.x}",
            "data-home-y": "{village.position.y}",
            "data-home-village-id": "{village.id}",
            "data-world-size": "{world_size}",

            div { class: "map-container-main relative w-full md:w-auto",
                div { class: "flex flex-col md:flex-row justify-between items-center w-full max-w-[560px] mb-4 px-2 md:pl-4",
                    h1 { class: "text-xl font-bold text-left w-full md:w-auto",
                        "Map "
                        span {
                            id: "header-coords",
                            class: "text-gray-700",
                            "({village.position.x}|{village.position.y})"
                        }
                    }
                }

                div { class: "map-layout",
                    // Y Axis (left side)
                    div {
                        id: "axis-y-container",
                        class: "axis-y"
                    }

                    // Map center (SVG grid) with navigation arrows
                    div {
                        class: "map-center",

                        // Navigation arrows (absolute positioned inside map-center)
                        div {
                            class: "nav-overlay nav-n",
                            "onclick": "moveMap(0, 1)",
                            title: "Nord (Y+)"
                        }
                        div {
                            class: "nav-overlay nav-s",
                            "onclick": "moveMap(0, -1)",
                            title: "Sud (Y-)"
                        }
                        div {
                            class: "nav-overlay nav-w",
                            "onclick": "moveMap(-1, 0)",
                            title: "Ovest (X-)"
                        }
                        div {
                            class: "nav-overlay nav-e",
                            "onclick": "moveMap(1, 0)",
                            title: "Est (X+)"
                        }

                        // SVG map
                        svg {
                            id: "map-svg",
                            class: "map-svg",
                            view_box: "0 0 1500 1500",
                            preserve_aspect_ratio: "none",

                            defs {
                                // Grid pattern
                                pattern {
                                    id: "gridPattern",
                                    width: "100",
                                    height: "100",
                                    pattern_units: "userSpaceOnUse",
                                    rect {
                                        width: "100",
                                        height: "100",
                                        fill: "none",
                                        stroke: "#9ACD32",
                                        stroke_width: "2",
                                        opacity: "0.3"
                                    }
                                }
                            }

                            // Background grid
                            rect {
                                width: "1500",
                                height: "1500",
                                fill: "url(#gridPattern)"
                            }

                            // Map tiles container (populated by JavaScript)
                            g { id: "map-tiles-container" }
                        }
                    }

                    // X Axis (bottom)
                    div {
                        id: "axis-x-container",
                        class: "axis-x"
                    }
                }

                div { class: "coords-input-container z-20",
                    span { class: "font-bold text-sm text-gray-700", "x" }
                    input {
                        r#type: "text",
                        id: "input-x",
                        value: "{village.position.x}",
                        class: "w-12 p-1.5 border border-gray-300 rounded text-center text-sm outline-none focus:border-green-500 font-semibold"
                    }
                    span { class: "font-bold text-sm text-gray-700", "y" }
                    input {
                        r#type: "text",
                        id: "input-y",
                        value: "{village.position.y}",
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
