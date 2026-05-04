use rocket::http::{ContentType, Header, Status};
use serde_json::{json, Value};
use uuid::Uuid;

mod common;

#[rocket::async_test]
async fn test_auth_register_and_login() {
    let client = common::get_client().await;

    // Use random email to avoid collision on multiple runs
    let email = format!("testuser_{}@example.com", Uuid::new_v4());
    let password = "SecurePassword123!";

    // 1. Register User
    let register_body = json!({
        "email": email,
        "password": password,
        "display_name": "Test User"
    });

    let register_resp = client
        .post("/api/auth/register")
        .header(Header::new("X-Device-ID", "test-device-id"))
        .header(ContentType::JSON)
        .body(register_body.to_string())
        .dispatch()
        .await;

    assert_eq!(register_resp.status(), Status::Ok);
    
    // Check if tokens are returned
    let resp_json: Value = register_resp
        .into_json()
        .await
        .unwrap();
    assert!(resp_json.get("data").is_some());
    let data = resp_json.get("data").unwrap();
    assert!(data.get("access_token").is_some());
    assert!(data.get("refresh_token").is_some());

    // 2. Login User
    let login_body = json!({
        "email": email,
        "password": password
    });

    let login_resp = client
        .post("/api/auth/login")
        .header(Header::new("X-Device-ID", "test-device-id"))
        .header(ContentType::JSON)
        .body(login_body.to_string())
        .dispatch()
        .await;

    assert_eq!(login_resp.status(), Status::Ok);
    let login_json: Value = login_resp
        .into_json()
        .await
        .unwrap();
    assert!(login_json.get("data").is_some());
}

#[rocket::async_test]
async fn test_auth_login_invalid_credentials() {
    let client = common::get_client().await;

    let login_body = json!({
        "email": "nonexistent_user@example.com",
        "password": "WrongPassword!"
    });

    let login_resp = client
        .post("/api/auth/login")
        .header(Header::new("X-Device-ID", "test-device-id"))
        .header(ContentType::JSON)
        .body(login_body.to_string())
        .dispatch()
        .await;

    assert_eq!(login_resp.status(), Status::Unauthorized);
}
