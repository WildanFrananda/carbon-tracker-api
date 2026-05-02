use crate::RedisPool;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Method};
use rocket::{Request, Data, self};
use rocket::http::uri::Origin;

pub struct RateLimitFairing;

#[rocket::async_trait]
impl Fairing for RateLimitFairing {
    fn info(&self) -> Info {
        Info {
            name: "Global Rate Limitter",
            kind: Kind::Request
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        if req.uri().path() == "/errors/429" {
            return;
        }

        let device_id = match req.headers().get_one("X-Device-ID") {
            Some(id) if id.len() >= 8 => id,
            _ => {
                req.set_method(Method::Get);

                req.set_uri(Origin::parse("/errors/400").unwrap());
                return;
            }
        };

        let redis_pool = match req.rocket().state::<RedisPool>() {
            Some(pool) => pool,
            None => return
        };

        let mut conn = match redis_pool.0.get().await {
            Ok(c) => c,
            Err(_) => return
        };

        let key = format!("ratelimit:{}", device_id);
        let limit = 3;
        let window = 1;

        let count: i64 = redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut *conn)
            .await
            .unwrap_or(0);

        if count == 1 {
            let _: () = redis::cmd("EXPIRE")
                .arg(&key)
                .arg(window)
                .query_async(&mut *conn)
                .await
                .unwrap_or(());
        }

        if count > limit {
            req.set_method(Method::Get);
            req.set_uri(Origin::parse("/errors/429").unwrap());
        }
    }
}