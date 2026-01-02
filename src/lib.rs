use clap::Parser;
use surrealdb::Surreal;
use surrealdb::engine::any::{Any, connect};

pub mod gw2_api;
pub mod history_pruning;
pub mod history_record;
pub mod item_definition;
pub mod item_sync;
pub mod price_sync;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PriceDetail {
    pub quantity: u32,
    pub unit_price: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DBItem {
    pub id: surrealdb::sql::Thing,
    pub gw2_id: u32,
    pub name: String,
    pub icon: Option<String>,
    pub rarity: String,
    pub buys: Option<PriceDetail>,
    pub sells: Option<PriceDetail>,
    pub profit: Option<f64>,
    pub roi: Option<f32>,
}

#[derive(serde::Deserialize)]
pub struct ItemParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub search: Option<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        env = "SURREAL_DB_URI",
        default_value = "ws://127.0.0.1:8000"
    )]
    pub surreal_uri: String,

    #[arg(long, env = "SURREAL_USER", default_value = "root")]
    pub surreal_user: String,

    #[arg(long, env = "SURREAL_PASS", default_value = "root")]
    pub surreal_pass: String,
}

// Database connection placeholder
pub struct Database {
    pub db: Surreal<Any>,
}

impl Database {
    pub async fn init(uri: &str, user: &str, pass: &str) -> surrealdb::Result<Self> {
        let db = connect(uri).await?;
        db.signin(surrealdb::opt::auth::Root {
            username: user,
            password: pass,
        })
        .await?;
        db.use_ns("gw2shinies").use_db("colony_brain").await?;
        Ok(Self { db })
    }
}
