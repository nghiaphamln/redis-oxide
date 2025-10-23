//! Pub/Sub support for Redis
//!
//! This module provides functionality for Redis publish/subscribe messaging.
//! Redis Pub/Sub allows you to send messages between different parts of your
//! application or between different applications.
//!
//! # Examples
//!
//! ## Publisher
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig::new("redis://localhost:6379");
//! let client = Client::connect(config).await?;
//!
//! // Publish a message to a channel
//! let subscribers = client.publish("news", "Breaking news!").await?;
//! println!("Message sent to {} subscribers", subscribers);
//! # Ok(())
//! # }
//! ```
//!
//! ## Subscriber
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//! use futures::StreamExt;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig::new("redis://localhost:6379");
//! let client = Client::connect(config).await?;
//!
//! // Subscribe to channels
//! let mut subscriber = client.subscriber().await?;
//! subscriber.subscribe(vec!["news".to_string(), "updates".to_string()]).await?;
//!
//! // Listen for messages
//! while let Some(message) = subscriber.next_message().await? {
//!     println!("Received: {} on channel {}", message.payload, message.channel);
//! }
//! # Ok(())
//! # }
//! ```

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use futures_util::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};

/// A message received from a Redis channel
#[derive(Debug, Clone)]
pub struct PubSubMessage {
    /// The channel the message was received on
    pub channel: String,
    /// The message payload
    pub payload: String,
    /// The pattern that matched (for pattern subscriptions)
    pub pattern: Option<String>,
}

/// Redis Pub/Sub subscriber
pub struct Subscriber {
    connection: Arc<Mutex<dyn PubSubConnection + Send + Sync>>,
    message_rx: mpsc::UnboundedReceiver<PubSubMessage>,
    subscribed_channels: HashMap<String, bool>,
    subscribed_patterns: HashMap<String, bool>,
}

/// Trait for Pub/Sub connections
#[async_trait::async_trait]
pub trait PubSubConnection {
    /// Subscribe to channels
    async fn subscribe(&mut self, channels: Vec<String>) -> RedisResult<()>;

    /// Unsubscribe from channels
    async fn unsubscribe(&mut self, channels: Vec<String>) -> RedisResult<()>;

    /// Subscribe to patterns
    async fn psubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()>;

    /// Unsubscribe from patterns
    async fn punsubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()>;

    /// Start listening for messages
    async fn listen(&mut self, message_tx: mpsc::UnboundedSender<PubSubMessage>)
        -> RedisResult<()>;

    /// Publish a message to a channel
    async fn publish(&mut self, channel: String, message: String) -> RedisResult<i64>;
}

impl Subscriber {
    /// Create a new subscriber
    pub fn new(connection: Arc<Mutex<dyn PubSubConnection + Send + Sync>>) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        // Start listening for messages in the background
        let conn_clone = connection.clone();
        tokio::spawn(async move {
            let mut conn = conn_clone.lock().await;
            if let Err(e) = conn.listen(message_tx).await {
                eprintln!("Pub/Sub listener error: {}", e);
            }
        });

        Self {
            connection,
            message_rx,
            subscribed_channels: HashMap::new(),
            subscribed_patterns: HashMap::new(),
        }
    }

    /// Subscribe to one or more channels
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let mut subscriber = client.subscriber().await?;
    ///
    /// // Subscribe to multiple channels
    /// subscriber.subscribe(vec![
    ///     "news".to_string(),
    ///     "updates".to_string(),
    ///     "alerts".to_string()
    /// ]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn subscribe(&mut self, channels: Vec<String>) -> RedisResult<()> {
        let mut connection = self.connection.lock().await;
        connection.subscribe(channels.clone()).await?;

        for channel in channels {
            self.subscribed_channels.insert(channel, true);
        }

        Ok(())
    }

    /// Unsubscribe from one or more channels
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let mut subscriber = client.subscriber().await?;
    /// subscriber.subscribe(vec!["news".to_string()]).await?;
    ///
    /// // Later, unsubscribe
    /// subscriber.unsubscribe(vec!["news".to_string()]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unsubscribe(&mut self, channels: Vec<String>) -> RedisResult<()> {
        let mut connection = self.connection.lock().await;
        connection.unsubscribe(channels.clone()).await?;

        for channel in channels {
            self.subscribed_channels.remove(&channel);
        }

        Ok(())
    }

    /// Subscribe to one or more patterns
    ///
    /// Patterns support glob-style matching:
    /// - `*` matches any sequence of characters
    /// - `?` matches any single character
    /// - `[abc]` matches any character in the set
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let mut subscriber = client.subscriber().await?;
    ///
    /// // Subscribe to all channels starting with "news"
    /// subscriber.psubscribe(vec!["news*".to_string()]).await?;
    ///
    /// // Subscribe to all channels ending with "log"
    /// subscriber.psubscribe(vec!["*log".to_string()]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn psubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()> {
        let mut connection = self.connection.lock().await;
        connection.psubscribe(patterns.clone()).await?;

        for pattern in patterns {
            self.subscribed_patterns.insert(pattern, true);
        }

        Ok(())
    }

    /// Unsubscribe from one or more patterns
    pub async fn punsubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()> {
        let mut connection = self.connection.lock().await;
        connection.punsubscribe(patterns.clone()).await?;

        for pattern in patterns {
            self.subscribed_patterns.remove(&pattern);
        }

        Ok(())
    }

    /// Get the next message from subscribed channels
    ///
    /// This method will block until a message is received or an error occurs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let mut subscriber = client.subscriber().await?;
    /// subscriber.subscribe(vec!["news".to_string()]).await?;
    ///
    /// // Wait for the next message
    /// if let Some(message) = subscriber.next_message().await? {
    ///     println!("Received: {} on {}", message.payload, message.channel);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn next_message(&mut self) -> RedisResult<Option<PubSubMessage>> {
        match self.message_rx.recv().await {
            Some(message) => Ok(Some(message)),
            None => Ok(None), // Channel closed
        }
    }

    /// Get the next message with a timeout
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # use std::time::Duration;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let mut subscriber = client.subscriber().await?;
    /// subscriber.subscribe(vec!["news".to_string()]).await?;
    ///
    /// // Wait for a message with 5 second timeout
    /// match subscriber.next_message_timeout(Duration::from_secs(5)).await? {
    ///     Some(message) => println!("Received: {}", message.payload),
    ///     None => println!("No message received within timeout"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn next_message_timeout(
        &mut self,
        duration: Duration,
    ) -> RedisResult<Option<PubSubMessage>> {
        match timeout(duration, self.message_rx.recv()).await {
            Ok(Some(message)) => Ok(Some(message)),
            Ok(None) => Ok(None), // Channel closed
            Err(_) => Ok(None),   // Timeout
        }
    }

    /// Get a list of currently subscribed channels
    #[must_use]
    pub fn subscribed_channels(&self) -> Vec<String> {
        self.subscribed_channels.keys().cloned().collect()
    }

    /// Get a list of currently subscribed patterns
    #[must_use]
    pub fn subscribed_patterns(&self) -> Vec<String> {
        self.subscribed_patterns.keys().cloned().collect()
    }

    /// Check if subscribed to a specific channel
    #[must_use]
    pub fn is_subscribed_to_channel(&self, channel: &str) -> bool {
        self.subscribed_channels.contains_key(channel)
    }

    /// Check if subscribed to a specific pattern
    #[must_use]
    pub fn is_subscribed_to_pattern(&self, pattern: &str) -> bool {
        self.subscribed_patterns.contains_key(pattern)
    }
}

/// Stream implementation for Subscriber
impl Stream for Subscriber {
    type Item = RedisResult<PubSubMessage>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.message_rx.poll_recv(cx) {
            Poll::Ready(Some(message)) => Poll::Ready(Some(Ok(message))),
            Poll::Ready(None) => Poll::Ready(None), // Channel closed
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Publisher for sending messages to Redis channels
pub struct Publisher {
    connection: Arc<Mutex<dyn PubSubConnection + Send + Sync>>,
}

impl Publisher {
    /// Create a new publisher
    pub fn new(connection: Arc<Mutex<dyn PubSubConnection + Send + Sync>>) -> Self {
        Self { connection }
    }

    /// Publish a message to a channel
    ///
    /// Returns the number of subscribers that received the message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let publisher = client.publisher().await?;
    ///
    /// let subscribers = publisher.publish("news", "Breaking news!").await?;
    /// println!("Message delivered to {} subscribers", subscribers);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn publish(
        &self,
        channel: impl Into<String>,
        message: impl Into<String>,
    ) -> RedisResult<i64> {
        let mut connection = self.connection.lock().await;
        connection.publish(channel.into(), message.into()).await
    }

    /// Publish multiple messages to different channels
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # use std::collections::HashMap;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let publisher = client.publisher().await?;
    ///
    /// let mut messages = HashMap::new();
    /// messages.insert("news".to_string(), "Breaking news!".to_string());
    /// messages.insert("updates".to_string(), "System update available".to_string());
    ///
    /// let results = publisher.publish_multiple(messages).await?;
    /// for (channel, count) in results {
    ///     println!("Channel {}: {} subscribers", channel, count);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn publish_multiple(
        &self,
        messages: HashMap<String, String>,
    ) -> RedisResult<HashMap<String, i64>> {
        let mut results = HashMap::new();

        for (channel, message) in messages {
            let count = self.publish(&channel, message).await?;
            results.insert(channel, count);
        }

        Ok(results)
    }
}

/// Pub/Sub message types for internal parsing
#[derive(Debug)]
enum PubSubMessageType {
    Subscribe,
    Unsubscribe,
    Message,
    PSubscribe,
    PUnsubscribe,
    PMessage,
}

impl PubSubMessageType {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "subscribe" => Some(Self::Subscribe),
            "unsubscribe" => Some(Self::Unsubscribe),
            "message" => Some(Self::Message),
            "psubscribe" => Some(Self::PSubscribe),
            "punsubscribe" => Some(Self::PUnsubscribe),
            "pmessage" => Some(Self::PMessage),
            _ => None,
        }
    }
}

/// Parse a Pub/Sub message from Redis response
pub fn parse_pubsub_message(response: RespValue) -> RedisResult<Option<PubSubMessage>> {
    match response {
        RespValue::Array(items) if items.len() >= 3 => {
            let message_type = items[0].as_string()?;
            let msg_type = PubSubMessageType::from_str(&message_type);

            match msg_type {
                Some(PubSubMessageType::Message) => {
                    let channel = items[1].as_string()?;
                    let payload = items[2].as_string()?;

                    Ok(Some(PubSubMessage {
                        channel,
                        payload,
                        pattern: None,
                    }))
                }
                Some(PubSubMessageType::PMessage) if items.len() >= 4 => {
                    let pattern = items[1].as_string()?;
                    let channel = items[2].as_string()?;
                    let payload = items[3].as_string()?;

                    Ok(Some(PubSubMessage {
                        channel,
                        payload,
                        pattern: Some(pattern),
                    }))
                }
                Some(
                    PubSubMessageType::Subscribe
                    | PubSubMessageType::Unsubscribe
                    | PubSubMessageType::PSubscribe
                    | PubSubMessageType::PUnsubscribe,
                ) => {
                    // These are subscription confirmations, not actual messages
                    Ok(None)
                }
                _ => Err(RedisError::Protocol(format!(
                    "Unknown pub/sub message type: {}",
                    message_type
                ))),
            }
        }
        _ => Err(RedisError::Protocol(format!(
            "Invalid pub/sub message format: {:?}",
            response
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockPubSubConnection {
        published_messages: Vec<(String, String)>,
        subscribed_channels: Vec<String>,
        subscribed_patterns: Vec<String>,
    }

    impl MockPubSubConnection {
        fn new() -> Self {
            Self {
                published_messages: Vec::new(),
                subscribed_channels: Vec::new(),
                subscribed_patterns: Vec::new(),
            }
        }
    }

    #[async_trait::async_trait]
    impl PubSubConnection for MockPubSubConnection {
        async fn subscribe(&mut self, channels: Vec<String>) -> RedisResult<()> {
            self.subscribed_channels.extend(channels);
            Ok(())
        }

        async fn unsubscribe(&mut self, channels: Vec<String>) -> RedisResult<()> {
            for channel in channels {
                self.subscribed_channels.retain(|c| c != &channel);
            }
            Ok(())
        }

        async fn psubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()> {
            self.subscribed_patterns.extend(patterns);
            Ok(())
        }

        async fn punsubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()> {
            for pattern in patterns {
                self.subscribed_patterns.retain(|p| p != &pattern);
            }
            Ok(())
        }

        async fn listen(
            &mut self,
            _message_tx: mpsc::UnboundedSender<PubSubMessage>,
        ) -> RedisResult<()> {
            // Mock implementation - would normally listen for messages
            Ok(())
        }

        async fn publish(&mut self, channel: String, message: String) -> RedisResult<i64> {
            self.published_messages.push((channel, message));
            Ok(1) // Mock: 1 subscriber
        }
    }

    #[tokio::test]
    async fn test_subscriber_creation() {
        let connection = MockPubSubConnection::new();
        let subscriber = Subscriber::new(Arc::new(Mutex::new(connection)));

        assert!(subscriber.subscribed_channels().is_empty());
        assert!(subscriber.subscribed_patterns().is_empty());
    }

    #[tokio::test]
    async fn test_subscriber_subscribe() {
        let connection = MockPubSubConnection::new();
        let mut subscriber = Subscriber::new(Arc::new(Mutex::new(connection)));

        subscriber
            .subscribe(vec!["news".to_string(), "updates".to_string()])
            .await
            .unwrap();

        assert_eq!(subscriber.subscribed_channels().len(), 2);
        assert!(subscriber.is_subscribed_to_channel("news"));
        assert!(subscriber.is_subscribed_to_channel("updates"));
    }

    #[tokio::test]
    async fn test_subscriber_unsubscribe() {
        let connection = MockPubSubConnection::new();
        let mut subscriber = Subscriber::new(Arc::new(Mutex::new(connection)));

        subscriber
            .subscribe(vec!["news".to_string(), "updates".to_string()])
            .await
            .unwrap();
        subscriber
            .unsubscribe(vec!["news".to_string()])
            .await
            .unwrap();

        assert_eq!(subscriber.subscribed_channels().len(), 1);
        assert!(!subscriber.is_subscribed_to_channel("news"));
        assert!(subscriber.is_subscribed_to_channel("updates"));
    }

    #[tokio::test]
    async fn test_publisher_publish() {
        let connection = MockPubSubConnection::new();
        let publisher = Publisher::new(Arc::new(Mutex::new(connection)));

        let count = publisher.publish("news", "Breaking news!").await.unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_parse_pubsub_message() {
        // Test regular message
        let response = RespValue::Array(vec![
            RespValue::from("message"),
            RespValue::from("news"),
            RespValue::from("Breaking news!"),
        ]);

        let message = parse_pubsub_message(response).unwrap().unwrap();
        assert_eq!(message.channel, "news");
        assert_eq!(message.payload, "Breaking news!");
        assert!(message.pattern.is_none());
    }

    #[test]
    fn test_parse_pubsub_pattern_message() {
        // Test pattern message
        let response = RespValue::Array(vec![
            RespValue::from("pmessage"),
            RespValue::from("news*"),
            RespValue::from("news-tech"),
            RespValue::from("Tech news!"),
        ]);

        let message = parse_pubsub_message(response).unwrap().unwrap();
        assert_eq!(message.channel, "news-tech");
        assert_eq!(message.payload, "Tech news!");
        assert_eq!(message.pattern, Some("news*".to_string()));
    }

    #[test]
    fn test_parse_pubsub_subscribe_confirmation() {
        // Test subscription confirmation (should return None)
        let response = RespValue::Array(vec![
            RespValue::from("subscribe"),
            RespValue::from("news"),
            RespValue::Integer(1),
        ]);

        let message = parse_pubsub_message(response).unwrap();
        assert!(message.is_none());
    }
}
