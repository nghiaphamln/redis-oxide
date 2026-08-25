//! Regression tests for connection lifecycle and protocol configuration.

use redis_oxide::{
    Client, ConnectionConfig, PoolConfig, PoolStrategy, ProtocolVersion, RedisError,
};
use std::time::Duration;

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

#[tokio::test]
async fn selected_database_is_applied_to_every_connection() -> Result<(), Box<dyn std::error::Error>>
{
    let key = "redis_oxide:regression:database";
    let default_client = Client::connect(ConnectionConfig::new(redis_url())).await?;
    default_client.del(vec![key.to_string()]).await?;

    let database_client = Client::connect(
        ConnectionConfig::new(redis_url())
            .with_database(1)
            .with_topology_mode(redis_oxide::TopologyMode::Standalone),
    )
    .await?;
    database_client.set(key, "database-one").await?;

    assert_eq!(default_client.get(key).await?, None);
    assert_eq!(
        database_client.get(key).await?,
        Some("database-one".to_string())
    );
    database_client.del(vec![key.to_string()]).await?;
    Ok(())
}

#[tokio::test]
async fn pubsub_uses_a_dedicated_live_connection() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect(ConnectionConfig::new(redis_url())).await?;
    let mut subscriber = client.subscriber().await?;
    let channel = "redis_oxide:regression:pubsub";
    subscriber.subscribe(vec![channel.to_string()]).await?;

    assert_eq!(client.publish(channel, "delivered").await?, 1);
    let message = subscriber
        .next_message_timeout(Duration::from_secs(2))
        .await?
        .ok_or("Pub/Sub message timed out")?;
    assert_eq!(message.channel, channel);
    assert_eq!(message.payload, "delivered");
    Ok(())
}

#[tokio::test]
async fn resp3_client_executes_regular_commands() -> Result<(), Box<dyn std::error::Error>> {
    let key = "redis_oxide:regression:resp3";
    let client = Client::connect(
        ConnectionConfig::new(redis_url()).with_protocol_version(ProtocolVersion::Resp3),
    )
    .await?;
    client.set(key, "resp3").await?;
    assert_eq!(client.get(key).await?, Some("resp3".to_string()));
    client.del(vec![key.to_string()]).await?;
    Ok(())
}

#[tokio::test]
async fn connection_pool_honors_acquisition_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let stream = "redis_oxide:regression:pool-timeout";
    let config = ConnectionConfig::new(redis_url())
        .with_operation_timeout(Duration::from_secs(3))
        .with_pool_config(PoolConfig {
            strategy: PoolStrategy::Pool,
            max_size: 1,
            min_idle: 1,
            connection_timeout: Duration::from_millis(100),
        });
    let client = Client::connect(config).await?;
    client.del(vec![stream.to_string()]).await?;

    let blocking_client = client.clone();
    let blocking = tokio::spawn(async move {
        blocking_client
            .xread(
                vec![(stream.to_string(), "$".to_string())],
                None,
                Some(Duration::from_secs(1)),
            )
            .await
    });
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(matches!(
        client
            .set("redis_oxide:regression:pool-other", "blocked")
            .await,
        Err(RedisError::Timeout)
    ));
    blocking.await??;
    client
        .del(vec!["redis_oxide:regression:pool-other".to_string()])
        .await?;
    Ok(())
}
