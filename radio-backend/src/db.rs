/// 数据库初始化、连接池和迁移。
use crate::config::DatabaseConfig;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

/// 初始化 SQLite 数据库连接池并运行迁移。
pub(crate) async fn init_database(config: &DatabaseConfig) -> anyhow::Result<SqlitePool> {
    if config.url.starts_with("sqlite:") {
        if let Some(path) = config.url.strip_prefix("sqlite://") {
            if path.contains('/') {
                if let Some(parent) = std::path::Path::new(path).parent() {
                    std::fs::create_dir_all(parent)?;
                }
            }
        }
    }

    let options = SqliteConnectOptions::from_str(&config.url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5))
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .pragma("cache_size", "-64000");

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    tracing::info!("Database initialized successfully (WAL mode enabled)");

    Ok(pool)
}
