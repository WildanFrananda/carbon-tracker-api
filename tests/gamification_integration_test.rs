use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

mod common;

async fn get_auth_token(client: &Client, device_id: String) -> String {
    let email = format!("gamify_{}@example.com", Uuid::new_v4());
    let password = "SecurePassword123!";

    let register_body = json!({
        "email": email,
        "password": password,
        "display_name": "Gamify Tester"
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
async fn test_gamification_badges() {
    let client = common::get_client().await;
    let device_id = Uuid::new_v4().to_string();
    let token = get_auth_token(&client, device_id.clone()).await;

    // 1. Initial check: Should have no badges
    let badges_resp = client
        .get("/api/gamification/badges")
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .dispatch()
        .await;

    assert_eq!(badges_resp.status(), Status::Ok);
    let badges_json: Value = badges_resp.into_json().await.unwrap();
    let badges_array = badges_json["data"].as_array().unwrap();
    assert_eq!(badges_array.len(), 0);

    // Wait 1.1s to avoid rate limiter (3 requests per sec)
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    // 2. Log first activity to earn FIRST_LOG badge
    let activity_body = json!({
        "category": "transport",
        "subcategory": "car",
        "quantity": 5.0,
        "date": "2026-05-15"
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

    // Wait 1.1s to avoid rate limiter (3 requests per sec)
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // 3. Check badges again, should have FIRST_LOG
    let badges_resp_after = client
        .get("/api/gamification/badges")
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .dispatch()
        .await;

    assert_eq!(badges_resp_after.status(), Status::Ok);
    let badges_json_after: Value = badges_resp_after.into_json().await.unwrap();
    let badges_array_after = badges_json_after["data"].as_array().unwrap();
    assert_eq!(badges_array_after.len(), 2);
    let mut badge_types = vec![];

    for b in badges_array_after {
        badge_types.push(b["badge_type"].as_str().unwrap());
    }

    assert!(badge_types.contains(&"FIRST_LOG"));
    assert!(badge_types.contains(&"LOW_CARBON_COMMUTER"));
}
