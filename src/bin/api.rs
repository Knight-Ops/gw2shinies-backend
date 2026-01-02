use axum::{Json, Router, routing::get};
use clap::Parser;
use gw2shinies_backend::{Args, DBItem, Database, ItemParams};
use serde::Serialize;

#[derive(Serialize)]
struct HealthCheck {
    status: String,
    message: String,
}

async fn health_handler() -> Json<HealthCheck> {
    Json(HealthCheck {
        status: "ok".to_string(),
        message: "Skritt colony active. Yes.".to_string(),
    })
}

async fn get_items_handler(
    axum::extract::State(db): axum::extract::State<surrealdb::Surreal<surrealdb::engine::any::Any>>,
    axum::extract::Query(params): axum::extract::Query<ItemParams>,
) -> Result<Json<Vec<DBItem>>, (axum::http::StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).min(100);
    let page = params.page.unwrap_or(1);
    let start = (page - 1) * limit;

    let mut query_string = "SELECT *, 
        (math::round((sells.unit_price OR 0) * 0.85) - (buys.unit_price OR 0)) AS profit,
        (IF (buys.unit_price OR 0) > 0 THEN (math::round((sells.unit_price OR 0) * 0.85) - (buys.unit_price OR 0)) / (buys.unit_price OR 0) * 100 ELSE 0 END) AS roi
        FROM item".to_string();
    let mut bindings: Vec<(String, String)> = Vec::new();

    if let Some(search) = params.search {
        if !search.is_empty() {
            // Basic case-insensitive search
            query_string
                .push_str(" WHERE string::lowercase(name) CONTAINS string::lowercase($search)");
            bindings.push(("search".to_string(), search));
        }
    }

    // Default sort by profit descending if no search, otherwise maybe just relevance?
    // For now let's just add a basic sort
    query_string.push_str(" ORDER BY profit DESC");

    query_string.push_str(&format!(" LIMIT {} START {}", limit, start));

    let mut response = db.query(query_string);

    for (key, value) in bindings {
        response = response.bind((key, value));
    }

    match response.await {
        Ok(mut result) => {
            let items: Vec<DBItem> = result.take(0).map_err(|e| {
                eprintln!("Failed to parse items: {}", e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to parse data".to_string(),
                )
            })?;
            println!(
                "Fetched {} items (Page {}, Limit {})",
                items.len(),
                page,
                limit
            );
            Ok(Json(items))
        }
        Err(e) => {
            eprintln!("Failed to fetch items: {}", e);
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ))
        }
    }
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let database = Database::init(&args.surreal_uri)
        .await
        .expect("Failed to initialize database");

    // build our application with a route
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/items", get(get_items_handler))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(database.db);

    // run our app with hyper
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}
