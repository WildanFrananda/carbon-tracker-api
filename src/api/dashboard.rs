use chrono::{Duration, NaiveDate};
use rocket::serde::json::{json, Value};
use rocket::{Route, State};
use rust_decimal::Decimal;

use crate::models::dashboard::{DailySummaryResponse, HeatmapData, WeeklySummaryResponse};
use crate::utils::error::ApiError;
use crate::utils::jwt::Claims;
use crate::DbPool;

#[get("/daily?<date>")]
pub async fn get_daily_summary(
    claims: Claims,
    date: &str,
    pool: &State<DbPool>,
) -> Result<Value, ApiError> {
    let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Date Format invalid, use YYYY-MM-DD"))?;

    let summary = sqlx::query_as!(
        DailySummaryResponse,
        r#"
        SELECT date, total_emission, transport_kg, food_kg, energy_kg, shopping_kg, is_green_day
        FROM daily_summaries
        WHERE user_id = $1 AND date = $2
        "#,
        claims.sub,
        parsed_date
    )
    .fetch_optional(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to fetch daily summary"))?;

    let response_data = summary.unwrap_or_else(|| DailySummaryResponse {
        date: parsed_date,
        total_emission: Decimal::ZERO,
        transport_kg: Decimal::ZERO,
        food_kg: Decimal::ZERO,
        energy_kg: Decimal::ZERO,
        shopping_kg: Decimal::ZERO,
        is_green_day: true,
    });

    return Ok(json!({
        "status": "success",
        "data": response_data
    }));
}

#[get("/weekly?<end_date>")]
pub async fn get_weekly_summary(
    claims: Claims,
    end_date: &str,
    pool: &State<DbPool>,
) -> Result<Value, ApiError> {
    let parsed_end_date = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Date Format invalid"))?;
    let parsed_start_date = parsed_end_date - Duration::days(6);

    struct WeeklyAggr {
        total: Option<Decimal>,
        days: Option<i64>,
    }

    let summary = sqlx::query_as!(
        WeeklyAggr,
        r#"
        SELECT 
            SUM(total_emission) as "total",
            COUNT(date) as "days"
        FROM daily_summaries 
        WHERE user_id = $1 AND date >= $2 AND date <= $3
        "#,
        claims.sub,
        parsed_start_date,
        parsed_end_date
    )
    .fetch_one(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to fetch weekly summary"))?;

    let total_emission = summary.total.unwrap_or_default();
    let days_logged = summary.days.unwrap_or(0);

    let average_daily_emission = if days_logged > 0 {
        total_emission / rust_decimal::Decimal::from(days_logged)
    } else {
        rust_decimal::Decimal::ZERO
    };

    return Ok(json!({
        "status": "success",
        "data": WeeklySummaryResponse {
            start_date: parsed_start_date,
            end_date: parsed_end_date,
            total_emission,
            average_daily_emission,
            days_logged
        }
    }));
}

#[get("/heatmap?<year>")]
pub async fn get_heapmap(
    claims: Claims,
    year: i32,
    pool: &State<DbPool>,
) -> Result<Value, ApiError> {
    let start_date =
        NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(|| ApiError::bad_request("Year invalid"))?;
    let end_date = NaiveDate::from_ymd_opt(year, 12, 31)
        .ok_or_else(|| ApiError::bad_request("Year invalid"))?;

    let heatmap_data = sqlx::query_as!(
        HeatmapData,
        r#"
        SELECT date, total_emission
        FROM daily_summaries
        WHERE user_id = $1 AND date >= $2 AND date <= $3
        ORDER BY date ASC
        "#,
        claims.sub,
        start_date,
        end_date
    )
    .fetch_all(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Gagal mengambil data heatmap"))?;

    return Ok(json!({
        "status": "success",
        "data": heatmap_data
    }));
}

pub fn routes() -> Vec<Route> {
    return routes![get_daily_summary, get_weekly_summary, get_heapmap];
}
