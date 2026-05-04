#[macro_use]
extern crate rocket;

use carbon_tracker_api::build_rocket;

#[launch]
async fn rocket() -> _ {
    build_rocket().await
}

#[cfg(test)]
mod tests {
    use super::build_rocket;
    use dotenvy::dotenv;
    use rocket::http::Status;
    use rocket::local::asynchronous::{Client, LocalResponse};
    use std::sync::Once;

    static INIT: Once = Once::new();
    fn setup() {
        INIT.call_once(|| {
            dotenv().ok();
        });
    }

    #[tokio::test]
    async fn test_index_route() {
        setup();
        let client = Client::tracked(build_rocket().await)
            .await
            .expect("Valid rocket instance");

        let response: LocalResponse = client.get("/")
            .header(rocket::http::Header::new("X-Device-ID", "test-device-id-12345"))
            .dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body_str: String = response
            .into_string()
            .await
            .expect("Response body should be a string");
        assert!(body_str.contains("Welcome to Personal Footprint Tracker API"));
    }

    #[rocket::async_test]
    async fn test_health_check() {
        setup();
        let client = Client::tracked(build_rocket().await)
            .await
            .expect("Valid rocket instance");

        let response = client.get("/health")
            .header(rocket::http::Header::new("X-Device-ID", "test-device-id-12345"))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_unauthorized_access() {
        setup();
        let client = Client::tracked(build_rocket().await)
            .await
            .expect("Valid rocket instance");

        let response = client
            .get("/api/dashboard/daily?date=2026-04-17")
            .header(rocket::http::Header::new("X-Device-ID", "test-device-id-12345"))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Unauthorized);
    }
}
