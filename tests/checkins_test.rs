mod common;

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// calendar date derivation from tz
#[sqlx::test(migrations = "./migrations")]
async fn checkin_date_respects_user_timezone(db: PgPool) {
    let app = common::build_test_app(db.clone());
    let token = common::register_user(&app, "tz@test.com", "Africa/Lagos").await;

    let ts = Utc::now() - chrono::Duration::minutes(30);
    let tz: chrono_tz::Tz = "Africa/Lagos".parse().unwrap();

    let expected_date = ts.with_timezone(&tz).date_naive();

    let (status, json) = common::do_check_in(&app, &token, Some(&ts.to_rfc3339())).await;

    assert_eq!(status, 201);
    assert_eq!(
        json["check_in_date"].as_str().unwrap(),
        expected_date.to_string(),
        "Check-in date should be derived from client timestamp in user's timezone, not UTC"
    );
}

/// ensures that earliest checkin date is selected in case of duplicate
#[sqlx::test(migrations = "./migrations")]
async fn duplicate_checkin_keeps_earliest_timestamp(db: PgPool) {
    let app = common::build_test_app(db.clone());
    let token = common::register_user(&app, "dup@test.com", "UTC").await;

    let early = Utc::now() - chrono::Duration::hours(2);
    let late = Utc::now() - chrono::Duration::hours(1);

    let early_str = early.to_rfc3339();
    let late_str = late.to_rfc3339();

    // First check-in with earlier timestamp
    let (status1, json1) = common::do_check_in(&app, &token, Some(&early_str)).await;
    assert_eq!(status1, 201);
    assert_eq!(json1["already_checked_in"].as_bool().unwrap(), false);

    // Second check-in with later timestamp
    let (status2, json2) = common::do_check_in(&app, &token, Some(&late_str)).await;
    assert_eq!(status2, 200);
    assert_eq!(json2["already_checked_in"].as_bool().unwrap(), true);

    // Verify the earlier timestamp was preserved
    let row = sqlx::query!(
        "SELECT checked_in_at FROM check_ins WHERE user_id = (SELECT id FROM users WHERE email = 'dup@test.com')"
    )
    .fetch_one(&db)
    .await
    .unwrap();

    assert_eq!(
        row.checked_in_at.timestamp(),
        early.timestamp(),
        "LEAST() should preserve the earlier timestamp"
    );
}

/// Consecutive days produce correct streak count
#[sqlx::test(migrations = "./migrations")]
async fn streak_counts_consecutive_days(db: PgPool) {
    let app = common::build_test_app(db.clone());
    let token = common::register_user(&app, "streak@test.com", "UTC").await;

    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = 'streak@test.com'")
        .fetch_one(&db)
        .await
        .unwrap();

    let today = Utc::now().date_naive();

    // Seed 3 consecutive days
    for days_ago in 0..3 {
        let date = today - chrono::Duration::days(days_ago);
        sqlx::query!(
            "INSERT INTO check_ins (id, user_id, check_in_date, checked_in_at) VALUES ($1, $2, $3, NOW())",
            Uuid::new_v4(),
            user_id,
            date,
        )
        .execute(&db)
        .await
        .unwrap();
    }

    let status = common::get_status(&app, &token).await;
    assert_eq!(
        status["current_streak"].as_i64().unwrap(),
        3,
        "3 consecutive days should produce a streak of 3"
    );
}

/// Gap in dates resets streak to count only after the gap
#[sqlx::test(migrations = "./migrations")]
async fn streak_resets_on_gap(db: PgPool) {
    let app = common::build_test_app(db.clone());
    let token = common::register_user(&app, "gap@test.com", "UTC").await;

    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = 'gap@test.com'")
        .fetch_one(&db)
        .await
        .unwrap();

    let today = Utc::now().date_naive();

    // today, yesterday, then skip a day, then 3 and 4 days ago
    let dates = vec![
        today,
        today - chrono::Duration::days(1),
        // gap: day 2 is missing
        today - chrono::Duration::days(3),
        today - chrono::Duration::days(4),
    ];

    for date in dates {
        sqlx::query!(
            "INSERT INTO check_ins (id, user_id, check_in_date, checked_in_at) VALUES ($1, $2, $3, NOW())",
            Uuid::new_v4(),
            user_id,
            date,
        )
        .execute(&db)
        .await
        .unwrap();
    }

    let status = common::get_status(&app, &token).await;
    assert_eq!(
        status["current_streak"].as_i64().unwrap(),
        2,
        "Streak should be 2 (today + yesterday), not 4 the gap breaks the chain"
    );
}

/// No check-in today means streak is 0, even if there is yesterday
#[sqlx::test(migrations = "./migrations")]
async fn no_checkin_today_means_zero_streak(db: PgPool) {
    let app = common::build_test_app(db.clone());
    let token = common::register_user(&app, "zero@test.com", "UTC").await;

    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = 'zero@test.com'")
        .fetch_one(&db)
        .await
        .unwrap();

    let today = Utc::now().date_naive();

    // Seed yesterday and the day before   but NOT today
    for days_ago in 1..3 {
        let date = today - chrono::Duration::days(days_ago);
        sqlx::query!(
            "INSERT INTO check_ins (id, user_id, check_in_date, checked_in_at) VALUES ($1, $2, $3, NOW())",
            Uuid::new_v4(),
            user_id,
            date,
        )
        .execute(&db)
        .await
        .unwrap();
    }

    let status = common::get_status(&app, &token).await;
    assert_eq!(
        status["current_streak"].as_i64().unwrap(),
        0,
        "No check-in today should mean streak is 0, regardless of past check-ins"
    );
}

/// Test 7: User A's check-ins do not affect User B's streak.
#[sqlx::test(migrations = "./migrations")]
async fn streak_is_isolated_per_user(db: PgPool) {
    let app = common::build_test_app(db.clone());
    let token_a = common::register_user(&app, "a@test.com", "UTC").await;
    let token_b = common::register_user(&app, "b@test.com", "UTC").await;

    let user_a_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = 'a@test.com'")
        .fetch_one(&db)
        .await
        .unwrap();
    let user_b_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = 'b@test.com'")
        .fetch_one(&db)
        .await
        .unwrap();

    let today = Utc::now().date_naive();

    // User A: 5 consecutive days
    for days_ago in 0..5 {
        let date = today - chrono::Duration::days(days_ago);
        sqlx::query!(
            "INSERT INTO check_ins (id, user_id, check_in_date, checked_in_at) VALUES ($1, $2, $3, NOW())",
            Uuid::new_v4(),
            user_a_id,
            date,
        )
        .execute(&db)
        .await
        .unwrap();
    }

    // User B: 2 consecutive days
    for days_ago in 0..2 {
        let date = today - chrono::Duration::days(days_ago);
        sqlx::query!(
            "INSERT INTO check_ins (id, user_id, check_in_date, checked_in_at) VALUES ($1, $2, $3, NOW())",
            Uuid::new_v4(),
            user_b_id,
            date,
        )
        .execute(&db)
        .await
        .unwrap();
    }

    let status_a = common::get_status(&app, &token_a).await;
    let status_b = common::get_status(&app, &token_b).await;

    assert_eq!(status_a["current_streak"].as_i64().unwrap(), 5);
    assert_eq!(
        status_b["current_streak"].as_i64().unwrap(),
        2,
        "User B's streak must be 2, not polluted by User A's 5"
    );
}
