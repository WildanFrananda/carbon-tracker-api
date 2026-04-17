use rust_decimal::Decimal;
use sqlx::{Error, PgPool};

use crate::models::activity::Category;

pub struct EmissionFactor {
    pub factor_value: Decimal,
    pub unit: String,
}

pub async fn get_emission_factor(
    pool: &PgPool,
    category: &Category,
    subcategory: &str,
) -> Result<Option<EmissionFactor>, Error> {
    let cat_str = category.as_str();
    let result = sqlx::query!(
        r#"
        SELECT factor_value, unit 
        FROM emission_factors 
        WHERE category = $1 AND subcategory = $2
        "#,
        cat_str,
        subcategory
    )
    .fetch_optional(pool)
    .await?;

    return Ok(result.map(|record| EmissionFactor {
        factor_value: record.factor_value,
        unit: record.unit,
    }));
}
