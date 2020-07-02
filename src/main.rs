#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate chrono;
extern crate reqwest;

use chrono::{Duration, Local};
use redis::{Commands, Connection};
use rocket::http::Status;

#[get("/keys?<studio_id>&<yesterday>")]
fn load_newest_key_from_redis(studio_id: String, yesterday: bool) -> Result<String, Status> {
    let mut connection = open_redis_connection()?;
    let key = create_redis_search_key(&studio_id, yesterday);
    let redis_keys = connection
        .keys::<&str, Option<Vec<String>>>(&key)
        .or(Err(Status::InternalServerError))?
        .ok_or(Status::NotFound)?;
    match extract_newest_key(redis_keys) {
        Some(val) => Ok(val),
        None => Err(Status::NotFound),
    }
}

#[get("/?<studio_id>&<yesterday>")]
fn request_redis(studio_id: String, yesterday: bool) -> Result<String, Status> {
    let mut connection = open_redis_connection()?;
    let key = if yesterday {
        load_newest_key_from_redis(studio_id.to_string(), yesterday)?
    } else {
        create_current_key(&studio_id)
    };
    connection.get(&key).or_else(|_err| {
        let new_data = john_reed_data(studio_id.to_string())?;
        let _: Result<String, redis::RedisError> = connection.set(&key, &new_data);
        Ok(new_data)
    })
}

#[get("/jr?<studio>")]
fn john_reed_data(studio: String) -> Result<String, Status> {
    let url = format!(
        "https://typo3.johnreed.fitness/studiocapacity.json?studioId={}",
        studio
    );
    reqwest::blocking::get(&url)
        .and_then(|res| res.text())
        .or(Err(Status::NotFound))
}

fn open_redis_connection() -> Result<Connection, Status> {
    redis::Client::open("redis://127.0.0.1/")
        .and_then(|res| res.get_connection())
        .or(Err(Status::InternalServerError))
}

fn extract_newest_key(mut redis_keys: Vec<String>) -> Option<String> {
    if redis_keys.is_empty() {
        None
    } else {
        if redis_keys.len() != 1 {
            redis_keys.sort();
            redis_keys.reverse();
        }
        redis_keys.get(0).map(|val| val.to_string())
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

fn main() {
    rocket::ignite()
        .mount(
            "/",
            routes![request_redis, john_reed_data, load_newest_key_from_redis],
        )
        .launch();
}
