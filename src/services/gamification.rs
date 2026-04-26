use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::activity::Category;

#[derive(Debug, Clone, PartialEq)]
pub enum BadgeType {
    FirstLog,
    ThreeDaysStreak,
    SevenDaysStreak,
    VeggieHero,
    LowCarbonCommuter,
}

impl BadgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BadgeType::FirstLog => "FIRST_LOG",
            BadgeType::ThreeDaysStreak => "3_DAYS_STREAK",
            BadgeType::SevenDaysStreak => "7_DAYS_STREAK",
            BadgeType::VeggieHero => "VEGGIE_HERO",
            BadgeType::LowCarbonCommuter => "LOW_CARBON_COMMUTER",
        }
    }
}

pub async fn evaluate_achivements(
    pool: &PgPool,
    user_id: Uuid,
    date: NaiveDate,
) -> Result<(), String> {
    let has_first_log = check_badge_exists(pool, user_id, BadgeType::FirstLog).await?;
    if !has_first_log {
        grand_badge(pool, user_id, BadgeType::FirstLog).await?;
    }

    evaluate_streak(pool, user_id, date).await?;
    evaluate_category_badges(pool, user_id, date).await?;

    return Ok(());
}

async fn check_badge_exists(
    pool: &PgPool,
    user_id: Uuid,
    badge_type: BadgeType,
) -> Result<bool, String> {
    let result = sqlx::query!(
        "SELECT 1 as exists FROM achievements WHERE user_id = $1 AND badge_type = $2",
        user_id,
        badge_type.as_str()
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| "Failed to check badge existence".to_string())?;

    return Ok(result.is_some());
}

async fn grant_badge(pool: &PgPool, user_id: Uuid, badge_type: BadgeType) -> Result<(), String> {
    sqlx::query!(
        "INSERT INTO achievements (user_id, badge_type) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        user_id,
        badge_type.as_str()
    )
    .execute(pool)
    .await
    .map_err(|_| "Failed to grant badge".to_string())?;

    println!("User {} earned badge: {}", user_id, badge_type.as_str());
    return Ok();
}

async fn evaluate_streak(
    pool: &PgPool,
    user_id: Uuid,
    current_date: NaiveDate,
) -> Result<(), String> {
    let start_date = current_date - Duration::days(6);

    let logged_dates = sqlx::query!(
        "SELECT date FROM daily_summaries WHERE user_id = $1 AND date >= $2 AND date <= $3 ORDER BY date DESC",
        user_id,
        start_date,
        current_date
    )
    .fetch_all(pool)
    .await
    .map_err(|_| "Failed to fetch streak data".to_string())?;

    let mut current_streak = 0;
    let mut check_date = current_date;

    for record in &logged_dates {
        if record.date == check_date {
            current_streak += 1;
            check_date -= Duration::days(1);
        } else {
            break;
        }
    }

    if current_streak >= 3 && !check_badge_exists(pool, user_id, BadgeType::ThreeDaysStreak).await?
    {
        grant_badge(pool, user_id, BadgeType::ThreeDaysStreak).await?;
    }

    if current_streak >= 7 && !check_badge_exists(pool, user_id, BadgeType::SevenDaysStreak).await?
    {
        grant_badge(pool, user_id, BadgeType::SevenDaysStreak).await?;
    }

    return Ok(());
}

async fn evaluate_category_badges(
    pool: &PgPool,
    user_id: Uuid,
    date: NaiveDate,
) -> Result<(), String> {
    let summary = sqlx::query!(
        "SELECT food_kg, transport_kg FROM daily_summaries WHERE user_id = $1 AND date = $2",
        user_id,
        date
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| "Failed to fetch category data".to_string())?;

    if let Some(sum) = summary {
        let threshold_veggie = Decimal::new(20, 1);
        if sum.food_kg > Decimal::ZERO && sum.food_kg < threshold_veggie {
            if !check_badge_exists(pool, user_id, BadgeType::VeggieHero).await? {
                grant_badge(pool, user_id, BadgeType::VeggieHero).await?;
            }
        }

        let threshold_transport = Decimal::new(10, 1);
        if sum.transport_kg > Decimal::ZERO && sum.transport_kg < threshold_transport {
            if !check_badge_exists(pool, user_id, BadgeType::LowCarbonCommuter).await? {
                grant_badge(pool, user_id, BadgeType::LowCarbonCommuter).await?;
            }
        }
    }

    return Ok(());
}

async fn evaluate_category_badges(
    pool: &PgPool,
    user_id: Uuid,
    date: NaiveDate,
) -> Result<(), String> {
    let summary = sqlx::query!(
        "SELECT food_kg, transport_kg FROM daily_summaries WHERE user_id = $1 AND date = $2",
        user_id,
        date
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| "Failed to fetch summary".to_string());

    if let Some(sum) = summary {
        let threshold_vaggie = Decimal::new(20, 1);
        if sum.food_kg > Decimal::ZERO && sum.food_kg < threshold_vaggie {
            if !check_badge_exists(pool, user_id, BadgeType::VeggieHero).await? {
                grant_badge(pool, user_id, BadgeType::VeggieHero).await?;
            }
        }

        let threshold_transport = Decimal::new(10, 1); // 1.0
        if sum.transport_kg > Decimal::ZERO && sum.transport_kg < threshold_transport {
            if !check_badge_exists(pool, user_id, BadgeType::LowCarbonCommuter).await? {
                grant_badge(pool, user_id, BadgeType::LowCarbonCommuter).await?;
            }
        }
    }

    return Ok(());
}
