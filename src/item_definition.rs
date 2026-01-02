use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemDefinition {
    pub gw2_id: i32,
    pub name: String,

    // AI Features (Categorical)
    pub type_: String,
    pub rarity: String,
    pub level: i32,
    pub vendor_value: i64,

    // Logic Filters (Booleans are faster than String Arrays)
    pub is_tradeable: bool, // Computed from 'flags' during ingest
}

#[derive(Debug, Deserialize)]
pub struct RawItem {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub r#type: String,
    pub level: u32,
    pub rarity: String,
    pub vendor_value: u32,
    pub default_skin: Option<u32>,
    pub game_types: Vec<String>,
    pub flags: Vec<String>,
    pub restrictions: Vec<String>,
    pub chat_link: String,
    pub icon: Option<String>,
    pub details: Option<Value>,
    pub upgrades_into: Option<Value>,
    pub upgrades_from: Option<Value>,
}

impl From<RawItem> for ItemDefinition {
    fn from(item: RawItem) -> Self {
        let is_tradeable = !item
            .flags
            .iter()
            .any(|f| f == "AccountBound" || f == "SoulbindOnAcquire" || f == "NoSell");

        Self {
            gw2_id: item.id as i32,
            name: item.name,
            type_: item.r#type,
            rarity: item.rarity,
            level: item.level as i32,
            vendor_value: item.vendor_value as i64,
            is_tradeable,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_item_to_item_definition_tradeable() {
        let raw = RawItem {
            id: 123,
            name: "Test Item".to_string(),
            description: None,
            r#type: "Weapon".to_string(),
            level: 80,
            rarity: "Exotic".to_string(),
            vendor_value: 100,
            default_skin: None,
            game_types: vec!["PvE".to_string()],
            flags: vec!["Many".to_string(), "Tradeable".to_string()],
            restrictions: vec![],
            chat_link: "[&AgH1AAA=]".to_string(),
            icon: None,
            details: None,
            upgrades_into: None,
            upgrades_from: None,
        };

        let def: ItemDefinition = raw.into();
        assert_eq!(def.gw2_id, 123);
        assert_eq!(def.name, "Test Item");
        assert_eq!(def.type_, "Weapon");
        assert_eq!(def.rarity, "Exotic");
        assert_eq!(def.level, 80);
        assert_eq!(def.vendor_value, 100);
        assert!(def.is_tradeable);
    }

    #[test]
    fn test_raw_item_to_item_definition_account_bound() {
        let raw = RawItem {
            id: 124,
            name: "Bound Item".to_string(),
            description: None,
            r#type: "Armor".to_string(),
            level: 80,
            rarity: "Ascended".to_string(),
            vendor_value: 0,
            default_skin: None,
            game_types: vec!["PvE".to_string()],
            flags: vec!["AccountBound".to_string()],
            restrictions: vec![],
            chat_link: "[&AgH2AAA=]".to_string(),
            icon: None,
            details: None,
            upgrades_into: None,
            upgrades_from: None,
        };

        let def: ItemDefinition = raw.into();
        assert!(!def.is_tradeable);
    }

    #[test]
    fn test_raw_item_to_item_definition_no_sell() {
        let raw = RawItem {
            id: 125,
            name: "No Sell Item".to_string(),
            description: None,
            r#type: "Trophy".to_string(),
            level: 0,
            rarity: "Basic".to_string(),
            vendor_value: 10,
            default_skin: None,
            game_types: vec!["PvE".to_string()],
            flags: vec!["NoSell".to_string()],
            restrictions: vec![],
            chat_link: "[&AgH3AAA=]".to_string(),
            icon: None,
            details: None,
            upgrades_into: None,
            upgrades_from: None,
        };

        let def: ItemDefinition = raw.into();
        assert!(!def.is_tradeable);
    }
}
