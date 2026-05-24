use std::time::Duration;
use tokio::time::timeout;

/// End-to-end pipeline integration test.
///
/// Requires a running Postgres (&DATABASE_URL) and Redis (&REDIS_URL).
/// Use docker-compose up db redis to spin them up before running tests.
#[tokio::test]
async fn test_full_pipeline() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/arbitek".to_owned());
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_owned());

    // Seed the database with a master and follower
    seed_db(&database_url).await;

    // Use a unique group name per run so we never collide with a leftover group
    // from a previous test run that already consumed the messages.
    let group_name = format!("test_group_{}", uuid::Uuid::new_v4().simple());

    // 1. Connect publisher and consumer.
    let publisher = arbitek_queue::RedisStreamPublisher::connect(&redis_url)
        .await
        .expect("connect redis publisher");

    let consumer =
        arbitek_queue::RedisStreamConsumer::connect(&redis_url, &group_name, "test_consumer")
            .await
            .expect("connect redis consumer");

    // 2. Create the consumer group BEFORE publishing so the group offset starts
    //    at the current tail and will see the message we publish next.
    consumer
        .create_groups(&[arbitek_queue::StreamName::RawEvents])
        .await
        .expect("create consumer group");

    // 3. Publish a raw event.
    let envelope = arbitek_queue::MessageEnvelope::new(
        "raw_event.ingested",
        serde_json::json!({
            "platform": "polymarket",
            "external_event_id": "0xabc:123:BUY:YES",
        }),
    );
    publisher
        .publish(arbitek_queue::StreamName::RawEvents, &envelope)
        .await
        .expect("publish raw event");

    // 4. Read — the group was created before the publish, so ">" delivers it.
    let batches = timeout(
        Duration::from_secs(5),
        consumer.read(&[arbitek_queue::StreamName::RawEvents], 1, 2_000),
    )
    .await
    .expect("read within timeout")
    .expect("read succeeded");

    assert!(!batches.is_empty(), "should receive at least one batch");
    let (stream, messages) = &batches[0];
    assert_eq!(*stream, arbitek_queue::StreamName::RawEvents);
    assert!(!messages.is_empty(), "should receive at least one message");

    println!("integration test passed: Redis stream round-trip works");
}

async fn seed_db(database_url: &str) {
    let pool = sqlx::PgPool::connect(database_url)
        .await
        .expect("connect to postgres");

    // Insert a polymarket master with wallet
    let master_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"INSERT INTO masters (display_name, status, master_weight, wallet_address)
           VALUES ('Test Master', 'active', 1.0, '0xTestWallet')
           ON CONFLICT DO NOTHING
           RETURNING id"#,
    )
    .fetch_optional(&pool)
    .await
    .expect("insert master")
    .unwrap_or(
        sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT id FROM masters WHERE display_name = 'Test Master'",
        )
        .fetch_one(&pool)
        .await
        .expect("fetch master"),
    );

    // Insert a polymarket follower
    sqlx::query(
        r#"INSERT INTO followers (display_name, status, base_currency, platform, risk_config)
           VALUES ('Test Follower', 'active', 'USD', 'polymarket', '{}')
           ON CONFLICT DO NOTHING"#,
    )
    .execute(&pool)
    .await
    .expect("insert follower");

    // Insert market mapping
    sqlx::query(
        r#"INSERT INTO market_mappings (polymarket_condition_id, kalshi_ticker, confidence, enabled)
           VALUES ('0xcondition123', 'BTC-YES', 'high', true)
           ON CONFLICT DO NOTHING"#,
    )
    .execute(&pool)
    .await
    .expect("insert market mapping");

    println!("seeded db with master_id={}", master_id);
}
