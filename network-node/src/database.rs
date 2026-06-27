use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, instrument};
use sqlx::postgres::{PgPoolOptions, PgPool};
use crate::config::DatabaseConfig;
use crate::error::{DatabaseError, Result};

/// Database connection pool manager using sqlx
#[derive(Clone)]
pub struct ConnectionPool {
    pool: PgPool,
    config: DatabaseConfig,
}

impl ConnectionPool {
    /// Create a new connection pool
    #[instrument(fields(database_url = %database_url, max_connections = %config.max_connections))]
    pub async fn new(database_url: &str) -> Result<Self> {
        let config = DatabaseConfig::from_url(database_url)?;

        info!(
            "Creating database connection pool with max {} connections",
            config.max_connections
        );

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections as u32)
            .min_connections(config.min_connections as u32)
            .acquire_timeout(config.connection_timeout)
            .idle_timeout(config.idle_timeout)
            .connect(database_url)
            .await
            .map_err(|e| crate::error::NetworkError::Database(DatabaseError::ConnectionFailed(e.to_string())))?;

        let pool_manager = Self {
            pool,
            config,
        };

        // Initialize schema
        pool_manager.initialize_schema().await?;

        Ok(pool_manager)
    }

    /// Initialize database schema
    async fn initialize_schema(&self) -> Result<()> {
        info!("Initializing database schema...");

        // Create indexer_state table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS indexer_state (
                id SERIAL PRIMARY KEY,
                last_processed_ledger INTEGER NOT NULL DEFAULT 0,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| crate::error::NetworkError::Database(DatabaseError::QueryFailed(e.to_string())))?;

        // Initialize indexer_state if empty
        sqlx::query(
            "INSERT INTO indexer_state (id, last_processed_ledger)
             SELECT 1, 0
             WHERE NOT EXISTS (SELECT 1 FROM indexer_state WHERE id = 1)"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| crate::error::NetworkError::Database(DatabaseError::QueryFailed(e.to_string())))?;

        // Create events table (standardized, query-friendly)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS events (
                id SERIAL PRIMARY KEY,
                event_id TEXT NOT NULL UNIQUE,
                ledger_sequence INTEGER NOT NULL,
                contract_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                protocol TEXT,
                action TEXT,
                user_address TEXT,
                asset_address TEXT,
                amount NUMERIC,
                timestamp BIGINT,
                event_version INTEGER DEFAULT 1,
                data JSONB NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| crate::error::NetworkError::Database(DatabaseError::QueryFailed(e.to_string())))?;

        // Create indexes for query performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_ledger ON events (ledger_sequence)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_type ON events (event_type)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_user ON events (user_address)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_asset ON events (asset_address)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events (timestamp)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_action ON events (action)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_version ON events (event_version)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS deposits (
                id SERIAL PRIMARY KEY,
                user_address TEXT NOT NULL,
                token_address TEXT NOT NULL,
                amount NUMERIC NOT NULL,
                transaction_hash TEXT,
                ledger_sequence INTEGER,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| crate::error::NetworkError::Database(DatabaseError::QueryFailed(e.to_string())))?;

        info!("Database schema initialized successfully");
        Ok(())
    }

    /// Get the underlying sqlx pool
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get active connections count
    pub fn active_connections(&self) -> u32 {
        self.pool.size() - self.pool.num_idle()
    }

    /// Close the connection pool
    pub async fn close_all(&mut self) -> Result<()> {
        info!("Closing database connection pool...");
        self.pool.close().await;
        info!("Database connection pool closed");
        Ok(())
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<bool> {
        match sqlx::query("SELECT 1").execute(&self.pool).await {
            Ok(_) => Ok(true),
            Err(e) => {
                error!("Database health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Insert an event into the events table
    pub async fn insert_event(
        &self,
        event_id: &str,
        ledger_sequence: u32,
        contract_id: &str,
        event_type: &str,
        protocol: Option<&str>,
        action: Option<&str>,
        user_address: Option<&str>,
        asset_address: Option<&str>,
        amount: Option<f64>,
        timestamp: Option<i64>,
        data: serde_json::Value,
    ) -> Result<()> {
        // Extract event_version from the data payload if present
        let event_version = data.get("event_version").and_then(|v| v.as_i64());

        sqlx::query(
            "INSERT INTO events 
             (event_id, ledger_sequence, contract_id, event_type, protocol, action, user_address, asset_address, amount, timestamp, event_version, data)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             ON CONFLICT (event_id) DO NOTHING"
        )
        .bind(event_id)
        .bind(ledger_sequence as i32)
        .bind(contract_id)
        .bind(event_type)
        .bind(protocol)
        .bind(action)
        .bind(user_address)
        .bind(asset_address)
        .bind(amount)
        .bind(timestamp)
        .bind(event_version)
        .bind(data)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::error::NetworkError::Database(DatabaseError::QueryFailed(e.to_string())))?;
        Ok(())
    }

    /// Get events by user address
    pub async fn get_events_by_user(
        &self,
        user_address: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT data FROM events WHERE user_address = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3"
        )
        .bind(user_address)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::error::NetworkError::Database(DatabaseError::QueryFailed(e.to_string())))?;
        Ok(rows.into_iter().map(|r| r.get(0)).collect())
    }

    /// Get all events with pagination
    pub async fn get_all_events(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT data FROM events ORDER BY timestamp DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::error::NetworkError::Database(DatabaseError::QueryFailed(e.to_string())))?;
        Ok(rows.into_iter().map(|r| r.get(0)).collect())
    }

    /// Update last processed ledger
    pub async fn update_last_processed_ledger(&self, ledger: u32) -> Result<()> {
        sqlx::query(
            "UPDATE indexer_state SET last_processed_ledger = $1, updated_at = CURRENT_TIMESTAMP WHERE id = 1"
        )
        .bind(ledger as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::error::NetworkError::Database(DatabaseError::QueryFailed(e.to_string())))?;
        Ok(())
    }

    /// Get last processed ledger
    pub async fn get_last_processed_ledger(&self) -> Result<u32> {
        let row: (i32,) = sqlx::query_as("SELECT last_processed_ledger FROM indexer_state WHERE id = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| crate::error::NetworkError::Database(DatabaseError::QueryFailed(e.to_string())))?;
        Ok(row.0 as u32)
    }
}
