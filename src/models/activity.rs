use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

#[derive(Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Transport,
    Food,
    Energy,
    Shopping,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Transport => "transport",
            Category::Food => "food",
            Category::Energy => "energy",
            Category::Shopping => "shopping",
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ActivityRequest {
    pub category: Category,
    pub subcategory: String,
    pub quantity: Decimal,
    pub date: NaiveDate,
}

#[derive(Serialize)]
pub struct ActivityResponse {
    pub id: Uuid,
    pub category: Category,
    pub subcategory: String,
    pub quantity: Decimal,
    pub unit: String,
    pub calculated_emission_kg: Decimal,
    pub date: NaiveDate,
}
