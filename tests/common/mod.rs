use rocket::local::asynchronous::Client;
use std::sync::Once;
use dotenvy::dotenv;

static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        // We try to load .env, but for tests we might override some env vars
        dotenv().ok();
        
        // Ensure REDIS_URL is set for tests
        if std::env::var("REDIS_URL").is_err() {
            std::env::set_var("REDIS_URL", "redis://127.0.0.1:6379");
        }
        
        // Setup a test database URL if not explicitly set
        if std::env::var("DATABASE_URL").is_err() {
            std::env::set_var("DATABASE_URL", "postgres://tracker_user:tracker_pass@localhost:5432/carbon_tracker_db");
        }
    });
}

pub async fn get_client() -> Client {
    setup();
    let rocket = carbon_tracker_api::build_rocket().await;
    Client::tracked(rocket).await.expect("Valid rocket instance")
}
