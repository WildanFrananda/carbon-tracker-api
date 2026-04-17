use rocket::serde::json::{json, Value};
use rocket::{Route, State};
use rust_decimal::Decimal;

use crate::models::dashboard::RecommendationResponse;
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

    let mut response = RecommendationResponse {
        dominant_category: "General".to_string(),
        message: "Your carbon activity is balanced this week. Keep it up!".to_string(),
        actionable_tips: vec![
            "Try turning off lights when not in use.".to_string(),
            "Reduce the use of single-use plastics.".to_string(),
        ],
    };

    if transport >= food && transport >= energy && transport > Decimal::ZERO {
        response.dominant_category = "Transportation".to_string();
        response.message = "Your biggest emission this week comes from transportation.".to_string();
        response.actionable_tips = vec![
            "Consider using public transportation 1-2 times next week.".to_string(),
            "If possible, try carpooling with office friends.".to_string(),
            "Regular vehicle maintenance can save up to 4% fuel.".to_string(),
        ];
    } else if food >= transport && food >= energy && food > Decimal::ZERO {
        response.dominant_category = "Food".to_string();
        response.message =
            "Your carbon footprint is dominated by certain food consumption.".to_string();
        response.actionable_tips = vec![
            "Try implementing 'Meatless Monday'.".to_string(),
            "Finish your food to reduce food waste carbon footprint.".to_string(),
        ];
    }

    return Ok(json!({
        "status": "success",
        "data": response
    }));
}

pub fn routes() -> Vec<Route> {
    return routes![get_recommendations];
}
