use clap::Parser;
use gw2shinies_backend::history_pruning::HistoryPruning;
use gw2shinies_backend::item_sync::ItemSync;
use gw2shinies_backend::price_sync::PriceSync;
use gw2shinies_backend::{Args, Database};

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let database = Database::init(&args.surreal_uri, &args.surreal_user, &args.surreal_pass)
        .await
        .expect("Failed to initialize database");

    let token = tokio_util::sync::CancellationToken::new();

    // Orderly Background Startup
    let item_sync = ItemSync::new(database.db.clone());
    let price_sync = PriceSync::new(database.db.clone());
    let history_pruning = HistoryPruning::new(database.db.clone());

    // 1. Initial Item Sync (Crucial for other tasks)
    println!("Performing initial item sync...");
    if let Err(e) = item_sync.run_sync().await {
        eprintln!("Initial item sync failed: {}", e);
    }

    // 2. Start Price Sync and History Recovery
    let price_sync_periodic = price_sync.clone();
    let token_periodic = token.clone();
    let handle_periodic = tokio::spawn(async move {
        price_sync_periodic
            .spawn(std::time::Duration::from_secs(900), token_periodic)
            .await;
    });

    let price_sync_recovery = price_sync.clone();
    let token_recovery = token.clone();
    let handle_recovery = tokio::spawn(async move {
        if let Err(e) = price_sync_recovery.recover_history(token_recovery).await {
            eprintln!("History recovery failed: {}", e);
        }
    });

    let history_pruning_worker = history_pruning.clone();
    let token_pruning = token.clone();
    let handle_pruning = tokio::spawn(async move {
        // Run every 24 hours
        history_pruning_worker
            .spawn(std::time::Duration::from_secs(86400), token_pruning)
            .await;
    });

    // 3. Keep Item Sync running daily
    let item_sync_worker = item_sync.clone();
    let token_item = token.clone();
    let handle_item = tokio::spawn(async move {
        item_sync_worker
            .spawn(std::time::Duration::from_secs(86400), token_item)
            .await;
    });

    // 4. Wait for shutdown signal
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl-C");
    println!("Shutdown signal received. Gracefully shutting down workers...");
    token.cancel();

    // Wait for all workers to finish
    let _ = tokio::join!(
        handle_periodic,
        handle_recovery,
        handle_pruning,
        handle_item
    );
    println!("All workers shut down. Exiting.");
}
