use crate::components::{
    BuildingQueue, BuildingQueueItem, ProductionPanel, ResourceFieldsMap, ResourceSlot, TroopsPanel,
};
use dioxus::prelude::*;
use parabellum_game::models::village::Village;

#[component]
pub fn ResourcesPage(
    village: Village,
    resource_slots: Vec<ResourceSlot>,
    building_queue: Vec<BuildingQueueItem>,
) -> Element {
    let production = &village.production.effective;

    rsx! {
        div { class: "container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col md:flex-row justify-center items-center md:items-start gap-8 pb-12",
            div { class: "flex flex-col items-center w-full md:w-auto",
                h1 { class: "text-xl font-bold mb-4 w-full text-left md:text-left",
                    "{village.name} ({village.position.x}|{village.position.y})"
                }

                // Resource fields map
                ResourceFieldsMap { slots: resource_slots.clone() }

                // Building queue
                BuildingQueue { queue: building_queue }
            }

            div { class: "w-full max-w-[360px] md:w-56 pt-4 md:pt-12 border-t md:border-t-0 border-gray-200 md:border-none",
                div { class: "flex flex-row md:flex-col justify-between md:justify-start gap-8 md:gap-0",

                    // Production info
                    ProductionPanel {
                        lumber: production.lumber,
                        clay: production.clay,
                        iron: production.iron,
                        crop: production.crop as u32
                    }

                    // Troops
                    TroopsPanel { village: village.clone() }
                }
            }
        }
    }
}
