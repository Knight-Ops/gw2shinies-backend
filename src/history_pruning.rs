use std::time::Duration;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct HistoryPruning {
    db: Surreal<Any>,
}

impl HistoryPruning {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }

    pub async fn run_pruning(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting history pruning...");

        // Strategy: Instead of strict minute-based rules (which fail with sync jitters
        // or external imports), we keep the EARLIEST record in each time bucket.
        // This ensures at least one data point per period even if it's "late".

        // 1. Older than 3 days, keep 1 per hour
        let q1 = "DELETE item_history WHERE 
            <datetime>timestamp < (time::now() - 3d) AND 
            <datetime>timestamp >= (time::now() - 7d) AND 
            count(SELECT id FROM item_history WHERE item = $parent.item AND time::floor(<datetime>timestamp, 1h) = time::floor(<datetime>$parent.timestamp, 1h) AND <datetime>timestamp < <datetime>$parent.timestamp LIMIT 1) > 0";

        // 2. Older than 1 week, keep 1 per 3 hours
        let q2 = "DELETE item_history WHERE 
            <datetime>timestamp < (time::now() - 7d) AND 
            <datetime>timestamp >= (time::now() - 14d) AND 
            count(SELECT id FROM item_history WHERE item = $parent.item AND time::floor(<datetime>timestamp, 3h) = time::floor(<datetime>$parent.timestamp, 3h) AND <datetime>timestamp < <datetime>$parent.timestamp LIMIT 1) > 0";

        // 3. Older than 2 weeks, keep 1 per 6 hours
        let q3 = "DELETE item_history WHERE 
            <datetime>timestamp < (time::now() - 14d) AND 
            count(SELECT id FROM item_history WHERE item = $parent.item AND time::floor(<datetime>timestamp, 6h) = time::floor(<datetime>$parent.timestamp, 6h) AND <datetime>timestamp < <datetime>$parent.timestamp LIMIT 1) > 0";

        // Execute queries
        self.db.query(q1).await?.check()?;
        self.db.query(q2).await?.check()?;
        self.db.query(q3).await?.check()?;

        println!("History pruning complete.");
        Ok(())
    }

    pub async fn spawn(self, interval_duration: Duration, token: CancellationToken) {
        let mut interval = interval(interval_duration);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.run_pruning().await {
                        eprintln!("History pruning error: {}", e);
                    }
                }
                _ = token.cancelled() => {
                    println!("History pruning worker shutting down...");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_record::HistoryRecord;
    use chrono::{Duration as ChronoDuration, Utc};
    use surrealdb::engine::any::connect;

    async fn setup_db() -> Surreal<Any> {
        let db = connect("mem://").await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();
        db.query(
            "DEFINE TABLE item SCHEMALESS; 
                 DEFINE TABLE item_history SCHEMALESS;
                 DEFINE INDEX item_history_item_ts_idx ON TABLE item_history COLUMNS item, timestamp;",
        )
        .await
        .unwrap();
        db
    }

    #[tokio::test]
    async fn test_pruning_1h_bucket() {
        let db = setup_db().await;
        let pruner = HistoryPruning::new(db.clone());
        let now = Utc::now();

        // Older than 3 days, same hour. One should be deleted.
        let t1 = now - ChronoDuration::days(4);
        let t2 = t1 + ChronoDuration::minutes(10);

        db.query("CREATE item_history SET item = item:123, timestamp = <datetime>$t, buy_price = 10, sell_price = 11, buy_quantity = 100, sell_quantity = 100").bind(("t", t1)).await.unwrap();
        db.query("CREATE item_history SET item = item:123, timestamp = <datetime>$t, buy_price = 12, sell_price = 13, buy_quantity = 110, sell_quantity = 110").bind(("t", t2)).await.unwrap();

        pruner.run_pruning().await.unwrap();

        let mut res = db
            .query("SELECT * FROM item_history ORDER BY timestamp ASC")
            .await
            .unwrap();
        let remaining: Vec<HistoryRecord> = res.take(0).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].timestamp.timestamp(), t1.timestamp());
    }

    #[tokio::test]
    async fn test_pruning_3h_bucket() {
        let db = setup_db().await;
        let pruner = HistoryPruning::new(db.clone());
        let now = Utc::now();
        db.query("CREATE item:123").await.unwrap();

        // Older than 7 days, same 3h bucket.
        // We use a small offset (1 min) to stay within the same 3h block.
        let t1 = now - ChronoDuration::days(8);
        let t2 = t1 + ChronoDuration::minutes(1);

        db.query("CREATE item_history SET item = item:123, timestamp = <datetime>$t, buy_price = 10, sell_price = 11, buy_quantity = 100, sell_quantity = 100").bind(("t", t1)).await.unwrap();
        db.query("CREATE item_history SET item = item:123, timestamp = <datetime>$t, buy_price = 12, sell_price = 13, buy_quantity = 110, sell_quantity = 110").bind(("t", t2)).await.unwrap();

        pruner.run_pruning().await.unwrap();

        let mut res = db
            .query("SELECT * FROM item_history ORDER BY timestamp ASC")
            .await
            .unwrap();
        let remaining: Vec<HistoryRecord> = res.take(0).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].timestamp.timestamp(), t1.timestamp());
    }

    #[tokio::test]
    async fn test_pruning_6h_bucket() {
        let db = setup_db().await;
        let pruner = HistoryPruning::new(db.clone());
        let now = Utc::now();
        db.query("CREATE item:123").await.unwrap();

        // Older than 14 days, same 6h bucket.
        let t1 = now - ChronoDuration::days(15);
        let t2 = t1 + ChronoDuration::minutes(1);

        db.query("CREATE item_history SET item = item:123, timestamp = <datetime>$t, buy_price = 10, sell_price = 11, buy_quantity = 100, sell_quantity = 100").bind(("t", t1)).await.unwrap();
        db.query("CREATE item_history SET item = item:123, timestamp = <datetime>$t, buy_price = 12, sell_price = 13, buy_quantity = 110, sell_quantity = 110").bind(("t", t2)).await.unwrap();

        pruner.run_pruning().await.unwrap();

        let mut res = db
            .query("SELECT * FROM item_history ORDER BY timestamp ASC")
            .await
            .unwrap();
        let remaining: Vec<HistoryRecord> = res.take(0).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].timestamp.timestamp(), t1.timestamp());
    }

    #[tokio::test]
    async fn test_pruning_retention() {
        let db = setup_db().await;
        let pruner = HistoryPruning::new(db.clone());
        let now = Utc::now();

        // Within 3 days. None should be deleted even if in same hour.
        let t1 = now - ChronoDuration::days(1);
        let t2 = t1 + ChronoDuration::minutes(10);

        db.query("CREATE item_history SET item = item:123, timestamp = <datetime>$t, buy_price = 10, sell_price = 11, buy_quantity = 100, sell_quantity = 100").bind(("t", t1)).await.unwrap();
        db.query("CREATE item_history SET item = item:123, timestamp = <datetime>$t, buy_price = 12, sell_price = 13, buy_quantity = 110, sell_quantity = 110").bind(("t", t2)).await.unwrap();

        pruner.run_pruning().await.unwrap();

        let mut res = db
            .query("SELECT * FROM item_history ORDER BY timestamp ASC")
            .await
            .unwrap();
        let remaining: Vec<HistoryRecord> = res.take(0).unwrap();
        assert_eq!(remaining.len(), 2);
    }
}
