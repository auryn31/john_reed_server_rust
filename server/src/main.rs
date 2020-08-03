extern crate chrono;
extern crate reqwest;

use actix_web::{get, web, App, HttpServer, middleware};
use anyhow::Result;
use chrono::{Duration, Local};
use redis::{Commands, Connection};
use serde::Deserialize;

// #[get("/keys?<studio_id>&<yesterday>")]
fn load_newest_key_from_redis(studio_id: String, yesterday: bool) -> Result<Option<String>> {
    let mut connection = open_redis_connection()?;
    let key = create_redis_search_key(&studio_id, yesterday);
    let redis_keys = connection.keys(&key)?;
    match redis_keys {
        Some(it) => {
            let keys = extract_newest_key(it)?;
            Ok(Some(keys))
        }
        None => Ok(None),
    }
}

#[derive(Deserialize, Debug)]
struct RequestParams {
    studio: String,
    yesterday: bool,
}

#[get("/")]
async fn request_redis(web::Query(params): web::Query<RequestParams>) -> actix_web::Result<actix_web::HttpResponse> {
    println!("{:?}",params);
    let mut connection =
        open_redis_connection().map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    let key = create_key(&params.studio, params.yesterday)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    let resp = match connection.get(&key) {
        Ok(it) => match it {
            Some(res) => Ok(res),
            None => Ok(load_and_save_data(params.studio, &mut connection, key).await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?),
        },
        Err(_err) => load_and_save_data(params.studio, &mut connection, key).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e)),
    };
    resp.map(|it| {
        actix_web::HttpResponse::Ok().body(it)
    })
}

async fn john_reed_data(studio: String) -> Result<String> {
    let url = format!(
        "https://typo3.johnreed.fitness/studiocapacity.json?studioId={}",
        studio
    );
    let body = reqwest::get(&url).await?.text().await?;
    Ok(body)
}

fn create_key(studio_id: &str, yesterday: bool) -> Result<String> {
    let mut key = create_current_key(&studio_id);
    if yesterday {
        key = match load_newest_key_from_redis(studio_id.to_string(), yesterday) {
            Ok(Some(it)) => it,
            _ => return Err(anyhow::Error::msg("Could not load newest key from redis")),
        };
    }
    Ok(key)
}

async fn load_and_save_data(
    studio_id: String,
    connection: &mut Connection,
    unwraped_key: String,
) -> Result<String> {
    let john_reed_data = john_reed_data(studio_id).await?;
    connection.set(&unwraped_key, john_reed_data.clone())?;
    Ok(john_reed_data)
}

fn open_redis_connection() -> anyhow::Result<Connection> {
    let client = redis::Client::open("redis://redis/")?;
    let connection = client.get_connection()?;
    Ok(connection)
}

fn extract_newest_key(mut redis_keys: Vec<String>) -> Result<String> {
    if redis_keys.is_empty() {
        Err(anyhow::Error::msg("The array of keys is empty"))
    } else if redis_keys.len() == 1 {
        match redis_keys.get(0) {
            Some(it) => Ok(it.to_owned()),
            None => Err(anyhow::Error::msg("")),
        }
    } else {
        redis_keys.sort();
        redis_keys.reverse();
        match redis_keys.get(0) {
            Some(it) => Ok(it.to_owned()),
            None => Err(anyhow::Error::msg("")),
        }
    }
}

fn create_redis_search_key(studio_id: &str, yesterday: bool) -> String {
    let mut now = Local::now();
    if yesterday {
        now = now - Duration::days(1);
    }
    let date_formatted_string = now.format("%Y-%m-%d-*").to_string();
    studio_id.to_string() + &date_formatted_string
}

fn create_current_key(studio_id: &str) -> String {
    let now = Local::now();
    let date_formatted_string = now.format("%Y-%m-%d-%H").to_string();
    studio_id.to_string() + &date_formatted_string
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    HttpServer::new(|| App::new()
        .wrap(middleware::Logger::default())
        .service(request_redis))
        .bind("0.0.0.0:8000")?
        .run()
        .await
}
