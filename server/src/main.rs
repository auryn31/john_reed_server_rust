#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate chrono;
extern crate reqwest;

use redis::{Commands, Connection};
use chrono::{Local, Duration};
use reqwest::Error;

// #[get("/keys?<studio_id>&<yesterday>")]
fn load_newest_key_from_redis(studio_id: String, yesterday: bool) -> Option<String> {
    let mut connection = open_redis_connection()?;
    let key = create_redis_search_key(&studio_id, yesterday);
    let redis_keys: Option<Vec<String>> = match connection.keys(&key) {
        Ok(it) => it,
        Err(_err) => {
            println!("No keys found for studio with id: {} and key: {}", &studio_id, &key);
            None
        }
    };
    match redis_keys {
        Some(it) => extract_newest_key(it),
        None => None
    }
}

#[get("/?<studio>&<yesterday>")]
fn request_redis(studio: String, yesterday: bool) -> Option<String> {
    let mut connection = match open_redis_connection() {
        Some(it) => it,
        None => {
            println!("Could not open the redis connection");
            return None;
        }
    };
    let key = match create_key(&studio, yesterday) {
        Some(it) => it,
        None => return None
    };
    match connection.get(&key) {
        Ok(it) => {
            match it {
                Some(res) => res,
                None => load_and_save_data(studio, &mut connection, key.to_string())
            }
        }
        Err(_err) => load_and_save_data(studio, &mut connection, key.to_string())
    }
}

// #[get("/jr?<studio>")]
fn john_reed_data(studio: String) -> Result<String, Error> {
    let url = format!("https://typo3.johnreed.fitness/studiocapacity.json?studioId={}", studio);
    let body = reqwest::blocking::get(&url)?
        .text();
    body
}

fn create_key(studio_id: &String, yesterday: bool) -> Option<String> {
    let mut key = Option::from(create_current_key(&studio_id));
    if yesterday {
        key = load_newest_key_from_redis(studio_id.to_string(), yesterday.clone());
    }
    key
}

fn load_and_save_data(studio_id: String, connection: &mut Connection, unwraped_key: String) -> Option<String> {
    let john_reed_data = match john_reed_data(studio_id.to_string()) {
        Ok(it) => Option::from(it),
        _ => {
            println!("Could not load data from John Reed");
            Option::None
        }
    };
    let unwraped_john_reed_data = match &john_reed_data {
        Some(it) => {
            let _: () = match connection.set(&unwraped_key, it) {
                Ok(it) => it,
                _ => { println!("Could not save data to redis") }
            };
            john_reed_data
        }
        None => None
    };
    unwraped_john_reed_data
}

fn open_redis_connection() -> Option<Connection> {
    let client = match redis::Client::open("redis://redis/") {
        Ok(it) => it,
        Err(_err) => panic!("Could not reach redis")
    };
    match client.get_connection() {
        Ok(it) => Option::from(it),
        Err(_err) => {
            println!("Could not get a connection");
            Option::None
        }
    }
}

fn extract_newest_key(mut redis_keys: Vec<String>) -> Option<String> {
    if redis_keys.len() == 0 {
        println!("The array of keys is empty");
        return Option::None;
    }
    return if redis_keys.len() == 1 {
        Option::from(redis_keys.get(0).unwrap().to_string())
    } else {
        redis_keys.sort();
        redis_keys.reverse();
        Option::from(redis_keys.get(0).unwrap().to_string())
    };
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
    rocket::ignite().mount("/", routes![request_redis]).launch();
}