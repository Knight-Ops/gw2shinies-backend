# GW2Shinies Backend

The backend service for GW2Shinies, responsible for data synchronization and providing a REST API for Guild Wars 2 item and price data.

## Features

- **Data Scraper**: Synchronizes item definitions, recipes, and current prices from the official Guild Wars 2 API.
- **REST API**: Provides endpoints for querying items and historical price data stored in SurrealDB.
- **Background Workers**: Automated tasks for periodic price updates and recipe discovery.

## Tech Stack

- **Rust**: Language for high-performance data processing.
- **SurrealDB**: Multi-model database for flexible data storage and graph-like relations.
- **Axum**: Modern web framework for the REST API.
- **Tokio**: Asynchronous runtime for concurrent operations.

## Configuration

The application is configured primarily through environment variables:

- `SURREAL_DB_URI`: Connection string for the SurrealDB instance (e.g., `127.0.0.1:8000`).

## Binaries

There are two main entry points defined in `Cargo.toml`:

### 1. Scraper (`scraper.rs`)

Synchronizes all data from the GW2 API to the database.

```bash
SURREAL_DB_URI=<db_uri> cargo run --bin scraper
```

### 2. API (`api.rs`)

Starts the Axum REST API server.

```bash
SURREAL_DB_URI=<db_uri> cargo run --bin api
```

## Database Schema

The SurrealDB schema is defined in `schema.surql` at the root of the backend directory. Apply this schema to your SurrealDB instance before running the services.
