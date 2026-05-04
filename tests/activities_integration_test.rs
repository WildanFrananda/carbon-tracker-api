use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

mod common;

async fn get_auth_token(client: &Client, device_id: String) -> String {
    let email = format!("activity_{}@example.com", Uuid::new_v4());
    let password = "SecurePassword123!";

    let register_body = json!({
        "email": email,
        "password": password,
        "display_name": "Activity Tester"
    });

    let register_resp = client
        .post("/api/auth/register")
        .header(Header::new("X-Device-ID", device_id.clone()))
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
async fn test_activities_crud() {
    let client = common::get_client().await;
    let device_id = Uuid::new_v4().to_string();
    let token = get_auth_token(&client, device_id.clone()).await;

    // 1. Create Activity
    let activity_body = json!({
        "category": "transport",
        "subcategory": "car",
        "quantity": 10.5,
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
    let create_json: Value = create_resp
        .into_json()
        .await
        .unwrap();
    let activity_id = create_json["data"]["id"]
        .as_str()
        .unwrap();

    // 2. Get Activities
    let get_resp = client
        .get("/api/activities/?date=2026-05-15")
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .dispatch()
        .await;

    assert_eq!(get_resp.status(), Status::Ok);
    let get_json: Value = get_resp
        .into_json()
        .await
        .unwrap();
    let activities = get_json["data"].as_array().unwrap();
    assert_eq!(activities.len(), 1);

    // Wait 1.1s to avoid rate limiter (3 requests per sec)
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // 3. Delete Activity
    let delete_resp = client
        .delete(format!("/api/activities/{}", activity_id))
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .dispatch()
        .await;

    assert_eq!(delete_resp.status(), Status::Ok);

    // 4. Verify Deletion
    let get_resp_after = client
        .get("/api/activities/?date=2026-05-15")
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id.clone()))
        .dispatch()
        .await;

    assert_eq!(get_resp_after.status(), Status::Ok);
    let get_json_after: Value = get_resp_after
        .into_json()
        .await
        .unwrap();
    let activities_after = get_json_after["data"]
        .as_array()
        .unwrap();
    assert_eq!(activities_after.len(), 0);
}

#[rocket::async_test]
async fn test_activities_invalid_category() {
    let client = common::get_client().await;
    let device_id = Uuid::new_v4().to_string();
    let token = get_auth_token(&client, device_id.clone()).await;

    // Invalid category
    let activity_body = json!({
        "category": "invalid_category",
        "subcategory": "car",
        "quantity": 10.5,
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

    assert_eq!(create_resp.status(), Status::UnprocessableEntity);
}
