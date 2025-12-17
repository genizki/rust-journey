use crate::share::{SearchResponse, WorkerMessage, YT_DLP_BINARY};
use reqwest::Client;
use serde_json;
use std::env;
use std::error::Error;
use tokio::io::AsyncBufReadExt;

pub async fn call_yt_api(query: String, max_results: i8) -> Result<SearchResponse, Box<dyn Error>> {
    if let Ok(yt_key) = env::var("YT_API") {
        println!("{}", yt_key);
        let url = format!(
            "https://www.googleapis.com/youtube/v3/search?part=snippet&q={}&key={}&maxResults={}&type=video&videCategoryId=10",
            query.replace(" ", "%20"),
            yt_key,
            max_results
        );
        println!("{url}");

        let client = Client::new();
        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            println!("Request failed: {}", response.status());
        }
        let data: SearchResponse = response.json::<SearchResponse>().await?;
        println!("Alle Youtube Title: ");
        let mut index: i8 = 1;
        for item in &data.items {
            let video_title = &item.snippet.title;
            println!("{index}: {video_title}");
            index += 1;
        }
        Ok(data)
    } else {
        println!("No key detected");
        Err("YT_API Key not found".into())
    }
}

pub async fn set_video_durration(
    video_id: Vec<String>,
    meta_data: &mut SearchResponse,
) -> Result<(), Box<dyn Error>> {
    let final_string = video_id.join(",");
    let key = env::var("YT_API").unwrap();
    let url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=contentDetails&id={final_string}&key={key}",
    );
    println!("{}", url);
    let client = Client::new();
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        println!("Request failed: {}", response.status());
    }
    let data: serde_json::Value = response.json::<serde_json::Value>().await?;
    if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
        for item in items {
            if let (Some(video_id), Some(duration)) = (
                item.get("id").and_then(|v| v.as_str()),
                item.get("contentDetails")
                    .and_then(|cd| cd.get("duration"))
                    .and_then(|d| d.as_str()),
            ) {
                println!("{}", duration);
                let formatted_duration = duration
                    .replace("PT", "")
                    .replace("H", ":")
                    .replace("M", ":")
                    .replace("S", "");
                for item in meta_data.items.iter_mut() {
                    if let Some(obj_video_id) = item.id.video_id.as_ref() {
                        if obj_video_id == video_id {
                            item.video_durration = Some(formatted_duration.clone());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn download_from_dlp(
    tx: tokio::sync::mpsc::Sender<WorkerMessage>,
    item_id: usize,
    url: &String,
    download_path: &String,
    audio_format: &'static str,
) -> Result<(), Box<dyn Error>> {
    let download_string = format!("{download_path}/%(title)s.%(ext)s");

    let command = [
        "-x",
        "--audio-format",
        audio_format,
        "-o",
        &download_string,
        "--add-metadata",
        "--embed-thumbnail",
        "--ffmpeg-location",
        "./ffmpeg/ffmpeg",
        "--progress-template",
        "download:%(progress)j",
        "--progress-template",
        "postprocess:%(progress)j",
        url,
    ];

    let mut output = tokio::process::Command::new(YT_DLP_BINARY)
        .args(&command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(stdout) = output.stdout.take() {
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(progress) => {
                    if let Some(procent) = progress.get("_percent_str") {
                        println!("{procent}")
                    }
                }
                Err(_) => {}
            }
        }
    }
    if let Some(stderr) = output.stderr.take() {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            println!("{line}");
        }
    }

    tx.send(WorkerMessage::Done(item_id)).await.unwrap();
    Ok(())
}

pub async fn test_io() -> Result<(), Box<dyn Error>> {
    println!("starting test_io");
    let mut child = tokio::process::Command::new("ping")
        .arg("google.com")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            println!("STDOUT: {}", line);
        }
    }
    Ok(())
}
