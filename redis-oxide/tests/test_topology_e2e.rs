//! End-to-end tests for Redis Cluster and Redis Sentinel.
//!
//! They are ignored by default because they require the Docker topology fixture.

use redis_oxide::{Client, ConnectionConfig, SentinelConfig, TopologyMode};
use std::time::{Duration, Instant};

fn cluster_url() -> String {
    std::env::var("REDIS_CLUSTER_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:7000,127.0.0.1:7001,127.0.0.1:7002".to_string())
}

fn sentinel_endpoints() -> Vec<String> {
    std::env::var("REDIS_SENTINEL_ENDPOINTS")
        .unwrap_or_else(|_| "127.0.0.1:26379,127.0.0.1:26380,127.0.0.1:26381".to_string())
        .split(',')
        .map(str::trim)
        .filter(|endpoint| !endpoint.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn sentinel_config() -> Result<SentinelConfig, redis_oxide::RedisError> {
    sentinel_endpoints()
        .into_iter()
        .try_fold(SentinelConfig::new("mymaster"), |config, endpoint| {
            config.add_sentinel(endpoint)
        })
}

#[tokio::test]
#[ignore = "requires the Redis Cluster Docker topology"]
async fn cluster_bootstraps_slots_and_routes_multiple_keys(
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect(
        ConnectionConfig::new(cluster_url()).with_topology_mode(TopologyMode::Cluster),
    )
    .await?;
    assert_eq!(
        client.topology_type(),
        redis_oxide::connection::TopologyType::Cluster
    );

    let mut keys = Vec::new();
    for bucket in 0..3 {
        let key = (0..10_000)
            .map(|index| format!("redis_oxide:cluster:{bucket}:{index}"))
            .find(|key| {
                usize::from(redis_oxide::cluster::calculate_slot(key.as_bytes())) / 5_462 == bucket
            })
            .ok_or("Could not find a key for cluster slot bucket")?;
        keys.push(key);
    }

    let mut pipeline = client.pipeline();
    for (index, key) in keys.iter().enumerate() {
        pipeline.set(key, format!("value-{index}"));
    }
    for key in &keys {
        pipeline.get(key);
    }
    let responses = pipeline.execute().await?;
    assert_eq!(responses.len(), keys.len() * 2);
    for (index, response) in responses.into_iter().skip(keys.len()).enumerate() {
        assert_eq!(response.as_string()?, format!("value-{index}"));
    }

    let mut transaction = client.transaction().await?;
    transaction.set(&keys[0], "transactional");
    assert_eq!(transaction.exec().await?.len(), 1);

    let mut cross_slot_transaction = client.transaction().await?;
    cross_slot_transaction.set(&keys[0], "first");
    cross_slot_transaction.set(&keys[1], "second");
    assert!(cross_slot_transaction.exec().await.is_err());

    for key in keys {
        client.del(vec![key.clone()]).await?;
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires the Redis Sentinel Docker topology"]
async fn sentinel_discovers_a_master_for_client_operations(
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect(ConnectionConfig::new_with_sentinel(sentinel_config()?)).await?;
    let key = "redis_oxide:sentinel:discovery";
    client.set(key, "available").await?;
    assert_eq!(client.get(key).await?, Some("available".to_string()));
    client.del(vec![key.to_string()]).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires the Redis Sentinel Docker topology"]
async fn sentinel_client_refreshes_after_master_failover() -> Result<(), Box<dyn std::error::Error>>
{
    let client = Client::connect(
        ConnectionConfig::new_with_sentinel(sentinel_config()?)
            .with_operation_timeout(Duration::from_secs(1)),
    )
    .await?;
    let key = "redis_oxide:sentinel:failover";
    client.set(key, "replicated").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut master = redis_oxide::connection::RedisConnection::connect(
        "127.0.0.1",
        6380,
        ConnectionConfig::new("redis://127.0.0.1:6380"),
    )
    .await?;
    master
        .send_command(&redis_oxide::RespValue::Array(vec![
            redis_oxide::RespValue::from("SHUTDOWN"),
            redis_oxide::RespValue::from("NOSAVE"),
        ]))
        .await?;

    let sentinel = redis_oxide::SentinelClient::new(sentinel_config()?).await?;
    let failover_deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match sentinel.refresh_master().await {
            Ok(master) if master.port == 6381 => break,
            Ok(_) | Err(_) if Instant::now() < failover_deadline => {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Ok(master) => {
                return Err(
                    format!("Unexpected Sentinel master after failover: {master:?}").into(),
                );
            }
            Err(error) => return Err(Box::<dyn std::error::Error>::from(error)),
        }
    }

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match client.get(key).await {
            Ok(Some(value)) if value == "replicated" => break,
            Ok(_) | Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Ok(value) => return Err(format!("Unexpected value after failover: {value:?}").into()),
            Err(error) => return Err(Box::<dyn std::error::Error>::from(error)),
        }
    }
    Ok(())
}
