use parabellum_types::buildings::BuildingName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingSlot {
    pub slot_id: u8,
    pub building_name: Option<BuildingName>,
    pub level: u8,
    pub in_queue: Option<bool>, // None = not in queue, Some(true) = processing, Some(false) = pending
}

impl BuildingSlot {
    /// Get CSS classes for rendering including queue state
    pub fn render_classes(&self, base_class: &str, include_occupied: bool) -> String {
        let mut classes = base_class.to_string();

        if include_occupied && self.building_name.is_some() {
            classes.push_str(" occupied");
        }

        if let Some(is_processing) = self.in_queue {
            if is_processing {
                classes.push_str(" construction-active");
            } else {
                classes.push_str(" construction-pending");
            }
        }

        classes
    }

    /// Get title/tooltip for the slot
    pub fn title(&self) -> String {
        if let Some(ref building) = self.building_name {
            if self.level > 0 {
                format!("{} (Level {})", building, self.level)
            } else {
                "Empty slot".to_string()
            }
        } else {
            "Empty slot".to_string()
        }
    }
}
