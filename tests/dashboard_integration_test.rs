use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

mod common;

async fn get_auth_token(client: &Client, device_id: String) -> String {
    let email = format!("dash_{}@example.com", Uuid::new_v4());
    let password = "SecurePassword123!";

    let register_body = json!({
        "email": email,
        "password": password,
        "display_name": "Dashboard Tester"
    });

    let register_resp = client
        .post("/api/auth/register")
        .header(Header::new("X-Device-ID", device_id))
        .header(ContentType::JSON)
        .body(register_body.to_string())
        .dispatch()
        .await;

    let resp_json: Value = register_resp.into_json().await.unwrap();
    let data = resp_json.get("data").unwrap();

    return data
        .get("access_token")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
}

#[rocket::async_test]
async fn test_dashboard_endpoints() {
    let client = common::get_client().await;
    let device_id = Uuid::new_v4().to_string();
    let token = get_auth_token(&client, device_id.clone()).await;

    // 1. Log an activity to ensure we have data in the dashboard
    let activity_body = json!({
        "category": "food",
        "subcategory": "beef",
        "quantity": 1.5,
        "date": "2026-06-10"
    });

    let create_resp = client
        .post("/api/activities/")
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .header(ContentType::JSON)
        .body(activity_body.to_string())
        .dispatch()
        .await;

    assert_eq!(create_resp.status(), Status::Ok);

    // 2. Test Daily Dashboard
    let daily_resp = client
        .get("/api/dashboard/daily?date=2026-06-10")
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .dispatch()
        .await;

    assert_eq!(daily_resp.status(), Status::Ok);
    let daily_json: Value = daily_resp
        .into_json()
        .await
        .unwrap();
    let daily_data = &daily_json["data"];
    assert!(daily_data["total_emission"].as_f64().unwrap() > 0.0);

    // Wait 1.1s to avoid rate limiter (3 requests per sec)
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // 3. Test Weekly Dashboard
    let weekly_resp = client
        .get("/api/dashboard/weekly?end_date=2026-06-12") // 10th is within the week ending on 12th
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .dispatch()
        .await;

    assert_eq!(weekly_resp.status(), Status::Ok);
    let weekly_json: Value = weekly_resp
        .into_json()
        .await
        .unwrap();
    let weekly_data = &weekly_json["data"];
    assert!(weekly_data["total_emission"].as_f64().unwrap() > 0.0);

    // 4. Test Heatmap
    let heatmap_resp = client
        .get("/api/dashboard/heatmap?year=2026")
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .dispatch()
        .await;

    assert_eq!(heatmap_resp.status(), Status::Ok);
    let heatmap_json: Value = heatmap_resp.into_json().await.unwrap();
    let heatmap_data = heatmap_json["data"].as_array().unwrap();
    assert!(!heatmap_data.is_empty());
}

#[rocket::async_test]
async fn test_dashboard_invalid_date_format() {
    let client = common::get_client().await;
    let device_id = Uuid::new_v4().to_string();
    let token = get_auth_token(&client, device_id.clone()).await;

    let daily_resp = client
        .get("/api/dashboard/daily?date=invalid-date")
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .dispatch()
        .await;

    assert_eq!(daily_resp.status(), Status::BadRequest);
}
