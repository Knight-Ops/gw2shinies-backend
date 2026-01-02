use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryRecord {
    // This is the link! It points to "item:⟨19684⟩"
    pub item: RecordId,
    pub timestamp: DateTime<Utc>,
    pub buy_price: i64,
    pub sell_price: i64,
    pub buy_quantity: i64,
    pub sell_quantity: i64,
}

#[derive(Debug, Deserialize)]
pub struct RawPrice {
    pub id: u32,
    pub buys: RawPriceDetail,
    pub sells: RawPriceDetail,
}

#[derive(Debug, Deserialize)]
pub struct RawPriceDetail {
    pub quantity: i32,
    pub unit_price: i64,
}

impl HistoryRecord {
    pub fn from_raw(raw: RawPrice, timestamp: DateTime<Utc>) -> Self {
        Self {
            item: RecordId::from(("item", raw.id.to_string())),
            timestamp,
            buy_price: raw.buys.unit_price,
            sell_price: raw.sells.unit_price,
            buy_quantity: raw.buys.quantity as i64,
            sell_quantity: raw.sells.quantity as i64,
        }
    }

    pub fn from_bltc(id: u32, data: &[i64]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        // Index 0: timestamp (Unix Epoch Seconds)
        // Index 1: sell_price
        // Index 2: buy_price
        // Index 3: supply (sell_quantity)
        // Index 4: demand (buy_quantity)

        let timestamp = DateTime::from_timestamp(data[0], 0)?;

        Some(Self {
            item: RecordId::from(("item", id.to_string())),
            timestamp,
            sell_price: data[1],
            buy_price: data[2],
            sell_quantity: data[3],
            buy_quantity: data[4],
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_history_record_from_raw() {
        let raw = RawPrice {
            id: 19684,
            buys: RawPriceDetail {
                quantity: 100,
                unit_price: 50,
            },
            sells: RawPriceDetail {
                quantity: 200,
                unit_price: 60,
            },
        };
        let now = Utc::now();
        let record = HistoryRecord::from_raw(raw, now);

        assert_eq!(record.item.to_string(), "item:⟨19684⟩");
        assert_eq!(record.buy_price, 50);
        assert_eq!(record.sell_price, 60);
        assert_eq!(record.buy_quantity, 100);
        assert_eq!(record.sell_quantity, 200);
        assert_eq!(record.timestamp, now);
    }

    #[test]
    fn test_history_record_from_bltc() {
        let id = 19684;
        // Timestamp, sell_price, buy_price, sell_quantity, buy_quantity
        let data = vec![1735689600, 60, 50, 200, 100];
        let record = HistoryRecord::from_bltc(id, &data).unwrap();

        assert_eq!(record.item.to_string(), "item:⟨19684⟩");
        assert_eq!(record.timestamp, Utc.timestamp_opt(1735689600, 0).unwrap());
        assert_eq!(record.sell_price, 60);
        assert_eq!(record.buy_price, 50);
        assert_eq!(record.sell_quantity, 200);
        assert_eq!(record.buy_quantity, 100);
    }

    #[test]
    fn test_history_record_from_bltc_invalid() {
        let id = 19684;
        let data = vec![1735689600, 60, 50, 200]; // Too short
        let record = HistoryRecord::from_bltc(id, &data);
        assert!(record.is_none());
    }
}
