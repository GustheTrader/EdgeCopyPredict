use anyhow::Result;
use arbitek_config::AppConfig;
use arbitek_queue::{MessageEnvelope, RedisStreamPublisher, StreamName};
use serde_json::json;
use tracing::info;

/// Publish a signal normalized event to Redis.
pub async fn publish_signal(config: &AppConfig, signal_id: uuid::Uuid) -> Result<()> {
    let publisher = RedisStreamPublisher::connect(&config.redis_url).await?;
    let envelope = MessageEnvelope::new(
        "signal.normalized",
        json!({"signal_id": signal_id.to_string()}),
    );
    publisher
        .publish(StreamName::SignalsNormalized, &envelope)
        .await?;
    info!(signal_id = %signal_id, "published signal normalized event");
    Ok(())
}

/// Publish a copy intent created event to Redis.
pub async fn publish_copy_intent(config: &AppConfig, intent_id: uuid::Uuid) -> Result<()> {
    let publisher = RedisStreamPublisher::connect(&config.redis_url).await?;
    let envelope = MessageEnvelope::new(
        "copy_intent.created",
        json!({"intent_id": intent_id.to_string()}),
    );
    publisher
        .publish(StreamName::CopyIntents, &envelope)
        .await?;
    info!(intent_id = %intent_id, "published copy intent event");
    Ok(())
}

/// Publish an execution report to Redis.
pub async fn publish_execution(
    config: &AppConfig,
    order_id: uuid::Uuid,
    status: &str,
) -> Result<()> {
    let publisher = RedisStreamPublisher::connect(&config.redis_url).await?;
    let envelope = MessageEnvelope::new(
        "execution.report",
        json!({
            "order_id": order_id.to_string(),
            "status": status,
        }),
    );
    publisher
        .publish(StreamName::ExecutionReports, &envelope)
        .await?;
    info!(order_id = %order_id, status = %status, "published execution report");
    Ok(())
}

/// Publish a raw event to Redis.
pub async fn publish_raw_event(
    config: &AppConfig,
    platform: &str,
    external_id: &str,
) -> Result<()> {
    let publisher = RedisStreamPublisher::connect(&config.redis_url).await?;
    let envelope = MessageEnvelope::new(
        "raw_event.ingested",
        json!({
            "platform": platform,
            "external_event_id": external_id,
        }),
    );
    publisher.publish(StreamName::RawEvents, &envelope).await?;
    info!(platform = %platform, external_id = %external_id, "published raw event");
    Ok(())
}
