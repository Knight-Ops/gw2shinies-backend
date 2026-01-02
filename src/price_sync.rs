use crate::gw2_api::Gw2Client;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct PriceSync {
    db: Surreal<Any>,
    gw2: Gw2Client,
}

impl PriceSync {
    pub fn new(db: Surreal<Any>) -> Self {
        Self {
            db,
            gw2: Gw2Client::new(),
        }
    }

    pub async fn run_sync(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting Price Sync...");
        let all_ids = self.gw2.fetch_all_price_ids().await?;
        println!("Found {} prices to sync.", all_ids.len());

        let chunks = all_ids.chunks(200);
        for (i, chunk) in chunks.enumerate() {
            if i % 10 == 0 {
                println!("Syncing price chunk {}...", i + 1);
            }
            let prices = self.gw2.fetch_prices_chunk(chunk).await?;

            for history in &prices {
                let item_id = history.item.clone();

                // 1. Update the item record with current price information for quick lookup
                let _: Option<serde::de::IgnoredAny> = self
                    .db
                    .update(&item_id)
                    .merge(serde_json::json!({
                        "buys": {
                            "quantity": history.buy_quantity,
                            "unit_price": history.buy_price,
                        },
                        "sells": {
                            "quantity": history.sell_quantity,
                            "unit_price": history.sell_price,
                        },
                        "last_price_update": history.timestamp,
                    }))
                    .await?;
            }

            // 2. Insert historical records for tracking trends (Batch)
            let _: Result<Vec<serde::de::IgnoredAny>, _> =
                self.db.insert("item_history").content(prices).await;
        }

        println!("Price sync complete.");
        Ok(())
    }

    pub async fn recover_history(
        &self,
        token: CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting historical data recovery check...");

        // 1. Get all items
        #[derive(serde::Deserialize)]
        struct ItemId {
            id: surrealdb::sql::Thing,
            gw2_id: u32,
        }
        let items: Vec<ItemId> = self
            .db
            .query("SELECT id, gw2_id FROM item WHERE gw2_id != NONE AND is_tradeable = true")
            .await?
            .take(0)?;
        println!("Checked {} items for history recovery.", items.len());

        // 2. Identify items that need history
        //    (For efficiency, we could do this via a complex query, but iterating is safer/easier for now to detect 'missing' history)
        //    Let's find all items that HAVE history first.
        #[derive(serde::Deserialize)]
        struct HistoryCount {
            item: surrealdb::sql::Thing,
            count: usize,
        }
        let history_counts: Vec<HistoryCount> = self
            .db
            .query("SELECT item, count() AS count FROM item_history GROUP BY item")
            .await?
            .take(0)?;

        let history_map: std::collections::HashMap<_, _> = history_counts
            .into_iter()
            .map(|h| (h.item.id.to_string(), h.count))
            .collect();

        let mut items_to_recover = Vec::new();
        for item in &items {
            let count = history_map.get(&item.id.id.to_string()).unwrap_or(&0);
            if *count < 5 {
                items_to_recover.push(item);
            }
        }

        println!(
            "Found {} items needing history recovery.",
            items_to_recover.len()
        );

        // 3. Recover history for these items
        for (i, item) in items_to_recover.iter().enumerate() {
            if i % 50 == 0 {
                println!("Recovering history: {}/{}", i + 1, items_to_recover.len());
            }

            match self.gw2.fetch_item_history(item.gw2_id).await {
                Ok(history) => {
                    if !history.is_empty() {
                        // Batch insert history records for efficiency
                        let _: Result<Vec<serde::de::IgnoredAny>, _> =
                            self.db.insert("item_history").content(history).await;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch history for item {}: {}", item.gw2_id, e);
                }
            }

            // Rate limiting for gw2bltc
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                _ = token.cancelled() => {
                    println!("Historical data recovery shutting down...");
                    return Ok(());
                }
            }
        }

        println!("Historical data recovery complete.");
        Ok(())
    }

    pub async fn spawn(self, interval_duration: Duration, token: CancellationToken) {
        let mut interval = interval(interval_duration);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.run_sync().await {
                        eprintln!("Price sync error: {}", e);
                    }
                }
                _ = token.cancelled() => {
                    println!("Price sync worker shutting down...");
                    break;
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gw2_api::Gw2Client;
    use surrealdb::engine::any::connect;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_db() -> Surreal<Any> {
        let db = connect("mem://").await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_price_sync_run_sync() {
        let db = setup_db().await;
        let server = MockServer::start().await;

        // Create an item in DB so update() works
        db.query("CREATE item:⟨1⟩ SET name = 'Test Item'")
            .await
            .unwrap();

        let mock_prices = vec![serde_json::json!({
            "id": 1,
            "buys": { "quantity": 100, "unit_price": 50 },
            "sells": { "quantity": 200, "unit_price": 60 }
        })];

        Mock::given(method("GET"))
            .and(path("/v2/commerce/prices"))
            .and(wiremock::matchers::query_param_is_missing("ids"))
            .respond_with(ResponseTemplate::new(200).set_body_json(vec![1]))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v2/commerce/prices"))
            .and(wiremock::matchers::query_param("ids", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_prices))
            .mount(&server)
            .await;

        let gw2 = Gw2Client::with_urls(server.uri(), "".to_string());
        let sync = PriceSync {
            db: db.clone(),
            gw2,
        };

        sync.run_sync().await.unwrap();

        // Verify item update
        #[derive(serde::Deserialize)]
        struct PriceCheck {
            buys: PriceDetail,
        }
        #[derive(serde::Deserialize)]
        struct PriceDetail {
            unit_price: u32,
        }
        let mut res = db.query("SELECT buys FROM item:⟨1⟩").await.unwrap();
        let item: PriceCheck = res.take::<Option<PriceCheck>>(0).unwrap().unwrap();
        assert_eq!(item.buys.unit_price, 50);

        // Verify history insertion
        let count: usize = db
            .query("SELECT count() FROM item_history GROUP ALL")
            .await
            .unwrap()
            .take::<Option<serde_json::Value>>(0)
            .unwrap()
            .and_then(|v| v.get("count")?.as_u64())
            .unwrap_or(0) as usize;
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_price_sync_recover_history() {
        let db = setup_db().await;
        let server = MockServer::start().await;

        // Create a tradeable item with NO history
        db.query("CREATE item:⟨1⟩ SET gw2_id = 1, is_tradeable = true, name = 'Tradeable Item'")
            .await
            .unwrap();

        let mock_history = vec![vec![1735689600, 60, 50, 200, 100]];

        Mock::given(method("GET"))
            .and(path("/api/tp/chart/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_history))
            .mount(&server)
            .await;

        let gw2 = Gw2Client::with_urls("".to_string(), server.uri());
        let sync = PriceSync {
            db: db.clone(),
            gw2,
        };
        let token = CancellationToken::new();

        sync.recover_history(token).await.unwrap();

        // Verify history recovery
        let count: usize = db
            .query("SELECT count() FROM item_history GROUP ALL")
            .await
            .unwrap()
            .take::<Option<serde_json::Value>>(0)
            .unwrap()
            .and_then(|v| v.get("count")?.as_u64())
            .unwrap() as usize;
        assert_eq!(count, 1);
    }
}
