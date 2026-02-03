# Pub/Sub

Publish/Subscribe allows you to send and receive messages on channels.

## Subscribe to Channels

```rust
use redis_oxide::{Client, ConnectionConfig, PubSubConnection};

let mut pubsub = client.pubsub().await?;

let channels = vec!["channel1", "channel2"];
let subscriptions = pubsub.subscribe(channels).await?;

for sub in &subscriptions {
    println!("Subscribed to: {} - {}", sub.0, sub.1);
}
```

## Subscribe to Patterns

```rust
let patterns = vec!["news.*", "updates.*"];
let subscriptions = pubsub.psubscribe(patterns).await?;

for sub in &subscriptions {
    println!("Pattern subscribed: {} - {}", sub.0, sub.1);
}
```

## Publish Messages

```rust
use redis_oxide::PubSubConnection;

let channel = "mychannel";
let message = "Hello, subscribers!";

let subscriber_count = pubsub.publish(channel.to_string(), message.to_string()).await?;
println!("Message sent to {} subscribers", subscriber_count);
```

## Unsubscribe

```rust
// Unsubscribe from specific channels
let unsubscriptions = pubsub.unsubscribe(vec!["channel1"]).await?;

// Unsubscribe from patterns
let punsubscriptions = pubsub.punsubscribe(vec!["news.*"]).await?;
```

## Message Listening

```rust
use redis_oxide::PubSubMessage;

loop {
    match pubsub.listen().await {
        Ok(msg) => {
            match msg {
                PubSubMessage::Message(channel, message) => {
                    println!("[{}] {}", channel, message);
                }
                PubSubMessage::PMessage(pattern, channel, message) => {
                    println!("[{} via {}] {}", pattern, channel, message);
                }
                PubSubMessage::Subscribe(channel, count) => {
                    println!("Subscribed to {}, total: {}", channel, count);
                }
                PubSubMessage::Unsubscribe(channel, count) => {
                    println!("Unsubscribed from {}, total: {}", channel, count);
                }
                PubSubMessage::PSubscribe(pattern, count) => {
                    println!("Pattern subscribed {}, total: {}", pattern, count);
                }
                PubSubMessage::PUnsubscribe(pattern, count) => {
                    println!("Pattern unsubscribed {}, total: {}", pattern, count);
                }
                _ => {}
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            break;
        }
    }
}
```

## PubSubConnection API

```rust
// Create PubSub connection
let mut pubsub = client.pubsub().await?;

// Subscribe to channels
pubsub.subscribe(vec!["channel1"]).await?;

// Subscribe to patterns
pubsub.psubscribe(vec!["news.*"]).await?;

// Publish message
pubsub.publish("channel", "message").await?;

// Unsubscribe
pubsub.unsubscribe(vec!["channel1"]).await?;
pubsub.punsubscribe(vec!["news.*"]).await?;

// Listen for messages
let message = pubsub.listen().await?;
```

## Best Practices

1. **Use separate connection** for Pub/Sub (dedicated connection)
2. **Handle reconnection** for long-lived subscriptions
3. **Use patterns** for dynamic channel subscriptions
4. **Unsubscribe properly** when done

## Message Types

| Type | Description |
|------|-------------|
| `Message` | Regular channel message |
| `PMessage` | Pattern-matched message |
| `Subscribe` | Confirmation of channel subscription |
| `Unsubscribe` | Confirmation of channel unsubscription |
| `PSubscribe` | Confirmation of pattern subscription |
| `PUnsubscribe` | Confirmation of pattern unsubscription |
