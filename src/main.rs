use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use url::Url;

const YOUTUBE_API_BASE: &str = "https://www.googleapis.com/youtube/v3/";
const API_KEY_ENV: &str = "REEL_SYSTEM_API_KEY";
type AppResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug)]
enum AppError {
    Message(String),
    Api(StatusCode),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => write!(f, "{message}"),
            Self::Api(status) => match status.as_u16() {
                400 => write!(f, "API Error: API key is incorrect. (400; Bad Request)"),
                401 => write!(f, "API Error: Authority is incorrect. (401; Unauthorized)"),
                404 => write!(
                    f,
                    "API Error: Playlist or video is not found. (404; Not Found)"
                ),
                418 => write!(
                    f,
                    "API Error: Brewed a Coffee in a Teapot. (418; I'm a teapot)"
                ),
                _ => write!(f, "API Error: HTTP {}", status.as_u16()),
            },
        }
    }
}

impl Error for AppError {}

#[derive(Clone)]
struct DownloadTask {
    index: usize,
    video_id: String,
    holder: String,
}

struct DownloadFailure {
    index: usize,
    video_id: String,
    holder: String,
    reason: String,
}

enum DownloadOutcome {
    Success,
    Failure(DownloadFailure),
}

#[derive(Debug, Deserialize)]
struct PlaylistResponse {
    items: Vec<Playlist>,
}

#[derive(Debug, Deserialize)]
struct Playlist {
    #[serde(rename = "contentDetails")]
    content_details: PlaylistContentDetails,
}

#[derive(Debug, Deserialize)]
struct PlaylistContentDetails {
    #[serde(rename = "itemCount")]
    item_count: u32,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemsResponse {
    kind: String,
    etag: String,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
    #[serde(default, rename = "nextPageToken")]
    next_page_token: Option<String>,
    items: Vec<PlaylistItem>,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[allow(dead_code)]
    #[serde(rename = "totalResults")]
    total_results: u32,
    #[allow(dead_code)]
    #[serde(rename = "resultsPerPage")]
    results_per_page: u32,
}

#[derive(Debug, Deserialize)]
struct PlaylistItem {
    #[serde(rename = "contentDetails")]
    content_details: PlaylistItemContentDetails,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemContentDetails {
    #[serde(rename = "videoId")]
    video_id: String,
}

#[derive(Debug, Deserialize)]
struct VideosResponse {
    items: Vec<Video>,
}

#[derive(Debug, Deserialize)]
struct Video {
    id: String,
    snippet: VideoSnippet,
}

#[derive(Debug, Deserialize)]
struct VideoSnippet {
    #[serde(rename = "channelTitle")]
    channel_title: String,
    title: String,
}

struct YoutubeApi {
    key: String,
    client: Client,
}

impl YoutubeApi {
    fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            client: Client::new(),
        }
    }

    fn playlists(&self, playlist_id: &str) -> AppResult<PlaylistResponse> {
        let url = format!("{YOUTUBE_API_BASE}playlists");
        let response = self
            .client
            .get(url)
            .query(&[
                ("key", self.key.as_str()),
                ("part", "contentDetails"),
                ("id", playlist_id),
            ])
            .send()?;

        if !response.status().is_success() {
            return Err(Box::new(AppError::Api(response.status())));
        }

        Ok(response.json()?)
    }

    fn playlist_items(&self, playlist_id: &str) -> AppResult<Vec<PlaylistItem>> {
        let playlist = self
            .playlists(playlist_id)?
            .items
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Message("API Error: Playlist is not found.".to_string()))?;

        let mut items = Vec::with_capacity(playlist.content_details.item_count as usize);
        let mut page_token: Option<String> = None;

        loop {
            let url = format!("{YOUTUBE_API_BASE}playlistItems");
            let mut request = self.client.get(url).query(&[
                ("key", self.key.as_str()),
                ("part", "contentDetails,id"),
                ("playlistId", playlist_id),
                ("maxResults", "50"),
            ]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token.as_str())]);
            }

            let response = request.send()?;
            if !response.status().is_success() {
                return Err(Box::new(AppError::Api(response.status())));
            }

            let page: PlaylistItemsResponse = response.json()?;
            let next_page_token = page.next_page_token.clone();
            let _metadata = (page.kind, page.etag, page.page_info);
            items.extend(page.items);

            match next_page_token {
                Some(token) => page_token = Some(token),
                None => break,
            }
        }

        Ok(items)
    }

    fn videos(&self, video_ids: &[String]) -> AppResult<Vec<Video>> {
        let joined_ids = video_ids.join(",");
        let url = format!("{YOUTUBE_API_BASE}videos");
        let response = self
            .client
            .get(url)
            .query(&[
                ("key", self.key.as_str()),
                ("part", "snippet,id"),
                ("id", joined_ids.as_str()),
            ])
            .send()?;

        if !response.status().is_success() {
            return Err(Box::new(AppError::Api(response.status())));
        }

        let videos: VideosResponse = response.json()?;
        Ok(videos.items)
    }
}

fn playlist_url_to_id(input: &str) -> AppResult<String> {
    let url = Url::parse(input)?;
    let host = url.host_str().unwrap_or_default();

    if !matches!(host, "youtube.com" | "www.youtube.com") || url.path() != "/playlist" {
        return Err(Box::new(AppError::Message("URL is incorrect.".to_string())));
    }

    url.query_pairs()
        .find_map(|(key, value)| (key == "list").then(|| value.into_owned()))
        .ok_or_else(|| Box::new(AppError::Message("Parameter is incorrect.".to_string())) as _)
}

fn command_failure(command: &str, status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("{command} exit code {code}"),
        None => format!("{command} terminated by signal"),
    }
}

fn format_label(video_id: &str, holder: &str) -> String {
    let label = format!("{video_id} | {}", holder.trim());
    const MAX_CHARS: usize = 96;

    if label.chars().count() <= MAX_CHARS {
        return label;
    }

    let mut shortened: String = label.chars().take(MAX_CHARS - 3).collect();
    shortened.push_str("...");
    shortened
}

fn log_status(index: usize, total: usize, label: &str, status: &str) {
    println!("[{index:>3}/{total:<3}] {status:<12} {label}");
}

fn download_video(directory: &Path, video_id: &str) -> AppResult<bool> {
    let url = format!("https://youtu.be/{video_id}");
    let output_template = directory.join(format!("{video_id}.%(ext)s"));

    let status = Command::new("yt-dlp")
        .args([
            url.as_str(),
            "--merge-output-format",
            "mkv",
            "-o",
            output_template.to_str().ok_or_else(|| {
                AppError::Message("Directory path is not valid UTF-8.".to_string())
            })?,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !status.success() {
        return Err(Box::new(AppError::Message(command_failure(
            "yt-dlp", status,
        ))));
    }

    let remuxed_mp4 = remux_if_needed(directory, video_id, "mp4")?;
    let remuxed_webm = remux_if_needed(directory, video_id, "webm")?;
    let destination = directory.join(format!("{video_id}.mkv"));

    if !destination.exists() {
        return Err(Box::new(AppError::Message(
            "mkv file was not generated.".to_string(),
        )));
    }

    Ok(remuxed_mp4 || remuxed_webm)
}

fn remux_if_needed(directory: &Path, video_id: &str, source_ext: &str) -> AppResult<bool> {
    let source = directory.join(format!("{video_id}.{source_ext}"));
    if !source.exists() {
        return Ok(false);
    }

    let destination = directory.join(format!("{video_id}.mkv"));
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            source
                .to_str()
                .ok_or_else(|| AppError::Message("Source path is not valid UTF-8.".to_string()))?,
            "-c:v",
            "copy",
            "-c:a",
            "copy",
            destination.to_str().ok_or_else(|| {
                AppError::Message("Destination path is not valid UTF-8.".to_string())
            })?,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !status.success() {
        return Err(Box::new(AppError::Message(command_failure(
            "ffmpeg", status,
        ))));
    }

    fs::remove_file(source)?;
    Ok(true)
}

fn collect_download_tasks(yt_api: &YoutubeApi, ids: &[String]) -> AppResult<Vec<DownloadTask>> {
    let mut tasks = Vec::with_capacity(ids.len());

    for chunk in ids.chunks(50) {
        let videos_by_id: HashMap<String, VideoSnippet> = yt_api
            .videos(chunk)?
            .into_iter()
            .map(|video| (video.id, video.snippet))
            .collect();

        for video_id in chunk {
            if let Some(snippet) = videos_by_id.get(video_id) {
                tasks.push(DownloadTask {
                    index: tasks.len(),
                    video_id: video_id.clone(),
                    holder: format!("{} - {} ", snippet.channel_title, snippet.title),
                });
            }
        }
    }

    Ok(tasks)
}

fn download_videos_parallel(
    directory: &Path,
    tasks: Vec<DownloadTask>,
    parallelism: usize,
    display_total: usize,
    retry: bool,
) -> AppResult<Vec<DownloadOutcome>> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }

    let worker_count = parallelism.max(1).min(tasks.len());
    let directory = Arc::new(directory.to_path_buf());
    let queue = Arc::new(Mutex::new(VecDeque::from(tasks)));
    let results: Arc<Mutex<Vec<DownloadOutcome>>> = Arc::new(Mutex::new(Vec::new()));

    thread::scope(|scope| {
        for _ in 0..worker_count {
            let directory = Arc::clone(&directory);
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);

            scope.spawn(move || {
                loop {
                    let task = {
                        let mut queue = queue.lock().expect("download queue mutex poisoned");
                        queue.pop_front()
                    };

                    let Some(task) = task else {
                        break;
                    };

                    let display_index = task.index + 1;
                    let label = format_label(&task.video_id, &task.holder);
                    log_status(
                        display_index,
                        display_total,
                        &label,
                        if retry {
                            "\u{518d}DL\u{4e2d}"
                        } else {
                            "DL\u{4e2d}"
                        },
                    );

                    let result = match download_video(&directory, &task.video_id) {
                        Ok(remuxed) => {
                            log_status(
                                display_index,
                                display_total,
                                &label,
                                if retry {
                                    "\u{518d}DL\u{5b8c}\u{4e86}"
                                } else {
                                    "DL\u{5b8c}\u{4e86}"
                                },
                            );
                            log_status(
                                display_index,
                                display_total,
                                &label,
                                if remuxed {
                                    "mkv\u{5316}\u{5b8c}\u{4e86}"
                                } else {
                                    "mkv\u{78ba}\u{8a8d}\u{5b8c}\u{4e86}"
                                },
                            );
                            DownloadOutcome::Success
                        }
                        Err(error) => {
                            log_status(
                                display_index,
                                display_total,
                                &label,
                                &format!(
                                    "{} ({error})",
                                    if retry {
                                        "\u{518d}\u{5931}\u{6557}"
                                    } else {
                                        "\u{5f8c}\u{56de}\u{3057}"
                                    }
                                ),
                            );
                            DownloadOutcome::Failure(DownloadFailure {
                                index: task.index,
                                video_id: task.video_id,
                                holder: task.holder,
                                reason: error.to_string(),
                            })
                        }
                    };

                    let mut results = results.lock().expect("download result mutex poisoned");
                    results.push(result);
                }
            });
        }
    });

    let mut results = results
        .lock()
        .map_err(|_| AppError::Message("Download result mutex is poisoned.".to_string()))?;
    Ok(std::mem::take(&mut *results))
}

fn write_playlist(directory: &Path, tasks: &[DownloadTask]) -> AppResult<()> {
    let playlist_path = directory.join("list.m3u8");
    let mut playlist = File::create(playlist_path)?;

    writeln!(playlist, "#EXTM3U")?;

    for task in tasks {
        writeln!(playlist, "#EXTINF:-1, {}", task.holder)?;
        writeln!(playlist, "{}.mkv", task.video_id)?;
    }

    Ok(())
}

fn print_download_summary(successes: usize, failures: &[DownloadFailure]) {
    println!();
    println!(
        "\u{5b8c}\u{4e86}: {successes} \u{4ef6} / \u{5931}\u{6557}: {} \u{4ef6}",
        failures.len()
    );

    if failures.is_empty() {
        return;
    }

    println!("\u{5931}\u{6557}\u{4e00}\u{89a7}:");
    for failure in failures {
        println!(
            "- {} | {} ({})",
            failure.video_id,
            failure.holder.trim(),
            failure.reason
        );
    }
}

fn task_from_failure(failure: &DownloadFailure) -> DownloadTask {
    DownloadTask {
        index: failure.index,
        video_id: failure.video_id.clone(),
        holder: failure.holder.clone(),
    }
}

fn get(playlist_id: &str, key: &str, directory: &Path, parallelism: usize) -> AppResult<()> {
    fs::create_dir_all(directory)?;

    let yt_api = YoutubeApi::new(key);
    let playlist_items = yt_api.playlist_items(playlist_id)?;
    let ids: Vec<String> = playlist_items
        .into_iter()
        .map(|item| item.content_details.video_id)
        .collect();

    let tasks = collect_download_tasks(&yt_api, &ids)?;
    let total = tasks.len();
    write_playlist(directory, &tasks)?;

    let outcomes = download_videos_parallel(directory, tasks, parallelism, total, false)?;
    let mut successes = 0;
    let mut failures = Vec::new();

    for outcome in outcomes {
        match outcome {
            DownloadOutcome::Success => successes += 1,
            DownloadOutcome::Failure(failure) => failures.push(failure),
        }
    }

    if !failures.is_empty() {
        println!();
        println!(
            "\u{5f8c}\u{56de}\u{3057}: {} \u{4ef6}\u{3092}\u{518d}\u{8a66}\u{884c}\u{3057}\u{307e}\u{3059}",
            failures.len()
        );
        let retry_tasks: Vec<DownloadTask> = failures.iter().map(task_from_failure).collect();
        let retry_parallelism = parallelism.clamp(1, 2);
        let retry_outcomes =
            download_videos_parallel(directory, retry_tasks, retry_parallelism, total, true)?;
        failures.clear();

        for outcome in retry_outcomes {
            match outcome {
                DownloadOutcome::Success => successes += 1,
                DownloadOutcome::Failure(failure) => failures.push(failure),
            }
        }
    }

    print_download_summary(successes, &failures);

    Ok(())
}

fn print_usage() {
    println!("reel_system - ");
    println!(
        "A software written in Rust to download all videos in the playlist in .mkv format and to generate playlist file in .m3u8 format."
    );
    println!();
    println!("Usage: reel_system [Playlist URL] [API Key] [Directory] [Parallelism]");
    println!("API Key is optional when REEL_SYSTEM_API_KEY is set.");
    println!("Directory is optional. Default: current directory.");
    println!("Parallelism is optional. Default: available CPU count.");
}

fn default_parallelism() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(4)
}

fn parse_parallelism(input: Option<&String>) -> AppResult<usize> {
    match input {
        Some(value) if !value.trim().is_empty() => {
            let parallelism = value.parse::<usize>()?;
            if parallelism == 0 {
                return Err(Box::new(AppError::Message(
                    "Parallelism must be 1 or greater.".to_string(),
                )));
            }
            Ok(parallelism)
        }
        _ => Ok(default_parallelism()),
    }
}

fn parse_api_key(input: Option<&String>) -> AppResult<String> {
    if let Some(value) = input {
        if !value.trim().is_empty() {
            return Ok(value.clone());
        }
    }

    env::var(API_KEY_ENV).map_err(|_| {
        Box::new(AppError::Message(format!(
            "API key is missing. Pass it as an argument or set {API_KEY_ENV}."
        ))) as Box<dyn Error + Send + Sync>
    })
}

fn parse_directory(input: Option<&String>) -> AppResult<PathBuf> {
    if let Some(value) = input {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value));
        }
    }

    Ok(env::current_dir()?)
}

fn run() -> AppResult<()> {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            print_usage();
            Ok(())
        }
        2..=5 => {
            let playlist_id = playlist_url_to_id(&args[1])?;
            let parallelism = parse_parallelism(args.get(4))?;
            let api_key = parse_api_key(args.get(2))?;
            let directory = parse_directory(args.get(3))?;
            get(&playlist_id, &api_key, &directory, parallelism)
        }
        _ => Err(Box::new(AppError::Message(
            "Prompt Error: Arguments is incorrect.".to_string(),
        ))),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
