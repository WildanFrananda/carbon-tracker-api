use chrono::NaiveDate;

use rocket::serde::json::{json, Json, Value};
use rocket::{Route, State};
use uuid::Uuid;

use crate::engine::calculator::calculate_emission;
use crate::engine::factors::get_emission_factor;
use crate::models::activity::Category;
use crate::models::activity::{ActivityRequest, ActivityResponse};
use crate::services::gamification::evaluate_achivements;
use crate::utils::error::ApiError;
use crate::utils::jwt::Claims;
use crate::DbPool;

#[post("/", data = "<req>")]
pub async fn log_activity(
    claims: Claims,
    req: Json<ActivityRequest>,
    pool: &State<DbPool>,
) -> Result<Value, ApiError> {
    let user_id = claims.sub;
    let factor = get_emission_factor(&pool.0, &req.category, &req.subcategory)
        .await
        .map_err(|_| ApiError::internal("Database Error while searching emission factor"))?;

    let factor = match factor {
        Some(f) => f,
        None => {
            return Err(ApiError::bad_request(&format!(
                "Emission factor not found for category '{}' and subcategory '{}'. Please check your input.",
                req.category.as_str(),
                req.subcategory
            )))
        }
    };

    let calculated_emission = calculate_emission(req.quantity, factor.factor_value);

    let mut tx = pool
        .0
        .begin()
        .await
        .map_err(|_| ApiError::internal("Database Error while starting transaction"))?;

    let cat_str = req.category.as_str();

    let activity_id = sqlx::query_scalar!(
        r#"
        INSERT INTO activities (user_id, category, subcategory, quantity, unit, calculated_emission_kg, date)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
        user_id,
        cat_str,
        req.subcategory,
        req.quantity,
        factor.unit,
        calculated_emission,
        req.date
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::internal("Gagal menyimpan aktivitas"))?;

    let update_query = format!(
        r#"
        INSERT INTO daily_summaries (user_id, date, total_emission, {0}_kg)
        VALUES ($1, $2, $3, $3)
        ON CONFLICT (user_id, date)
        DO UPDATE SET 
            total_emission = daily_summaries.total_emission + EXCLUDED.total_emission,
            {0}_kg = daily_summaries.{0}_kg + EXCLUDED.{0}_kg
        "#,
        cat_str
    );

    sqlx::query(&update_query)
        .bind(user_id)
        .bind(req.date)
        .bind(calculated_emission)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            println!("Error updating summary: {:?}", e);
            ApiError::internal("Failed to update daily summary")
        })?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Failed to commit transaction"))?;

    let _ = evaluate_achivements(&pool.0, user_id, req.date).await;

    return Ok(json!({
        "status": "success",
        "message": "Activity successfully logged",
        "data": ActivityResponse {
            id: activity_id,
            category: req.category.clone(),
            subcategory: req.subcategory.clone(),
            quantity: req.quantity,
            unit: factor.unit,
            calculated_emission_kg: calculated_emission,
            date: req.date,
        }
    }));
}

#[get("/?<date>")]
pub async fn get_activities(
    claims: Claims,
    date: &str,
    pool: &State<DbPool>,
) -> Result<Value, ApiError> {
    let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Date Format invalid"))?;
    let activities = sqlx::query_as!(
        ActivityResponse,
        r#"
        SELECT id, category as "category: Category", subcategory, quantity, unit, calculated_emission_kg, date
        FROM activities
        WHERE user_id = $1 AND date = $2
        ORDER BY created_at DESC
        "#,
        claims.sub,
        parsed_date
    )
    .fetch_all(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to fetch activities"))?;

    return Ok(json!({
        "status": "success",
        "data": activities
    }));
}

#[delete("/<id>")]
pub async fn delete_activity(
    claims: Claims,
    id: String,
    pool: &State<DbPool>,
) -> Result<Value, ApiError> {
    let activity_id = Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("ID invalid"))?;
    let mut tx = pool
        .0
        .begin()
        .await
        .map_err(|_| ApiError::internal("Failed to start transaction"))?;
    let activity = sqlx::query!(
        r#"SELECT category, calculated_emission_kg, date FROM activities WHERE id = $1 AND user_id = $2"#,
        activity_id, claims.sub
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| ApiError::internal("Failed to fetch activity"))?;

    let activity = match activity {
        Some(a) => a,
        None => return Err(ApiError::bad_request("Activity not found or unauthorized")),
    };

    sqlx::query!("DELETE FROM activities WHERE id = $1", activity_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Failed to delete activity"))?;

    let category_str = activity.category;
    let update_query = format!(
        r#"
        UPDATE daily_summaries 
        SET 
            total_emission = total_emission - $3,
            {0}_kg = {0}_kg - $3
        WHERE user_id = $1 AND date = $2
        "#,
        category_str
    );

    sqlx::query(&update_query)
        .bind(claims.sub)
        .bind(activity.date)
        .bind(activity.calculated_emission_kg)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Failed to rollback daily summary"))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Failed to commit transaction"))?;

    return Ok(json!({ "status": "success", "message": "Activity successfully deleted" }));
}

#[derive(serde::Serialize)]
pub struct FactorDto {
    pub category: String,
    pub subcategory: String,
    pub unit: String,
}

#[get("/factors")]
pub async fn get_factors(pool: &State<DbPool>) -> Result<Value, ApiError> {
    let factors = sqlx::query_as!(
        FactorDto,
        r#"SELECT category, subcategory, unit FROM emission_factors ORDER BY category, subcategory"#
    )
    .fetch_all(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to fetch emission factors"))?;

    return Ok(json!({
        "status": "success",
        "data": factors
    }));
}

pub fn routes() -> Vec<Route> {
    return routes![log_activity, get_activities, delete_activity, get_factors];
}
