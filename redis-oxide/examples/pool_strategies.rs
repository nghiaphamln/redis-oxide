//! Connection pool strategy example.

use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let multiplexed = Client::connect(
        ConnectionConfig::new("redis://localhost:6379").with_pool_config(PoolConfig {
            strategy: PoolStrategy::Multiplexed,
            ..Default::default()
        }),
    )
    .await?;
    multiplexed.set("mux:key", "value").await?;

    let pooled = Client::connect(
        ConnectionConfig::new("redis://localhost:6379").with_pool_config(PoolConfig {
            strategy: PoolStrategy::Pool,
            max_size: 10,
            min_idle: 2,
            connection_timeout: Duration::from_secs(5),
        }),
    )
    .await?;
    pooled.set("pool:key", "value").await?;
    Ok(())
}
