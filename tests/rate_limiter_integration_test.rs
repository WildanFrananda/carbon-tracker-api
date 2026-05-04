use rocket::http::{Header, Status};
use std::time::Duration;

mod common;

#[rocket::async_test]
async fn test_rate_limiter_missing_header() {
    let client = common::get_client().await;

    // A request without X-Device-ID should return 400 Bad Request
    let response = client.get("/health").dispatch().await;
    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn test_rate_limiter_valid_header() {
    let client = common::get_client().await;

    // A request with X-Device-ID should return 200 OK (if under limit)
    let response = client
        .get("/health")
        .header(Header::new("X-Device-ID", "test-device-id-123"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn test_rate_limiter_too_many_requests() {
    let client = common::get_client().await;
    let device_id = "test-device-id-rate-limit";

    // Rate limit is 3 requests per 1 second.
    // Send 3 successful requests
    for _ in 0..3 {
        let response = client
            .get("/health")
            .header(Header::new("X-Device-ID", device_id))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
    }

    // 4th request should fail
    let response = client
        .get("/health")
        .header(Header::new("X-Device-ID", device_id))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::TooManyRequests);

    // Wait for 1.1 seconds and it should succeed again
    tokio::time::sleep(Duration::from_millis(1100)).await;

    let response = client
        .get("/health")
        .header(Header::new("X-Device-ID", device_id))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}
