use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MerchantStats {
    pub speed: u8,
    pub capacity: u32,
}

#[derive(Debug, Clone)]
pub struct Cost {
    pub resources: ResourceGroup,
    pub upkeep: u32,
    pub time: u32,
}

#[derive(Debug, Clone)]
pub struct ResearchCost {
    pub resources: ResourceGroup,
    pub time: u64,
}

#[derive(Debug, Default, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ResourceGroup(pub u32, pub u32, pub u32, pub u32);

impl ResourceGroup {
    pub const fn new(lumber: u32, clay: u32, iron: u32, crop: u32) -> Self {
        Self(lumber, clay, iron, crop)
    }

    pub fn total(&self) -> u32 {
        self.0 + self.1 + self.2 + self.3
    }

    pub fn lumber(&self) -> u32 {
        self.0
    }
    pub fn clay(&self) -> u32 {
        self.1
    }
    pub fn iron(&self) -> u32 {
        self.2
    }
    pub fn crop(&self) -> u32 {
        self.3
    }
}

impl core::ops::Mul<f64> for ResourceGroup {
    type Output = ResourceGroup;

    fn mul(self, rhs: f64) -> Self::Output {
        let wood = (self.0 as f64 * rhs).floor() as u32;
        let clay = (self.1 as f64 * rhs).floor() as u32;
        let iron = (self.2 as f64 * rhs).floor() as u32;
        let crop = (self.3 as f64 * rhs).floor() as u32;
        ResourceGroup(wood, clay, iron, crop)
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    password_hash: String,
}

impl User {
    pub fn new(id: Uuid, email: String, hashed_password: String) -> Self {
        Self {
            id,
            email,
            password_hash: hashed_password,
        }
    }

    pub fn password_hash(&self) -> &String {
        &self.password_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_group_total() {
        let rg = ResourceGroup::new(100, 200, 300, 400);
        assert_eq!(rg.total(), 1000);

        let rg_zero = ResourceGroup::new(0, 0, 0, 0);
        assert_eq!(rg_zero.total(), 0);
    }
}