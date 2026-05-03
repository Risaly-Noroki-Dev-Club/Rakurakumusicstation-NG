/// 数据库初始化、连接池和迁移。

use crate::config::DatabaseConfig;
use crate::config::StationConfig;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::RwLock;

/// 所有请求处理器共享的应用状态。
pub struct AppState {
    pub db: SqlitePool,
    pub redis_conn: Option<redis::aio::ConnectionManager>,
    pub config: crate::config::AppConfig,
    pub station: RwLock<StationConfig>,
    pub jwt_secret: String,
    pub ws_tx: tokio::sync::broadcast::Sender<String>,
}

impl AppState {
    /// 创建包含所有已初始化组件的新 AppState。
    pub async fn new(config: crate::config::AppConfig) -> anyhow::Result<Self> {
        // 初始化数据库
        let db = init_database(&config.database).await?;

        // 初始化 Redis 连接（可选，失败时不阻止启动）
        let redis_conn = init_redis(&config.redis).await;

        // WebSocket 广播通道（容量为 1024 条消息）
        let (ws_tx, _) = tokio::sync::broadcast::channel(1024);

        let jwt_secret = config.jwt.secret.clone();
        let station = RwLock::new(config.station.clone());

        Ok(Self {
            db,
            redis_conn,
            config,
            station,
            jwt_secret,
            ws_tx,
        })
    }
}

/// 初始化 SQLite 数据库连接池并运行迁移。
async fn init_database(config: &DatabaseConfig) -> anyhow::Result<SqlitePool> {
    // 确保 SQLite 的数据目录存在
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
        .foreign_keys(true);   // 在 SQLite 中启用外键约束

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;

    // 从 migrations/ 目录运行迁移
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    tracing::info!("Database initialized successfully");

    Ok(pool)
}

    /// 初始化 Redis 连接（失败时返回 None，不阻止服务器启动）。
    async fn init_redis(config: &crate::config::RedisConfig) -> Option<redis::aio::ConnectionManager> {
        match redis::Client::open(config.url.as_str()) {
            Ok(client) => match redis::aio::ConnectionManager::new(client).await {
                Ok(conn) => {
                    tracing::info!("Redis connection established: {}", config.url);
                    Some(conn)
                }
                Err(e) => {
                    tracing::warn!("Redis connection failed ({}): {}, Redis features disabled", config.url, e);
                    None
                }
            },
            Err(e) => {
                tracing::warn!("Redis client error ({}): {}, Redis features disabled", config.url, e);
                None
            }
        }
    }
