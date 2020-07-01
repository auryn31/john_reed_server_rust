#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate chrono;
extern crate reqwest;

use curl::easy::Easy;
use redis::{AsyncCommands, Commands};
use chrono::Local;
use std::io::Read;
use reqwest::Error;

#[get("/hello")]
fn index() -> String {
    let mut easy = Easy::new();
    let mut data = Vec::new();
    easy.url("https://www.rust-lang.org/").unwrap();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|new_data| {
            data.extend_from_slice(new_data);
            Ok(new_data.len())
        }).unwrap();
        transfer.perform().unwrap();
    }
    return String::from_utf8(data).expect("body is not valid UTF8!");
}

#[get("/redis")]
fn request_redis() -> Result<String, Error> {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();
    let now = Local::now();
    let date_formatted_string = now.format("%Y-%m-%d-%H").to_string();
    let key = "1414810010".to_string() + &date_formatted_string;
    let return_val = match con.get(&key) {
        Ok(f) => f,
        Err(err) => {
            let new_data = john_reed_data("1414810010".to_string())?;
            let _ : () = con.set(&key, &new_data).unwrap();
            new_data
        }
    };

    Ok(return_val)
}

#[get("/jr?<studio>")]
fn john_reed_data(studio: String) -> Result<String, Error> {
    let url = format!("https://typo3.johnreed.fitness/studiocapacity.json?studioId={}",studio);
    let body = reqwest::blocking::get(&url)?
        .text();
    body
}

#[get("/?<studio>&<yesterday>")]
fn johnReedData(studio: String, yesterday: bool) -> String {
    let url = format!("https://typo3.johnreed.fitness/studiocapacity.json?studioId={}",studio);
    let mut easy = Easy::new();
    let mut data = Vec::new();
    easy.url(url.as_ref()).unwrap();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|new_data| {
            data.extend_from_slice(new_data);
            Ok(new_data.len())
        }).unwrap();
        transfer.perform().unwrap();
    }
    String::from_utf8(data).expect("body is not valid UTF8!")
}

fn main() {
    rocket::ignite().mount("/", routes![index, johnReedData, request_redis, john_reed_data]).launch();
}