use rocket::serde::json::{json, Value};
use rocket::{Route, State};
use rust_decimal::Decimal;

use crate::services::ai_service::get_ai_recommendation;
use crate::utils::error::ApiError;
use crate::utils::jwt::Claims;
use crate::DbPool;

struct CategorySums {
    total_transport: Option<Decimal>,
    total_food: Option<Decimal>,
    total_energy: Option<Decimal>,
}

#[get("/recommendations")]
pub async fn get_recommendations(claims: Claims, pool: &State<DbPool>) -> Result<Value, ApiError> {
    let sums = sqlx::query_as!(
        CategorySums,
        r#"
        SELECT 
            SUM(transport_kg) as total_transport,
            SUM(food_kg) as total_food,
            SUM(energy_kg) as total_energy
        FROM daily_summaries
        WHERE user_id = $1 AND date >= CURRENT_DATE - INTERVAL '7 days'
        "#,
        claims.sub
    )
    .fetch_one(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to fetch insights"))?;

    let transport = sums.total_transport.unwrap_or_default();
    let food = sums.total_food.unwrap_or_default();
    let energy = sums.total_energy.unwrap_or_default();

    let user_data = format!(
        "Transport: {}kg CO2, Food: {}kg CO2, Energy: {}kg CO2",
        transport, food, energy
    );

    let ai_message = match get_ai_recommendation(user_data).await {
        Ok(msg) => msg,
        Err(_) => "Keep up the good work in reducing your carbon footprint!".to_string(),
    };

    return Ok(json!({
        "status": "success",
        "data": {
            "dominant_category": if transport > food && transport > energy { "Transportation" } else if food > energy { "Food" } else { "Energy" },
            "message": "AI Analysis for your activity this week:",
            "ai_insight": ai_message
        }
    }));
}

pub fn routes() -> Vec<Route> {
    return routes![get_recommendations];
}
