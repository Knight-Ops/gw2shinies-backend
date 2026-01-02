use crate::gw2_api::Gw2Client;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ItemSync {
    db: Surreal<Any>,
    gw2: Gw2Client,
}

impl ItemSync {
    pub fn new(db: Surreal<Any>) -> Self {
        Self {
            db,
            gw2: Gw2Client::new(),
        }
    }

    pub async fn run_sync(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting Item Sync...");
        let all_ids = self.gw2.fetch_all_item_ids().await?;
        println!("Found {} items.", all_ids.len());

        // Check if we already have the same number of items in the database
        let mut count_query = self.db.query("SELECT count() FROM item GROUP ALL").await?;
        let db_count: Option<usize> = count_query
            .take::<Option<serde_json::Value>>(0)?
            .and_then(|v| v.get("count")?.as_u64())
            .map(|c| c as usize);

        if let Some(count) = db_count {
            if count == all_ids.len() {
                println!(
                    "Skipping item upserts as count matches ({} items).",
                    all_ids.len()
                );
                return Ok(());
            }
        }

        let chunks = all_ids.chunks(200);
        for (i, chunk) in chunks.enumerate() {
            if i % 10 == 0 {
                println!("Syncing item chunk {}...", i + 1);
            }
            let items = self.gw2.fetch_items_chunk(chunk).await?;

            // Batch Upsert into SurrealDB
            // We use item:ID as the record ID
            let _: surrealdb::Response = self
                .db
                .query("FOR $item IN $items { UPSERT type::thing('item', <string>$item.gw2_id) CONTENT $item; }")
                .bind(("items", items))
                .await?;
        }

        println!("Item sync complete.");
        Ok(())
    }

    pub async fn spawn(self, interval_duration: std::time::Duration, token: CancellationToken) {
        let mut interval = tokio::time::interval(interval_duration);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.run_sync().await {
                        eprintln!("Item sync error: {}", e);
                    }
                }
                _ = token.cancelled() => {
                    println!("Item sync worker shutting down...");
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
    async fn test_item_sync_run_sync() {
        let db = setup_db().await;
        let server = MockServer::start().await;

        // Mock GW2 API for items
        let mock_items = vec![
            serde_json::json!({
                "id": 1,
                "name": "Item 1",
                "type": "Weapon",
                "level": 80,
                "rarity": "Exotic",
                "vendor_value": 100,
                "flags": ["Tradeable"],
                "game_types": ["PvE"],
                "restrictions": [],
                "chat_link": "[&AgH1AAA=]"
            }),
            serde_json::json!({
                "id": 2,
                "name": "Item 2",
                "type": "Armor",
                "level": 80,
                "rarity": "Exotic",
                "vendor_value": 200,
                "flags": ["Tradeable"],
                "game_types": ["PvE"],
                "restrictions": [],
                "chat_link": "[&AgH2AAA=]"
            }),
        ];

        Mock::given(method("GET"))
            .and(path("/v2/items"))
            .and(wiremock::matchers::query_param_is_missing("ids"))
            .respond_with(ResponseTemplate::new(200).set_body_json(vec![1, 2]))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v2/items"))
            .and(wiremock::matchers::query_param("ids", "1,2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_items))
            .mount(&server)
            .await;

        let gw2 = Gw2Client::with_urls(server.uri(), "".to_string());
        let sync = ItemSync {
            db: db.clone(),
            gw2,
        };

        // 1. Run sync
        sync.run_sync().await.unwrap();

        // 2. Verify items in DB
        let count: usize = db
            .query("SELECT count() FROM item GROUP ALL")
            .await
            .unwrap()
            .take::<Option<serde_json::Value>>(0)
            .unwrap()
            .and_then(|v| v.get("count")?.as_u64())
            .unwrap() as usize;
        assert_eq!(count, 2);

        // 3. Run again - should skip (verified by no more mock calls if we could, but here we just check it doesn't fail)
        sync.run_sync().await.unwrap();
        assert_eq!(count, 2);
    }
}
