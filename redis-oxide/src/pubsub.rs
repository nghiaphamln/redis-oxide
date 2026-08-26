//! Redis Pub/Sub support using dedicated connections.

use crate::connection::RedisConnection;
use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use futures_util::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{timeout, Duration};

const CONTROL_CHANNEL_CAPACITY: usize = 64;
const MESSAGE_CHANNEL_CAPACITY: usize = 1024;
const CONTROL_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// A message received from a Redis channel.
#[derive(Debug, Clone)]
pub struct PubSubMessage {
    /// The channel the message was received on.
    pub channel: String,
    /// The message payload.
    pub payload: String,
    /// The pattern that matched, when pattern subscription was used.
    pub pattern: Option<String>,
}

enum SubscriptionCommand {
    Subscribe {
        channels: Vec<String>,
        response_tx: oneshot::Sender<RedisResult<()>>,
    },
    Unsubscribe {
        channels: Vec<String>,
        response_tx: oneshot::Sender<RedisResult<()>>,
    },
    PSubscribe {
        patterns: Vec<String>,
        response_tx: oneshot::Sender<RedisResult<()>>,
    },
    PUnsubscribe {
        patterns: Vec<String>,
        response_tx: oneshot::Sender<RedisResult<()>>,
    },
}

/// Redis Pub/Sub subscriber backed by one dedicated Redis connection.
pub struct Subscriber {
    command_tx: mpsc::Sender<SubscriptionCommand>,
    message_rx: mpsc::Receiver<RedisResult<PubSubMessage>>,
    subscribed_channels: HashMap<String, bool>,
    subscribed_patterns: HashMap<String, bool>,
}

impl Subscriber {
    /// Create a subscriber from a connection reserved for Pub/Sub.
    pub(crate) fn from_connection(connection: RedisConnection) -> Self {
        let (command_tx, command_rx) = mpsc::channel(CONTROL_CHANNEL_CAPACITY);
        let (message_tx, message_rx) = mpsc::channel(MESSAGE_CHANNEL_CAPACITY);
        tokio::spawn(Self::run_worker(connection, command_rx, message_tx));

        Self {
            command_tx,
            message_rx,
            subscribed_channels: HashMap::new(),
            subscribed_patterns: HashMap::new(),
        }
    }

    async fn run_worker(
        mut connection: RedisConnection,
        mut command_rx: mpsc::Receiver<SubscriptionCommand>,
        message_tx: mpsc::Sender<RedisResult<PubSubMessage>>,
    ) {
        loop {
            match command_rx.try_recv() {
                Ok(command) => {
                    Self::handle_command(&mut connection, command, &message_tx).await;
                    continue;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => return,
                Err(mpsc::error::TryRecvError::Empty) => {}
            }

            match timeout(CONTROL_POLL_INTERVAL, connection.read_response()).await {
                Ok(Ok(response)) => match parse_pubsub_message(response) {
                    Ok(Some(message)) => {
                        if message_tx.send(Ok(message)).await.is_err() {
                            return;
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        let _ = message_tx.send(Err(error)).await;
                        return;
                    }
                },
                Ok(Err(error)) => {
                    let _ = message_tx.send(Err(error)).await;
                    return;
                }
                Err(_) => {}
            }
        }
    }

    async fn handle_command(
        connection: &mut RedisConnection,
        command: SubscriptionCommand,
        message_tx: &mpsc::Sender<RedisResult<PubSubMessage>>,
    ) {
        let (name, confirmation, values, response_tx) = match command {
            SubscriptionCommand::Subscribe {
                channels,
                response_tx,
            } => ("SUBSCRIBE", "subscribe", channels, response_tx),
            SubscriptionCommand::Unsubscribe {
                channels,
                response_tx,
            } => ("UNSUBSCRIBE", "unsubscribe", channels, response_tx),
            SubscriptionCommand::PSubscribe {
                patterns,
                response_tx,
            } => ("PSUBSCRIBE", "psubscribe", patterns, response_tx),
            SubscriptionCommand::PUnsubscribe {
                patterns,
                response_tx,
            } => ("PUNSUBSCRIBE", "punsubscribe", patterns, response_tx),
        };

        let result =
            Self::send_subscription_command(connection, name, confirmation, values, message_tx)
                .await;
        let _ = response_tx.send(result);
    }

    async fn send_subscription_command(
        connection: &mut RedisConnection,
        command: &str,
        confirmation: &str,
        values: Vec<String>,
        message_tx: &mpsc::Sender<RedisResult<PubSubMessage>>,
    ) -> RedisResult<()> {
        let mut request = Vec::with_capacity(values.len() + 1);
        request.push(RespValue::from(command));
        request.extend(values.iter().cloned().map(RespValue::from));
        connection.send_command(&RespValue::Array(request)).await?;

        let expected_confirmations = values.len().max(1);
        let mut confirmations = 0usize;
        while confirmations < expected_confirmations {
            let response = connection.read_response_with_timeout().await?;
            if Self::is_confirmation(&response, confirmation)? {
                confirmations += 1;
                continue;
            }

            match parse_pubsub_message(response)? {
                Some(message) => {
                    message_tx
                        .send(Ok(message))
                        .await
                        .map_err(|_| RedisError::Connection("Subscriber closed".to_string()))?;
                }
                None => {
                    return Err(RedisError::UnexpectedResponse(
                        "Unexpected Pub/Sub confirmation".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn is_confirmation(response: &RespValue, expected: &str) -> RedisResult<bool> {
        let RespValue::Array(values) = response else {
            return Ok(false);
        };
        let Some(kind) = values.first() else {
            return Ok(false);
        };
        Ok(kind.as_string()? == expected)
    }

    async fn send_control(&self, command: SubscriptionCommand) -> RedisResult<()> {
        self.command_tx
            .send(command)
            .await
            .map_err(|_| RedisError::Connection("Pub/Sub worker stopped".to_string()))
    }

    /// Subscribe to one or more channels.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn subscribe(&mut self, channels: Vec<String>) -> RedisResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        self.send_control(SubscriptionCommand::Subscribe {
            channels: channels.clone(),
            response_tx,
        })
        .await?;
        response_rx
            .await
            .map_err(|_| RedisError::Connection("Pub/Sub worker stopped".to_string()))??;
        for channel in channels {
            self.subscribed_channels.insert(channel, true);
        }
        Ok(())
    }

    /// Unsubscribe from channels, or all channels when the list is empty.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn unsubscribe(&mut self, channels: Vec<String>) -> RedisResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        self.send_control(SubscriptionCommand::Unsubscribe {
            channels: channels.clone(),
            response_tx,
        })
        .await?;
        response_rx
            .await
            .map_err(|_| RedisError::Connection("Pub/Sub worker stopped".to_string()))??;
        if channels.is_empty() {
            self.subscribed_channels.clear();
        } else {
            for channel in channels {
                self.subscribed_channels.remove(&channel);
            }
        }
        Ok(())
    }

    /// Subscribe to one or more glob-style patterns.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn psubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        self.send_control(SubscriptionCommand::PSubscribe {
            patterns: patterns.clone(),
            response_tx,
        })
        .await?;
        response_rx
            .await
            .map_err(|_| RedisError::Connection("Pub/Sub worker stopped".to_string()))??;
        for pattern in patterns {
            self.subscribed_patterns.insert(pattern, true);
        }
        Ok(())
    }

    /// Unsubscribe from patterns, or all patterns when the list is empty.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn punsubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        self.send_control(SubscriptionCommand::PUnsubscribe {
            patterns: patterns.clone(),
            response_tx,
        })
        .await?;
        response_rx
            .await
            .map_err(|_| RedisError::Connection("Pub/Sub worker stopped".to_string()))??;
        if patterns.is_empty() {
            self.subscribed_patterns.clear();
        } else {
            for pattern in patterns {
                self.subscribed_patterns.remove(&pattern);
            }
        }
        Ok(())
    }

    /// Receive the next message or listener error.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn next_message(&mut self) -> RedisResult<Option<PubSubMessage>> {
        match self.message_rx.recv().await {
            Some(Ok(message)) => Ok(Some(message)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }

    /// Receive the next message until the supplied timeout expires.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn next_message_timeout(
        &mut self,
        duration: Duration,
    ) -> RedisResult<Option<PubSubMessage>> {
        match timeout(duration, self.message_rx.recv()).await {
            Ok(Some(Ok(message))) => Ok(Some(message)),
            Ok(Some(Err(error))) => Err(error),
            Ok(None) | Err(_) => Ok(None),
        }
    }

    /// List currently subscribed channels.
    #[must_use]
    pub fn subscribed_channels(&self) -> Vec<String> {
        self.subscribed_channels.keys().cloned().collect()
    }

    /// List currently subscribed patterns.
    #[must_use]
    pub fn subscribed_patterns(&self) -> Vec<String> {
        self.subscribed_patterns.keys().cloned().collect()
    }

    /// Check whether a channel is subscribed.
    #[must_use]
    pub fn is_subscribed_to_channel(&self, channel: &str) -> bool {
        self.subscribed_channels.contains_key(channel)
    }

    /// Check whether a pattern is subscribed.
    #[must_use]
    pub fn is_subscribed_to_pattern(&self, pattern: &str) -> bool {
        self.subscribed_patterns.contains_key(pattern)
    }
}

impl Stream for Subscriber {
    type Item = RedisResult<PubSubMessage>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.message_rx.poll_recv(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Publisher backed by an isolated connection.
pub struct Publisher {
    connection: Mutex<RedisConnection>,
}

impl Publisher {
    /// Create a publisher from a dedicated connection.
    pub(crate) fn from_connection(connection: RedisConnection) -> Self {
        Self {
            connection: Mutex::new(connection),
        }
    }

    /// Publish a message to a channel and return the number of receivers.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn publish(
        &self,
        channel: impl Into<String>,
        message: impl Into<String>,
    ) -> RedisResult<i64> {
        let channel = channel.into();
        let message = message.into();
        let mut connection = self.connection.lock().await;
        connection
            .execute_command(
                "PUBLISH",
                &[RespValue::from(channel), RespValue::from(message)],
            )
            .await?
            .as_int()
    }

    /// Publish multiple messages sequentially on the dedicated connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn publish_multiple(
        &self,
        messages: HashMap<String, String>,
    ) -> RedisResult<HashMap<String, i64>> {
        let mut results = HashMap::with_capacity(messages.len());
        for (channel, message) in messages {
            let count = self.publish(channel.clone(), message).await?;
            results.insert(channel, count);
        }
        Ok(results)
    }
}

/// Parse a Pub/Sub response from Redis.
///
/// # Errors
///
/// Returns an error if the operation cannot be completed.
pub fn parse_pubsub_message(response: RespValue) -> RedisResult<Option<PubSubMessage>> {
    let RespValue::Array(items) = response else {
        return Err(RedisError::Protocol(
            "Invalid Pub/Sub message format".to_string(),
        ));
    };
    let Some(message_type) = items.first() else {
        return Err(RedisError::Protocol(
            "Empty Pub/Sub message format".to_string(),
        ));
    };

    match message_type.as_string()?.as_str() {
        "message" if items.len() == 3 => Ok(Some(PubSubMessage {
            channel: items[1].as_string()?,
            payload: items[2].as_string()?,
            pattern: None,
        })),
        "pmessage" if items.len() == 4 => Ok(Some(PubSubMessage {
            pattern: Some(items[1].as_string()?),
            channel: items[2].as_string()?,
            payload: items[3].as_string()?,
        })),
        "subscribe" | "unsubscribe" | "psubscribe" | "punsubscribe" => Ok(None),
        other => Err(RedisError::Protocol(format!(
            "Unknown or malformed Pub/Sub message type: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_regular_messages() {
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
    fn parses_pattern_messages() {
        let response = RespValue::Array(vec![
            RespValue::from("pmessage"),
            RespValue::from("news*"),
            RespValue::from("news-tech"),
            RespValue::from("Tech news!"),
        ]);
        let message = parse_pubsub_message(response).unwrap().unwrap();
        assert_eq!(message.channel, "news-tech");
        assert_eq!(message.pattern.as_deref(), Some("news*"));
    }

    #[test]
    fn ignores_subscription_confirmations() {
        let response = RespValue::Array(vec![
            RespValue::from("subscribe"),
            RespValue::from("news"),
            RespValue::Integer(1),
        ]);
        assert!(parse_pubsub_message(response).unwrap().is_none());
    }
}
