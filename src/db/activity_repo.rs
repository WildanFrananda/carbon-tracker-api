use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::engine::calculator::calculate_emission;
use crate::engine::factors::get_emission_factor;
use crate::models::activity::{ActivityRequest, ActivityResponse};
use crate::services::gamification::evaluate_achievements;

pub async fn log_activity_with_transaction(
    pool: &PgPool,
    user_id: Uuid,
    req: &ActivityRequest,
) -> Result<ActivityResponse, String> {
    let factor = get_emission_factor(pool, &req.category, &req.subcategory)
        .await
        .map_err(|_| "Error fetching factors".to_string())?;

    let factor = match factor {
        Some(f) => f,
        None => return Err("Category or Subcategory invalid".to_string()),
    };

    let calculated_emission = calculate_emission(req.quantity, factor.factor_value);

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| "Failed to create transaction".to_string())?;
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
    .map_err(|_| "Failed to insert activity".to_string())?;

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
        .map_err(|_| "Failed to update daily summary".to_string())?;

    tx.commit()
        .await
        .map_err(|_| "Failed to commit transaction")?;

    if let Err(e) = evaluate_achievements(pool, user_id, req.date).await {
        println!(
            "Warning: Failed to evaluate achievements for user {}: {}",
            user_id, e
        );
    }

    return Ok(ActivityResponse {
        id: activity_id,
        category: req.category.clone(),
        subcategory: req.subcategory.clone(),
        quantity: req.quantity,
        unit: factor.unit,
        calculated_emission_kg: calculated_emission,
        date: req.date,
    });
}
