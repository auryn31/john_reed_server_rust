extern crate chrono;
extern crate reqwest;

use actix_protobuf::*;
use actix_web::{get, middleware, web, App, HttpServer};
use anyhow::Result;
use chrono::{Duration, Local};
use redis::{Commands, Connection};
use serde::Deserialize;
use std::time::SystemTime;
#[path = "model/model.rs"]
mod model;
use model::ResponseData;

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
async fn request_redis(
    web::Query(params): web::Query<RequestParams>,
) -> actix_web::Result<actix_web::HttpResponse> {
    println!("{:?}", params);
    let mut connection =
        open_redis_connection().map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    let key = create_key(&params.studio, params.yesterday)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    let resp = match connection.get(&key) {
        Ok(it) => match it {
            Some(res) => {
                let res: String = res;
                let mut data = serde_json::from_str::<ResponseData>(&res)?;
                data.items = data
                    .items
                    .into_iter()
                    .filter(|it| it.percentage > 0)
                    .collect();
                Ok(data)
            }
            None => Ok(load_and_save_data(params.studio, &mut connection, key)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?),
        },
        Err(_err) => load_and_save_data(params.studio, &mut connection, key)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e)),
    };
    resp.map(|it| actix_web::HttpResponse::Ok().body(serde_json::json!(it).to_string()))
}

pub mod items {
    include!(concat!(env!("OUT_DIR"), "/response.data.rs"));
}

#[get("/proto")]
async fn request_redis_proto(
    web::Query(params): web::Query<RequestParams>,
) -> actix_web::Result<actix_web::HttpResponse> {
    println!("{:?}", params);
    let mut connection =
        open_redis_connection().map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    let key = create_key(&params.studio, params.yesterday)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    let resp = match connection.get(&key) {
        Ok(it) => match it {
            Some(res) => {
                let res: String = res;
                Ok(serde_json::from_str::<ResponseData>(&res)?)
            }
            None => Ok(
                load_and_save_data_proto(params.studio, &mut connection, key)
                    .await
                    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
            ),
        },
        Err(_err) => load_and_save_data_proto(params.studio, &mut connection, key)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e)),
    };
    let response_data = map_response_data(resp?);
    actix_web::HttpResponse::Ok().protobuf(response_data)
}

fn map_response_data(resp: ResponseData) -> items::ResponseData {
    let mut response_data = items::ResponseData::default();
    response_data.start_time = resp.start_time.clone();
    response_data.end_time = resp.end_time.clone();
    response_data.items = resp
        .items
        .iter()
        .map(|it| {
            let mut entry = items::response_data::DataEntry::default();
            entry.start_time = it.start_time.clone();
            entry.end_time = it.end_time.clone();
            entry.percentage = it.percentage.clone().into();
            entry.is_current = it.is_current.clone();
            entry.level = match it.level {
                model::Level::LOW => items::response_data::data_entry::Level::Low as i32,
                model::Level::NORMAL => items::response_data::data_entry::Level::Normal as i32,
                model::Level::HIGH => items::response_data::data_entry::Level::High as i32,
            };
            entry
        })
        .collect();
    response_data
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

fn open_redis_connection() -> Result<Connection> {
    let client = redis::Client::open("redis://localhost:6379")?;
    let connection = client.get_connection()?;
    Ok(connection)
}

async fn load_and_save_data(
    studio_id: String,
    connection: &mut Connection,
    unwraped_key: String,
) -> Result<ResponseData> {
    let john_reed_data = john_reed_data(studio_id).await?;
    connection.set(&unwraped_key, john_reed_data.clone())?;
    let mut data = serde_json::from_str::<ResponseData>(&john_reed_data)?;
    data.items = data
        .items
        .into_iter()
        .filter(|it| it.percentage > 0)
        .collect();
    Ok(data)
}

async fn load_and_save_data_proto(
    studio_id: String,
    connection: &mut Connection,
    unwraped_key: String,
) -> Result<ResponseData> {
    let john_reed_data = john_reed_data(studio_id).await?;
    connection.set(&unwraped_key, john_reed_data.clone())?;
    let data = serde_json::from_str::<ResponseData>(&john_reed_data)?;
    Ok(data)
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
            None => Err(anyhow::Error::msg(
                "The array of keys is empty but it reached the sort function",
            )),
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

async fn load_every_hour(studio_id: &str) -> () {
    println!("Start laoding for ID: {}", studio_id);
    if let Ok(key) = create_key(studio_id, false) {
        if let Ok(mut connection) = open_redis_connection() {
            if let Ok(_) = load_and_save_data(studio_id.to_owned(), &mut connection, key).await {
                println!("Loaded data for ID: {}", studio_id);
            };
        };
    };
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    actix_rt::spawn(async {
        let mut now = SystemTime::now();
        loop {
            tokio::time::delay_for(std::time::Duration::from_secs(1200)).await;
            match now.elapsed() {
                Ok(elapsed) => {
                    if elapsed.as_secs() > 3600 {
                        load_every_hour("1414810010").await;
                        now = SystemTime::now();
                    }
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                }
            }
        }
    });

    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(request_redis)
            .service(request_redis_proto)
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
