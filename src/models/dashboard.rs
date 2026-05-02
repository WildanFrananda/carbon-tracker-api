use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Serialize)]
pub struct DailySummaryResponse {
    pub date: NaiveDate,
    pub total_emission: Decimal,
    pub transport_kg: Decimal,
    pub food_kg: Decimal,
    pub energy_kg: Decimal,
    pub shopping_kg: Decimal,
    pub is_green_day: bool,
}

#[derive(Serialize)]
pub struct WeeklySummaryResponse {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_emission: Decimal,
    pub average_daily_emission: Decimal,
    pub days_logged: i64,
}

#[derive(Serialize)]
pub struct HeatmapData {
    pub date: NaiveDate,
    pub total_emission: Decimal,
}
