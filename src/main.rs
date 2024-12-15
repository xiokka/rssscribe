use std::fs::File;
use std::io;
use std::env;
use serde_json::Value;
use std::process::{Command};
use std::error::Error;
use std::collections::HashMap;

fn get_video_json(channel_url: &str, number_of_items: u64) -> Result<Vec<HashMap<String, Value>>, Box<dyn Error>> {
    let output = Command::new("yt-dlp")
//	.arg("--flat-playlist") // --flat-playlist doesn't include upload_date :(
        .arg("--dump-json")
        .arg("--skip-download")
        .arg("--quiet")
        .arg("--ignore-errors")
        .arg("--playlist-end")
        .arg(number_of_items.to_string()) // Limit to #number_of_items
        .arg(channel_url)
        .output()?;

    if !output.status.success() {
        eprintln!(
            "Error fetching data from yt-dlp: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err("yt-dlp command failed".into());
    }

    let lines = String::from_utf8_lossy(&output.stdout);
    let mut videos = Vec::new();

    for line in lines.lines() {
        match serde_json::from_str::<HashMap<String, Value>>(line) {
            Ok(video) => videos.push(video),
            Err(err) => {
                eprintln!("Error parsing JSON object: {}", err);
            }
        }
    }
    Ok(videos)
}


use chrono::{NaiveDate, TimeZone, Utc};
fn parse_yt_dlp_date(upload_date: &str) -> Result<String, String> {
    let upload_date = upload_date.trim_matches('"').trim();
    if upload_date.len() != 8 {
        return Err("Invalid upload date format".to_string());
    }

    // Parse the upload date to a NaiveDate
    let year = upload_date[0..4].parse::<i32>().map_err(|_| "Invalid year")?;
    let month = upload_date[4..6].parse::<u32>().map_err(|_| "Invalid month")?;
    let day = upload_date[6..8].parse::<u32>().map_err(|_| "Invalid day")?;

    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or("Invalid date")?;

    // Convert the NaiveDate to DateTime in UTC, with midnight as the time
    let datetime = Utc.from_local_date(&date)
        .and_hms_opt(0, 0, 0)
        .single()
        .ok_or("Failed to convert to DateTime")?;

    let formatted = datetime.to_rfc2822();
    Ok(formatted)
}


use rss::Channel;
use rss::Item;
fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 2 {
        eprintln!("Invalid arguments. Usage: rssscribe <number_of_items> <url>");
        return Ok(());
    }
    let channel_url = args[2].to_string();
    let number_of_items: u64 = args[1].parse().expect("Failed to parse input as u64");

    let videos = match get_video_json(&channel_url, number_of_items) {
        Ok(videos) => videos,
        Err(e) => {
            eprintln!("Error: {}", e);
            return Ok(());
        }
    };

    let mut channel = Channel::default();
    let uploader = match videos[0].get(UPLOADER_KEY) {
        Some(&ref value) => value.to_string().trim_matches('"').trim().to_string(),
        None => "Uploader not found".to_string(),
    };
    channel.set_title(uploader);
    let channel_link = channel_url.to_string();
    channel.set_link(channel_link);

    // Generate each item
    let mut items:Vec<Item> = vec![];
    for video in videos {
        let title = match video.get(TITLE_KEY) {
            Some(&ref value) => Some(value.to_string().trim_matches('"').trim().to_string()),
            None => None,
        };

        let link = match video.get(URL_KEY) {
            Some(&ref value) => Some(value.to_string().trim_matches('"').trim().to_string()),
            None => None,
        };

        let mut description = match video.get(DESCRIPTION_KEY) {
            Some(&ref value) => Some(value.to_string().trim_matches('"').trim().to_string()),
            None => None,
        };

        let unparsed_pub_date = match video.get(DATE_KEY) {
            Some(&ref value) => value.to_string(),
            None => "".to_string(),
        };

	let pub_date = parse_yt_dlp_date(&unparsed_pub_date).ok();

        let author = match video.get(UPLOADER_KEY) {
            Some(&ref value) => Some(value.to_string().trim_matches('"').trim().to_string()),
            None => None,
        };

        let thumbnail = match video.get(THUMBNAIL_KEY) {
            Some(&ref value) => Some(value.to_string().trim_matches('"').trim().to_string()),
            None => None,
        };
	let content = format!("<a href=\"{}\"><img src=\"{}\"></a>", link.clone().unwrap(), thumbnail.unwrap());

	let mut item = Item::default();
	item.title = title;
	item.description = description;
	item.content = Some(content);
	item.link = link;
	item.pub_date = pub_date;
	item.author = author;
	items.push(item);

    }
    channel.items = items;

    // Write XML
    let output_path = format!("feed.xml");
    let mut file = File::create(output_path.clone())?;
    channel.write_to(file).unwrap();
    println!("Feed written to {}", output_path);
    return Ok(());
}

pub const TITLE_KEY:&str = "title";
pub const URL_KEY:&str = "original_url";
pub const UPLOADER_KEY:&str = "uploader";
pub const DESCRIPTION_KEY:&str = "description";
pub const DATE_KEY:&str = "upload_date";
pub const THUMBNAIL_KEY:&str = "thumbnail";
