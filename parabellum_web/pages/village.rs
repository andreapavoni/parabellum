use dioxus::prelude::*;

use crate::components::{
    BuildingQueue, BuildingQueueItem, BuildingSlot, VillageListItem, VillageMap, VillagesList,
};

use parabellum_game::models::village::Village;

#[component]
pub fn VillagePage(
    village: Village,
    building_slots: Vec<BuildingSlot>,
    building_queue: Vec<BuildingQueueItem>,
    villages: Vec<VillageListItem>,
    csrf_token: String,
) -> Element {
    rsx! {
        div { class: "container mx-auto mt-4 md:mt-6 px-2 md:px-4 flex flex-col md:flex-row justify-center items-center md:items-start gap-8 pb-12",
            div { class: "flex flex-col items-center w-full md:w-auto",
                h1 { class: "text-xl font-bold mb-4 w-full text-left md:text-left",
                    "{village.name} ({village.position.x}|{village.position.y})"
                }

                VillageMap { slots: building_slots.clone() }

                BuildingQueue { queue: building_queue }
            }

            VillagesList { villages: villages, csrf_token: csrf_token }
        }
    }
}
