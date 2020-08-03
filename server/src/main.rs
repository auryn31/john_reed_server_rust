#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate chrono;
extern crate reqwest;

use anyhow::Result;
use chrono::{Duration, Local};
use redis::{Commands, Connection};

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

#[get("/?<studio>&<yesterday>")]
fn request_redis(studio: String, yesterday: bool) -> Result<String> {
    let mut connection = open_redis_connection()?;
    let key = create_key(&studio, yesterday)?;
    match connection.get(&key) {
        Ok(it) => match it {
            Some(res) => Ok(res),
            None => Ok(load_and_save_data(studio, &mut connection, key)?),
        },
        Err(_err) => load_and_save_data(studio, &mut connection, key),
    }
}

fn john_reed_data(studio: String) -> Result<String> {
    let url = format!(
        "https://typo3.johnreed.fitness/studiocapacity.json?studioId={}",
        studio
    );
    let body = reqwest::blocking::get(&url)?.text()?;
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

fn load_and_save_data(
    studio_id: String,
    connection: &mut Connection,
    unwraped_key: String,
) -> Result<String> {
    let john_reed_data = john_reed_data(studio_id)?;
    connection.set(&unwraped_key, john_reed_data.clone())?;
    Ok(john_reed_data)
}

fn open_redis_connection() -> anyhow::Result<Connection> {
    let client = redis::Client::open("redis://localhost:6379/")?;
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

fn main() {
    rocket::ignite().mount("/", routes![request_redis]).launch();
}
