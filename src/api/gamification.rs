use chrono::{DateTime, Utc};
use rocket::serde::json::{json, Value};
use rocket::{Route, State};
use serde::Serialize;

use crate::utils::error::ApiError;
use crate::utils::jwt::Claims;
use crate::DbPool;

#[derive(Serialize)]
pub struct BadgeResponse {
    pub badge_type: String,
    pub earned_at: DateTime<Utc>,
    pub description: String,
}

#[get("/badges")]
pub async fn get_badges(claims: Claims, pool: &State<DbPool>) -> Result<Value, ApiError> {
    struct AchievementRecord {
        badge_type: String,
        earned_at: DateTime<Utc>,
    }

    let records = sqlx::query_as!(
        AchievementRecord,
        r#"
        SELECT badge_type, earned_at 
        FROM achievements 
        WHERE user_id = $1
        ORDER BY earned_at DESC
        "#,
        claims.sub
    )
    .fetch_all(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to fetch achievements"))?;

    let badges: Vec<BadgeResponse> = records
        .into_iter()
        .map(|rec| {
            let desc = match rec.badge_type.as_str() {
                "FIRST_LOG" => "Completing the first activity log.",
                "3_DAYS_STREAK" => "Logging activities for 3 consecutive days.",
                "7_DAYS_STREAK" => "Logging activities for 7 consecutive days.",
                "VEGGIE_HERO" => "Low-emission plant-based food consumption.",
                "LOW_CARBON_COMMUTER" => "Low-carbon transportation.",
                _ => "Special badge.",
            };

            BadgeResponse {
                badge_type: rec.badge_type,
                earned_at: rec.earned_at,
                description: desc.to_string(),
            }
        })
        .collect();

    return Ok(json!({
        "status": "success",
        "data": badges
    }));
}

pub fn routes() -> Vec<Route> {
    return routes![get_badges];
}
