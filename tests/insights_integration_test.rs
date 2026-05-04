use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use std::env;

mod common;

async fn get_auth_token(client: &Client, device_id: String) -> String {
    let email = format!("insights_{}@example.com", Uuid::new_v4());
    let password = "SecurePassword123!";

    let register_body = json!({
        "email": email,
        "password": password,
        "display_name": "Insights Tester"
    });

    let register_resp = client
        .post("/api/auth/register")
        .header(Header::new("X-Device-ID", device_id))
        .header(ContentType::JSON)
        .body(register_body.to_string())
        .dispatch()
        .await;

    let resp_json: Value = register_resp
        .into_json()
        .await
        .unwrap();
    let data = resp_json.get("data").unwrap();
    return data
        .get("access_token")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
}

#[rocket::async_test]
async fn test_insights_recommendations() {
    // 1. Setup Mock Server
    let mock_server = MockServer::start().await;

    // Define the mock response from Groq
    let mock_response = json!({
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "Bro, chill on the car rides. Walk more, fr! 🚶‍♂️🌿"
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/openai/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    // 2. Override Env Vars for Groq
    let mock_uri = format!("{}/openai/v1/chat/completions", mock_server.uri());
    env::set_var("GROQ_API_URL", mock_uri);
    env::set_var("GROQ_API_KEY", "mock_key");

    // 3. Setup Rocket Client
    let client = common::get_client().await;
    let device_id = Uuid::new_v4().to_string();
    let token = get_auth_token(&client, device_id.clone()).await;

    // 4. Hit Recommendations Endpoint
    let insights_resp = client
        .get("/api/insights/recommendations")
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .header(Header::new("X-Device-ID", device_id))
        .dispatch()
        .await;

    assert_eq!(insights_resp.status(), Status::Ok);
    
    let insights_json: Value = insights_resp.into_json().await.unwrap();
    let data = &insights_json["data"];
    
    assert_eq!(data["ai_insight"].as_str().unwrap(), "Bro, chill on the car rides. Walk more, fr! 🚶‍♂️🌿");
}
