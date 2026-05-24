use anyhow::Result;
use arbitek_config::AppConfig;
use arbitek_db::{
    connect_worker as connect, CopyIntentRepository, FollowerRepository, MasterRepository,
    RawEventRepository, SignalRepository,
};
use arbitek_domain::market_mapping::{default_binary_mappings, MarketMapper};
use arbitek_queue::{MessageEnvelope, RedisStreamConsumer, StreamName};
use serde_json::Value;
use tracing::{error, info, warn};

/// Run the queue consumer worker that reads from Redis streams and triggers
/// downstream processing for each event.
pub async fn run_queue_consume(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let raw_repo = RawEventRepository::new(pool.clone());
    let signal_repo = SignalRepository::new(pool.clone());
    let follower_repo = FollowerRepository::new(pool.clone());
    let master_repo = MasterRepository::new(pool.clone());
    let copy_repo = CopyIntentRepository::new(pool.clone());
    let mapper = MarketMapper::new(default_binary_mappings());

    let consumer =
        RedisStreamConsumer::connect(&config.redis_url, "arbitek_workers", "queue_consumer")
            .await?;

    let streams = vec![
        StreamName::RawEvents,
        StreamName::SignalsNormalized,
        StreamName::CopyIntents,
        StreamName::ExecutionReports,
    ];

    consumer.create_groups(&streams).await?;
    info!("queue consumer started, listening on 4 streams");

    let mut processed = 0u64;
    let mut failed = 0u64;

    loop {
        match consumer.read(&streams, 10, 5_000).await {
            Ok(batches) => {
                if batches.is_empty() {
                    continue;
                }

                for (stream, envelopes) in batches {
                    for (_redis_id, envelope) in &envelopes {
                        let result = match stream {
                            StreamName::RawEvents => {
                                handle_raw_event(
                                    envelope,
                                    &raw_repo,
                                    &signal_repo,
                                    &follower_repo,
                                    &master_repo,
                                    &copy_repo,
                                    &mapper,
                                    config,
                                )
                                .await
                            }
                            StreamName::SignalsNormalized => {
                                handle_signal_normalized(envelope, &signal_repo).await
                            }
                            StreamName::CopyIntents => {
                                handle_copy_intent(envelope, &copy_repo).await
                            }
                            StreamName::ExecutionReports => handle_execution_report(envelope).await,
                        };

                        if let Err(e) = &result {
                            warn!(stream = %stream.as_str(), event_type = %envelope.event_type, error = %e, "failed to handle queue event");
                            failed += 1;
                        } else {
                            processed += 1;
                        }
                    }

                    // Ack all messages in this batch for the stream
                    let ids: Vec<&str> = envelopes.iter().map(|(id, _)| id.as_str()).collect();
                    if let Err(e) = consumer.ack(stream, &ids).await {
                        warn!(stream = %stream.as_str(), error = %e, "failed to ack messages");
                    }
                }

                if processed % 100 == 0 && processed > 0 {
                    info!(processed, failed, "queue consumer progress");
                }
            }
            Err(e) => {
                error!(error = %e, "redis stream read failed");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}

async fn handle_raw_event(
    envelope: &MessageEnvelope,
    raw_repo: &RawEventRepository,
    signal_repo: &SignalRepository,
    follower_repo: &FollowerRepository,
    master_repo: &MasterRepository,
    copy_repo: &CopyIntentRepository,
    mapper: &MarketMapper,
    config: &AppConfig,
) -> Result<()> {
    let platform = envelope
        .payload
        .get("platform")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let external_event_id = envelope
        .payload
        .get("external_event_id")
        .and_then(Value::as_str)
        .unwrap_or("");

    if external_event_id.is_empty() {
        return Ok(());
    }

    let event = match raw_repo
        .get_by_external_event_id(platform, external_event_id)
        .await?
    {
        Some(e) => e,
        None => {
            warn!(platform, external_event_id, "raw event not found in db");
            return Ok(());
        }
    };

    if event.status != "new" {
        info!(event_id = %event.id, "raw event already processed, skipping");
        return Ok(());
    }

    match crate::signal_normalize::normalize_single_event(
        &event,
        mapper,
        signal_repo,
        follower_repo,
        master_repo,
        copy_repo,
        config,
    )
    .await
    {
        Ok(Some(signal_id)) => {
            raw_repo.mark_processed(event.id).await.ok();
            info!(event_id = %event.id, signal_id = %signal_id, "normalized raw event from queue");
        }
        Ok(None) => {
            raw_repo.mark_processed(event.id).await.ok();
            info!(event_id = %event.id, "skipped raw event from queue (no mapping)");
        }
        Err(e) => {
            warn!(event_id = %event.id, error = %e, "failed to normalize raw event from queue");
        }
    }

    Ok(())
}

async fn handle_signal_normalized(
    _envelope: &MessageEnvelope,
    _signal_repo: &SignalRepository,
) -> Result<()> {
    // TODO: trigger risk evaluation for the specific signal
    // For now the risk-evaluate worker polls pending copy intents.
    info!("signal normalized event received — risk evaluate will pick it up via polling");
    Ok(())
}

async fn handle_copy_intent(
    _envelope: &MessageEnvelope,
    _copy_repo: &CopyIntentRepository,
) -> Result<()> {
    // TODO: trigger execution for the specific intent
    // For now the execution-live worker polls approved copy intents.
    info!("copy intent event received — execution-live will pick it up via polling");
    Ok(())
}

async fn handle_execution_report(_envelope: &MessageEnvelope) -> Result<()> {
    // TODO: trigger reconciliation for the specific order
    // For now the reconcile worker polls platform order status.
    info!("execution report event received — reconcile will pick it up via polling");
    Ok(())
}
