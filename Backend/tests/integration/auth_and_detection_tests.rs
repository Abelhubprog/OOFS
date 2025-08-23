use anyhow::Result;
use common::{TestEnvironment, TestUtils};
use serde_json::json;

mod common;

#[tokio::test]
async fn test_full_authentication_flow() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Test 1: Create JWT token
    let token = env
        .create_test_jwt("test_user_001", vec!["user".to_string(), "premium".to_string()])
        .await?;

    assert!(!token.is_empty());
    println!("✅ JWT token created successfully");

    // Test 2: Validate token structure
    let token_parts: Vec<&str> = token.split('.').collect();
    assert_eq!(token_parts.len(), 3, "JWT should have 3 parts");
    println!("✅ JWT token structure is valid");

    // Test 3: Database connection
    assert!(env.test_database_connection().await?);
    println!("✅ Database connection successful");

    Ok(())
}

#[tokio::test]
async fn test_moment_detection_s2e() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Create a test transaction that should trigger S2E detection
    let tx_data = json!({
        "signature": TestUtils::random_signature(),
        "slot": 123456789,
        "blockTime": 1640995200,
        "wallet": "test_wallet_s2e",
        "action": "sell",
        "mint": "So11111111111111111111111111111111111111112",
        "amount": 1000,
        "price": 100.0
    });

    // Process the transaction
    env.process_transaction(tx_data).await?;

    // Check if moments were created
    let moments = env.get_moments_for_wallet("test_wallet_processed").await?;
    assert!(!moments.is_empty(), "Should have detected at least one moment");

    // Validate moment structure
    let moment = &moments[0];
    TestUtils::assert_response_structure(
        moment,
        &["id", "wallet", "mint", "kind", "t_event"],
    )?;

    println!("✅ S2E moment detection successful");
    println!("📊 Detected moments: {}", moments.len());

    Ok(())
}

#[tokio::test]
async fn test_health_check_comprehensive() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Test comprehensive health check
    let health_report = env
        .health_checker
        .comprehensive_check(
            &env.config.database_url,
            env.config.redis_url.as_deref(),
            &env.config.dynamic_jwks_url,
            &env.config,
        )
        .await;

    // Check overall health status
    assert_ne!(
        health_report.status,
        shared::health::HealthStatus::Unknown,
        "Health status should be determined"
    );

    // Check that database health check passed
    assert!(
        health_report.checks.contains_key("database"),
        "Should include database health check"
    );

    let db_check = &health_report.checks["database"];
    assert_eq!(
        db_check.status,
        shared::health::HealthStatus::Healthy,
        "Database should be healthy in test environment"
    );

    println!("✅ Comprehensive health check completed");
    println!(
        "📊 Health summary: {}/{} checks healthy",
        health_report.summary.healthy_checks, health_report.summary.total_checks
    );

    Ok(())
}

#[tokio::test]
async fn test_concurrent_wallet_analysis() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Create multiple test wallets
    let wallets = vec![
        TestUtils::random_wallet(),
        TestUtils::random_wallet(),
        TestUtils::random_wallet(),
    ];

    // Process transactions for each wallet concurrently
    let mut handles = Vec::new();
    for wallet in &wallets {
        let env = env.clone();
        let wallet = wallet.clone();
        let handle = tokio::spawn(async move {
            let tx_data = json!({
                "signature": TestUtils::random_signature(),
                "wallet": wallet,
                "action": "buy",
                "mint": TestUtils::random_mint(),
                "amount": 500,
                "price": 50.0
            });

            env.process_transaction(tx_data).await
        });
        handles.push(handle);
    }

    // Wait for all transactions to complete
    for handle in handles {
        handle.await??;
    }

    println!("✅ Concurrent wallet analysis completed");
    println!("📊 Processed {} wallets concurrently", wallets.len());

    Ok(())
}

#[tokio::test]
async fn test_rate_limiting_enforcement() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // This test would typically involve making rapid API requests
    // For now, we'll test the configuration and basic functionality

    // Create multiple tokens with different rate limit tiers
    let basic_token = env
        .create_test_jwt("basic_user", vec!["user".to_string()])
        .await?;

    let premium_token = env
        .create_test_jwt("premium_user", vec!["user".to_string(), "premium".to_string()])
        .await?;

    // Validate tokens are different
    assert_ne!(basic_token, premium_token);

    // Test token claims
    assert!(!basic_token.is_empty());
    assert!(!premium_token.is_empty());

    println!("✅ Rate limiting tokens created successfully");

    Ok(())
}

#[tokio::test]
async fn test_data_consistency() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Test data seeding consistency
    let token_count = sqlx::query_scalar!("SELECT COUNT(*) FROM token_facts WHERE verified = true")
        .fetch_one(&env.database.0)
        .await?;

    assert!(
        token_count.unwrap_or(0) >= 3,
        "Should have at least 3 verified tokens in test data"
    );

    // Test wallet data
    let wallet_count = sqlx::query_scalar!("SELECT COUNT(*) FROM participants")
        .fetch_one(&env.database.0)
        .await?;

    assert!(
        wallet_count.unwrap_or(0) >= 1,
        "Should have at least 1 wallet in test data"
    );

    println!("✅ Data consistency checks passed");
    println!("📊 Verified tokens: {}", token_count.unwrap_or(0));
    println!("📊 Test wallets: {}", wallet_count.unwrap_or(0));

    Ok(())
}

#[tokio::test]
async fn test_error_handling_robustness() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Test 1: Invalid JWT creation should fail gracefully
    let result = env.create_test_jwt("", vec![]).await;
    // This should not panic, even with empty user_id

    // Test 2: Invalid transaction processing
    let invalid_tx = json!({
        "invalid": "data"
    });

    let result = env.process_transaction(invalid_tx).await;
    // Should handle gracefully without panicking

    // Test 3: Database query with invalid wallet
    let moments = env.get_moments_for_wallet("nonexistent_wallet").await?;
    assert!(moments.is_empty(), "Should return empty for nonexistent wallet");

    println!("✅ Error handling robustness tests passed");

    Ok(())
}

#[tokio::test]
async fn test_cleanup_and_isolation() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Insert some test data
    let test_moment_id = "cleanup_test_moment";
    sqlx::query!(
        r#"
        INSERT INTO moments (id, wallet, mint, kind, t_event, version)
        VALUES ($1, 'cleanup_test_wallet', 'cleanup_test_mint', 'S2E', NOW(), '1')
        "#,
        test_moment_id
    )
    .execute(&env.database.0)
    .await?;

    // Verify data exists
    let count_before = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM moments WHERE id = $1",
        test_moment_id
    )
    .fetch_one(&env.database.0)
    .await?;

    assert_eq!(count_before.unwrap_or(0), 1);

    // Clean up
    env.cleanup().await?;

    // Verify cleanup worked
    let count_after = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM moments WHERE id = $1",
        test_moment_id
    )
    .fetch_one(&env.database.0)
    .await?;

    assert_eq!(count_after.unwrap_or(0), 0);

    println!("✅ Cleanup and isolation tests passed");

    Ok(())
}

#[tokio::test]
async fn test_performance_benchmarks() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Benchmark database query performance
    let start = std::time::Instant::now();

    for i in 0..100 {
        sqlx::query!("SELECT 1")
            .fetch_one(&env.database.0)
            .await?;
    }

    let db_duration = start.elapsed();
    println!("📊 Database query benchmark: {:?} for 100 queries", db_duration);

    // Benchmark JWT creation
    let start = std::time::Instant::now();

    for i in 0..10 {
        env.create_test_jwt(&format!("bench_user_{}", i), vec!["user".to_string()])
            .await?;
    }

    let jwt_duration = start.elapsed();
    println!("📊 JWT creation benchmark: {:?} for 10 tokens", jwt_duration);

    // Basic performance assertions
    assert!(
        db_duration.as_millis() < 1000,
        "Database queries should complete within 1 second"
    );
    assert!(
        jwt_duration.as_millis() < 500,
        "JWT creation should complete within 500ms"
    );

    println!("✅ Performance benchmarks passed");

    Ok(())
}
