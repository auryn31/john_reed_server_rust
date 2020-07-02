#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate chrono;
extern crate reqwest;

use redis::{Commands, Connection};
use chrono::{Local, Duration};
use reqwest::Error;

#[get("/keys?<studio_id>&<yesterday>")]
fn load_newest_key_from_redis(studio_id: String, yesterday: bool) -> Result<String, Error> {
    let mut connection = open_redis_connection();
    let key = create_redis_search_key(&studio_id, yesterday);
    let redis_keys: Vec<String> = match connection.keys(&key) {
        Ok(f) => f,
        Err(_err) => panic!(format!("No keys found for studio with id: {} and key: {}", &studio_id, &key))
    };
    extract_newest_key(redis_keys)
}

#[get("/?<studio_id>&<yesterday>")]
fn request_redis(studio_id: String, yesterday: bool) -> Result<String, Error> {
    let mut connection = open_redis_connection();
    let mut key = create_current_key(&studio_id);
    if yesterday {
        key = match load_newest_key_from_redis(studio_id.to_string(), yesterday.clone()) {
            Ok(it) => it,
            Err(_err) => panic!("Fail to create newest key for yesterday")
        };
    }
    let return_val = match connection.get(&key) {
        Ok(f) => f,
        Err(_err) => {
            let new_data = john_reed_data(studio_id.to_string())?;
            let _: () = connection.set(&key, &new_data).unwrap();
            new_data
        }
    };
    Ok(return_val)
}

#[get("/jr?<studio>")]
fn john_reed_data(studio: String) -> Result<String, Error> {
    let url = format!("https://typo3.johnreed.fitness/studiocapacity.json?studioId={}", studio);
    let body = reqwest::blocking::get(&url)?
        .text();
    body
}

fn open_redis_connection() -> Connection {
    let client = match redis::Client::open("redis://127.0.0.1/") {
        Ok(it) => it,
        Err(_err) => panic!("Could not reach redis")
    };
    match client.get_connection() {
        Ok(it) => it,
        Err(_err) => panic!("Could not create a connection to redis")
    }
}

fn extract_newest_key(mut redis_keys: Vec<String>) -> Result<String, Error> {
    if redis_keys.len() == 0 {
        panic!("The array of keys is empty")
    }
    if redis_keys.len() == 1 {
        Ok(redis_keys.get(0).unwrap().to_string())
    } else {
        redis_keys.sort();
        redis_keys.reverse();
        Ok(redis_keys.get(0).unwrap().to_string())
    }
}

fn create_redis_search_key(studio_id: &String, yesterday: bool) -> String {
    let mut now = Local::now();
    if yesterday {
        now = now - Duration::days(1);
    }
    let date_formatted_string = now.format("%Y-%m-%d-*").to_string();
    let key = studio_id.to_string() + &date_formatted_string;
    key
}

fn create_current_key(studio_id: &String) -> String {
    let now = Local::now();
    let date_formatted_string = now.format("%Y-%m-%d-%H").to_string();
    let key = studio_id.to_string() + &date_formatted_string;
    key
}

fn main() {
    rocket::ignite().mount("/", routes![request_redis, john_reed_data, load_newest_key_from_redis]).launch();
}