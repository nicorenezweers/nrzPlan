//! Server-only: Datenbank-Pool, Fehler, REST-API.

pub mod db;
pub mod events;
pub mod ics;
pub mod rest;

use std::sync::OnceLock;

use sqlx::SqlitePool;

static POOL: OnceLock<SqlitePool> = OnceLock::new();

pub fn set_pool(pool: SqlitePool) {
    let _ = POOL.set(pool);
}

pub fn pool() -> &'static SqlitePool {
    POOL.get().expect("Datenbank-Pool nicht initialisiert")
}

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized,
    Db(sqlx::Error),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound(m) | AppError::BadRequest(m) => write!(f, "{m}"),
            AppError::Unauthorized => write!(f, "Ungültiger oder fehlender API-Key"),
            AppError::Db(e) => write!(f, "Datenbankfehler: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db(e)
    }
}

/// Verbindet die SQLite-Datei (legt sie an) und führt Migrationen aus.
pub async fn init_db() -> SqlitePool {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/nrzplan.db".into());
    if let Some(path) = url.strip_prefix("sqlite://").or_else(|| url.strip_prefix("sqlite:"))
        && let Some(dir) = std::path::Path::new(path.split('?').next().unwrap_or(path)).parent()
        && !dir.as_os_str().is_empty()
    {
        let _ = tokio::fs::create_dir_all(dir).await;
    }
    let opts = SqliteConnectOptions::from_str(&url)
        .expect("DATABASE_URL ungültig")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new().max_connections(5).connect_with(opts).await.expect("Datenbank nicht erreichbar");
    sqlx::migrate!("./migrations").run(&pool).await.expect("Migrationen fehlgeschlagen");
    pool
}
