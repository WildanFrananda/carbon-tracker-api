use rocket::serde::json::{json, Json, Value};
use rocket::Route;
use rocket::State;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::utils::error::ApiError;
use crate::utils::jwt::Claims;
use crate::DbPool;

#[derive(Serialize)]
pub struct UserProfile {
    pub email: String,
    pub display_name: String,
    pub daily_target_kg: Decimal,
}

#[derive(Deserialize)]
pub struct UpdateTargetRequest {
    pub daily_target_kg: Decimal,
}

#[get("/profile")]
pub async fn get_profile(claims: Claims, pool: &State<DbPool>) -> Result<Value, ApiError> {
    let user = sqlx::query_as!(
        UserProfile,
        r#"
        SELECT email, display_name, daily_target_kg 
        FROM users WHERE id = $1
        "#,
        claims.sub
    )
    .fetch_optional(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to fetch user profile"))?;

    match user {
        Some(u) => Ok(json!({ "status": "success", "data": u })),
        None => Err(ApiError::bad_request("user not found")),
    }
}

#[put("/target", data = "<req>")]
pub async fn update_target(
    claims: Claims,
    req: Json<UpdateTargetRequest>,
    pool: &State<DbPool>,
) -> Result<Value, ApiError> {
    if req.daily_target_kg < Decimal::ZERO {
        return Err(ApiError::bad_request("Daily target cannot be negative"));
    }

    sqlx::query!(
        "UPDATE users SET daily_target_kg = $1 WHERE id = $2",
        req.daily_target_kg,
        claims.sub
    )
    .execute(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to update target"))?;

    return Ok(json!({
       "status": "success",
       "message": "Daily target updated successfully",
       "data": { "daily_target_kg": req.daily_target_kg }
    }));
}

pub fn routes() -> Vec<Route> {
    return routes![get_profile, update_target];
}
