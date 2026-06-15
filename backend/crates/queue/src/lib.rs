use anyhow::{Context, Result};
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

const MAX_STREAM_LEN: usize = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamName {
    RawEvents,
    SignalsNormalized,
    CopyIntents,
    ExecutionReports,
}

impl StreamName {
    pub fn as_str(self) -> &'static str {
        match self {
            StreamName::RawEvents => "raw.events",
            StreamName::SignalsNormalized => "signals.normalized",
            StreamName::CopyIntents => "copy_intents",
            StreamName::ExecutionReports => "execution.reports",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MessageEnvelope {
    pub id: Uuid,
    pub event_type: String,
    pub emitted_at: OffsetDateTime,
    pub payload: Value,
}

impl MessageEnvelope {
    pub fn new(event_type: impl Into<String>, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            emitted_at: OffsetDateTime::now_utc(),
            payload,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublishedMessage {
    pub stream: String,
    pub message_id: String,
}

#[derive(Clone)]
pub struct RedisStreamPublisher {
    manager: redis::aio::ConnectionManager,
}

impl RedisStreamPublisher {
    pub async fn connect(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let manager = client.get_connection_manager().await?;
        Ok(Self { manager })
    }

    pub async fn publish(
        &self,
        stream: StreamName,
        envelope: &MessageEnvelope,
    ) -> Result<PublishedMessage> {
        let stream_name = stream.as_str();
        let payload = serde_json::to_string(envelope)?;
        let emitted_at = envelope.emitted_at.unix_timestamp().to_string();
        let mut conn = self.manager.clone();
        let message_id: String = redis::cmd("XADD")
            .arg(stream_name)
            .arg("MAXLEN")
            .arg(MAX_STREAM_LEN)
            .arg("*")
            .arg("id")
            .arg(envelope.id.to_string())
            .arg("event_type")
            .arg(envelope.event_type.clone())
            .arg("emitted_at")
            .arg(emitted_at)
            .arg("payload")
            .arg(payload)
            .query_async(&mut conn)
            .await?;

        Ok(PublishedMessage {
            stream: stream_name.to_owned(),
            message_id,
        })
    }
}

#[derive(Clone)]
pub struct RedisStreamConsumer {
    manager: redis::aio::ConnectionManager,
    group_name: String,
    consumer_name: String,
}

impl RedisStreamConsumer {
    pub async fn connect(
        redis_url: &str,
        group_name: impl Into<String>,
        consumer_name: impl Into<String>,
    ) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let manager = client.get_connection_manager().await?;
        Ok(Self {
            manager,
            group_name: group_name.into(),
            consumer_name: consumer_name.into(),
        })
    }

    pub async fn create_groups(&self, streams: &[StreamName]) -> Result<()> {
        let mut conn = self.manager.clone();
        for stream in streams {
            let result: std::result::Result<String, redis::RedisError> = redis::cmd("XGROUP")
                .arg("CREATE")
                .arg(stream.as_str())
                .arg(&self.group_name)
                .arg("$")
                .arg("MKSTREAM")
                .query_async(&mut conn)
                .await;
            if let Err(e) = &result {
                let msg = e.to_string();
                if msg.contains("already exists") || msg.contains("Consumer Group") {
                    continue;
                }
                return Err(anyhow::anyhow!(
                    "failed to create consumer group for stream {}: {}",
                    stream.as_str(),
                    e
                ));
            }
        }
        Ok(())
    }

    pub async fn read(
        &self,
        streams: &[StreamName],
        count: usize,
        block_ms: usize,
    ) -> Result<Vec<(StreamName, Vec<(String, MessageEnvelope)>)>> {
        let mut conn = self.manager.clone();
        let keys: Vec<&str> = streams.iter().map(|s| s.as_str()).collect();
        let ids: Vec<&str> = std::iter::repeat(">").take(streams.len()).collect();

        let opts = StreamReadOptions::default()
            .group(&self.group_name, &self.consumer_name)
            .count(count)
            .block(block_ms);

        let reply: StreamReadReply = conn
            .xread_options(&keys, &ids, &opts)
            .await
            .unwrap_or(StreamReadReply { keys: vec![] });

        let mut result = Vec::new();
        for stream_key in reply.keys {
            let stream_name = stream_key.key;
            let name = match stream_name.as_str() {
                "raw.events" => StreamName::RawEvents,
                "signals.normalized" => StreamName::SignalsNormalized,
                "copy_intents" => StreamName::CopyIntents,
                "execution.reports" => StreamName::ExecutionReports,
                _ => continue,
            };
            let mut envelopes = Vec::new();
            for stream_id in stream_key.ids {
                if let Ok(env) = map_redis_id_to_envelope(stream_id.map) {
                    envelopes.push((stream_id.id, env));
                }
            }
            if !envelopes.is_empty() {
                result.push((name, envelopes));
            }
        }

        Ok(result)
    }

    pub async fn ack(
        &self,
        stream: StreamName,
        ids: &[impl redis::ToRedisArgs + Send + Sync],
    ) -> Result<()> {
        let mut conn = self.manager.clone();
        let stream_name = stream.as_str();
        if ids.is_empty() {
            return Ok(());
        }
        let _: () = conn.xack(stream_name, &self.group_name, ids).await?;
        Ok(())
    }
}

fn map_redis_id_to_envelope(
    map: std::collections::HashMap<String, redis::Value>,
) -> Result<MessageEnvelope> {
    let payload_str = match map.get("payload") {
        Some(redis::Value::SimpleString(s)) => s.clone(),
        Some(redis::Value::BulkString(b)) => String::from_utf8_lossy(b).to_string(),
        _ => return Err(anyhow::anyhow!("missing payload")),
    };
    let envelope: MessageEnvelope =
        serde_json::from_str(&payload_str).context("failed to parse envelope from Redis stream")?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_names_are_stable() {
        assert_eq!(StreamName::RawEvents.as_str(), "raw.events");
        assert_eq!(StreamName::SignalsNormalized.as_str(), "signals.normalized");
        assert_eq!(StreamName::CopyIntents.as_str(), "copy_intents");
        assert_eq!(StreamName::ExecutionReports.as_str(), "execution.reports");
    }
}
