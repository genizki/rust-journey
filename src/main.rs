mod locale;
mod share;
mod ui;
mod worker;

use eframe::egui::{self, Button, Color32, CornerRadius, InnerResponse, Rect, Stroke, Ui, vec2};
use serde::{Deserialize, Serialize};

// laod .env variables
use dotenv::dotenv;
use std::hash::Hash;
use std::{env, f32};

// Api crates
use reqwest::Client;
use tokio::{self, io::AsyncBufReadExt}; //asynch

// const
pub const WIDTH: f32 = 120.0;
pub const HEIGHT: f32 = 120.0;

#[cfg(target_os = "macos")]
const _DOWNLOAD_PATH: &str = "~/Downloads";

#[cfg(target_os = "windows")]
const DOWNLOAD_PATH: &str = "%USERPROFILE%\\Downloads";

#[cfg(target_os = "windows")]
const YT_DLP_BINARY: &str = "./yt_dlp/yt-dlp.exe";

#[cfg(target_os = "macos")]
const YT_DLP_BINARY: &str = "./yt_dlp/yt-dlp_macos";

struct TokioWorker {
    tx: tokio::sync::mpsc::Sender<share::WorkerMessage>,
    rx: tokio::sync::mpsc::Receiver<share::WorkerMessage>,
}
impl Default for TokioWorker {
    fn default() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(102);
        Self { tx, rx }
    }
}

#[derive(Default)]
enum AppState {
    #[default]
    App,
    Settings,
    Warning,
    Test,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct SettingsState {
    max_results: i8,
    first_run: bool,
    download_path: String,
    personal_yt_api: String,
}
impl SettingsState {
    fn default() -> Self {
        Self {
            max_results: 8,
            first_run: true,
            download_path: _DOWNLOAD_PATH.to_string(),
            personal_yt_api: "".to_string(),
        }
    }
}

#[derive(Default)]
struct YtGUI {
    pub data: share::SearchResponse,
    pub search_item: Vec<share::SearchResponseMeta>,
    pub search_text: String,
    pub side_width: f32,
    pub settings_state: SettingsState,
    pub image_loader_installed: bool,
    pub app_state: AppState,
    pub tokio_worker: TokioWorker,
}

impl YtGUI {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings_state: SettingsState = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        Self {
            settings_state,
            ..Default::default()
        }
    }
    fn search_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::Frame::default().show(ui, |ui| {
            ui.style_mut().spacing.item_spacing = egui::Vec2 { x: 0.0, y: 0.0 };
            ui.vertical_centered(|ui| {
                ui.horizontal_top(|ui| {
                    // ui.add_space();
                    // println!("{}", ui.available_width());
                    let avaibale_width = ui.available_width();
                    let searchfield_width = avaibale_width * 0.40;
                    // let search_button_width = avaibale_width * 0.10;
                    // let spacing =
                    //     (avaibale_width - (searchfield_width + search_button_width)) / 2.0;

                    let spacing = (avaibale_width - (searchfield_width)) / 2.0;

                    ui.add_space(spacing);
                    let searchfield = ui.add(
                        egui::TextEdit::singleline(&mut self.search_text)
                            .hint_text("🔍")
                            .desired_width(searchfield_width)
                            .min_size(vec2(330.0, 20.0)),
                    );
                    // .on_hover_text("some random text");
                    // let search_button = ui.add(Button::new("🔍"));

                    // if searchfield.clicked() {
                    //     searchfield.request_focus();
                    // }
                    if !searchfield.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    // || search_button.clicked()
                    {
                        let search_string = self.search_text.clone();
                        let max_reults = self.settings_state.max_results.clone();
                        let rx = self.tokio_worker.tx.clone();
                        let ctx_giver = ctx.clone();

                        tokio::spawn(async move {
                            let mut data = call_yt_api(search_string, max_reults).await.unwrap();

                            let mut ex_video_ids: Vec<String> = Vec::new();
                            for item in &data.items {
                                if let Some(video_id) = &item.id.video_id {
                                    ex_video_ids.push(video_id.clone());
                                }
                            }
                            set_video_durration(ex_video_ids, &mut data).await.unwrap();

                            rx.send(share::WorkerMessage::Data(data)).await.unwrap();
                            // rx.send({ data })
                            ctx_giver.request_repaint();
                        });

                        self.search_text.clear();
                    }

                    ui.add_space(spacing);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.add(Button::new("⚙")).clicked() {
                            self.app_state = AppState::Settings;
                        }
                    });
                })
                .response;
                ui.allocate_space(vec2(ui.available_width(), 10.0));

                ui.add_space(40.0);
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            for (index, item) in &mut self.data.items.iter().enumerate() {
                                self.search_item.push(share::SearchResponseMeta {
                                    is_enabled: true,
                                    download_progress: 0,
                                });

                                if self.search_item[index].is_enabled {
                                    if result_widget(ui, index, |ui| {
                                        let scroll_bar: f32 = 10.0;
                                        ui.set_width(ui.available_width() - scroll_bar);
                                        ui.horizontal(|ui| {
                                            let thumbnail_url: &str = if let Some(ref thumb) =
                                                item.snippet.thumbnails.default
                                            {
                                                &thumb.url
                                            } else {
                                                "notfound"
                                            };

                                            let image = egui::Image::from_uri(thumbnail_url)
                                                .fit_to_exact_size(vec2(WIDTH, HEIGHT));
                                            ui.vertical(|ui| {
                                                ui.add(image);
                                                if let Some(duration) =
                                                    item.video_durration.as_ref()
                                                {
                                                    ui.label(duration);
                                                }
                                            });

                                            ui.add_space(40.0);
                                            ui.vertical(|ui| {
                                                ui.label(&item.snippet.title);
                                                ui.colored_label(
                                                    Color32::GRAY,
                                                    &item.snippet.channel_title,
                                                );
                                                ui.add_space(10.0);
                                            });
                                        });
                                    })
                                    .response
                                    .clicked()
                                    {
                                        self.search_item[index].is_enabled = false;
                                        if let Some(video_id) = &item.id.video_id {
                                            let yt_link = format!(
                                                "https://www.youtube.com/watch?v={}",
                                                video_id
                                            );
                                            println!("{}", &self.settings_state.download_path);
                                            let path = self.settings_state.download_path.clone();
                                            let yt_link = yt_link.clone(); // auch Borrow zu String machen!
                                            let item_id = index.clone();

                                            let tx = self.tokio_worker.tx.clone();
                                            tokio::spawn(async move {
                                                let error_handle = downlaod_from_dlp(
                                                    tx, item_id, &yt_link, &path, "aac",
                                                )
                                                .await;
                                                match error_handle {
                                                    Ok(()) => {}
                                                    Err(error) => {
                                                        eprintln!("download failed with: {error}")
                                                    }
                                                }
                                            });
                                        } else {
                                            println!("Fehler Video_id nicht gefunden. Think");
                                        }
                                        println!("test am Index: {index}");
                                    }
                                    ui.add_space(20.0);
                                }
                            }
                        });
                    ui.allocate_space(ui.available_size());
                });
            });
        });
    }
}

impl eframe::App for YtGUI {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.settings_state);
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.settings_state.first_run {
            global_fontsize(ctx);
            self.settings_state.first_run = false;
            self.settings_state.download_path = _DOWNLOAD_PATH.to_string();
        }
        if "" == self.settings_state.personal_yt_api {
            self.app_state = AppState::Warning;
        }
        let screen_rect = ctx.screen_rect();
        let panel_size = calc_grid_size(&screen_rect, None);
        self.side_width = panel_size.side_width;

        //needed to display images
        if !self.image_loader_installed {
            egui_extras::install_image_loaders(ctx);
            self.image_loader_installed = true
        }

        if let Ok(msg) = self.tokio_worker.rx.try_recv() {
            match msg {
                share::WorkerMessage::Done(index) => {
                    self.search_item[index].is_enabled = true;
                }
                share::WorkerMessage::Progress(progress_value) => {}
                share::WorkerMessage::Error(error_msg) => {}
                share::WorkerMessage::Data(data) => {
                    self.data = data;
                }
            }
        }

        // if !check_setup_ok() {
        //     self.app_state = AppState::Warning;
        // }
        // UI entry point =>

        match self.app_state {
            AppState::App => {
                layout(self.side_width, ctx, |ui| self.search_bar(ctx, ui), false);
            }
            AppState::Settings => {
                layout(
                    self.side_width,
                    ctx,
                    |ui| {
                        egui::Grid::new("settings_header")
                            .num_columns(3)
                            .spacing([ui.available_width() / 3.0, 0.0])
                            .show(ui, |ui| {
                                let av_space = ui.available_width();
                                let spacer = av_space / 2.0;
                                if ui.button("back").clicked() {
                                    self.app_state = AppState::App;
                                }
                                // let button_size = av_space - ui.available_width();
                                // let button_spacer = spacer - button_size;

                                // ui.add_space(spacer - button_size);
                                // ui.label("settings");
                                // ui.add_space(spacer);
                                ui.end_row();
                            });
                        ui.label("settings");
                        ui.add_space(40.0);
                        ui.add(egui::Slider::new(
                            &mut self.settings_state.max_results,
                            0..=25,
                        ));
                        if ui.button("delete Api key").clicked() {
                            self.settings_state.personal_yt_api = "".to_string();
                        }
                        if ui.button("press me").clicked() {
                            let output = std::process::Command::new("pwd").output();
                            println!("{:?}", output);
                        }
                        if ui.button("test me").clicked() {
                            let output = std::process::Command::new(YT_DLP_BINARY)
                                .arg("--version")
                                .output();
                            println!("{:?}", output);
                        }
                    },
                    true,
                );
            }
            AppState::Warning => layout(
                self.side_width,
                ctx,
                |ui| {
                    ui.label("Warning no api Key found. Make sure you enter your Youtube API Key in here!");
                    if !ui
                        .add(
                            egui::TextEdit::singleline(&mut self.settings_state.personal_yt_api)
                                .hint_text("paste your api key"),
                        )
                        .has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        println!("{}", self.settings_state.personal_yt_api);
                        self.app_state = AppState::App;
                    }
                },
                false,
            ),
            AppState::Test => {
                layout(
                    self.side_width,
                    ctx,
                    |ui| {
                        if ui.button("yt_dlp me").clicked() {
                            let args = [
                                "-x",
                                "--audio-format",
                                "aac",
                                "-o",
                                "~/Downloads/%(title)s.%(ext)s",
                                "--add-metadata",
                                "https://www.youtube.com/watch?v=5kfPCxXZPdA",
                                "--ffmpeg-location",
                                "./ffmpeg/ffmpeg",
                            ];
                            let output = std::process::Command::new(YT_DLP_BINARY)
                                .args(&args)
                                .output()
                                .expect("Failed halt");
                            println!("{:?}", output);
                            println!("Exit status: {}", output.status);

                            // stdout (normale Ausgabe)
                            let stdout_str = String::from_utf8_lossy(&output.stdout);
                            if !stdout_str.trim().is_empty() {
                                println!("stdout:\n{}", stdout_str);
                            }

                            // stderr (Fehlermeldung)
                            let stderr_str = String::from_utf8_lossy(&output.stderr);
                            if !stderr_str.trim().is_empty() {
                                println!("stderr:\n{}", stderr_str);
                            }
                        }
                        if ui.button("test").clicked() {
                            println!("test clicked");
                            tokio::spawn(async move {
                                test_io().await;
                            });
                        }
                    },
                    true,
                );
            }
        }
    }
}

fn global_fontsize(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        style.text_styles = [
            (
                egui::TextStyle::Heading,
                egui::FontId::new(32.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(22.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(22.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(20.0, egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            ),
        ]
        .into();
    });
}

fn calc_grid_size(screen_rect: &Rect, scaling_factor: Option<f32>) -> PanelSize {
    const WIDTH_THRESHOLD: f32 = 1000.0;
    // let screen_min = screen_rect.min;

    let screen_max = screen_rect.max;
    let mut side_width: f32 = 0.0;
    let mut central_width: f32;
    let max_width: f32 = screen_max.x;

    central_width = max_width;

    if central_width >= WIDTH_THRESHOLD {
        side_width = (max_width - WIDTH_THRESHOLD) / scaling_factor.unwrap_or(2.5);
        central_width = central_width - side_width;
    }

    // println!("central:{central_width}, side: {side_width}");
    // let right_side = left_side.clone();
    PanelSize {
        side_width,
        _central_width: central_width,
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok(); // Enviroment variablen aus der .env laden
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder {
            // title: (),
            // app_id: (),
            // position: (),
            // inner_size: (),
            min_inner_size: Some(egui::vec2(800.0, 600.0)),
            // max_inner_size: (),
            // clamp_size_to_monitor_size: (),
            // fullscreen: (),
            // maximized: (),
            // resizable: (),
            // transparent: (),
            // decorations: (),
            // icon: (),
            // active: (),
            // visible: (),
            // fullsize_content_view: (),
            // movable_by_window_background: (),
            // title_shown: (),
            // titlebar_buttons_shown: (),
            // titlebar_shown: (),
            // has_shadow: (),
            // drag_and_drop: (),
            // taskbar: (),
            // close_button: (),
            // minimize_button: (),
            // maximize_button: (),
            // window_level: (),
            // mouse_passthrough: (),
            // window_type: (),
            ..Default::default()
        },
        ..Default::default()
    };

    let app = eframe::run_native(
        "Hier cooler Name 2",
        options,
        Box::new(|cc| Ok(Box::new(YtGUI::new(cc)))),
    );
    if let Err(error) = app {
        eprint!("Fehler beim Starten der App: {}", error);
    }
}

fn layout<Central>(
    side_width: f32,
    ctx: &egui::Context,
    // leftside_content: egui::Response,
    // rightside_content: egui::Response,
    central_content: Central,
    dev_mode: bool,
) where
    Central: FnOnce(&mut egui::Ui),
{
    egui::SidePanel::left(egui::Id::new("left_side"))
        .exact_width(side_width)
        .frame(egui::Frame::default().fill(ctx.style().visuals.panel_fill))
        .show_separator_line(dev_mode)
        .resizable(false)
        .show(ctx, |_ui| {});

    egui::SidePanel::right(egui::Id::new("right_side"))
        .exact_width(side_width)
        .frame(egui::Frame::default().fill(ctx.style().visuals.panel_fill))
        .show_separator_line(dev_mode)
        .resizable(false)
        .show(ctx, |_ui| {});

    egui::CentralPanel::default().show(ctx, |_ui| {
        central_content(_ui);
    });
}

async fn call_yt_api(
    query: String,
    max_results: i8,
) -> Result<share::SearchResponse, Box<dyn std::error::Error>> {
    // let yt_key: String = env::var("YT_API").unwrap();
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
        let data: share::SearchResponse = response.json::<share::SearchResponse>().await?;
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

async fn set_video_durration(
    video_id: Vec<String>,
    meta_data: &mut share::SearchResponse,
) -> Result<(), Box<dyn std::error::Error>> {
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

struct PanelSize {
    side_width: f32,
    _central_width: f32,
}

async fn downlaod_from_dlp(
    rx: tokio::sync::mpsc::Sender<share::WorkerMessage>,
    item_id: usize,
    url: &String,
    download_path: &String,
    audio_format: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    let download_string = format!("{download_path}/%(title)s.%(ext)s");

    let command = [
        "-x",
        "--audio-format",
        { &audio_format },
        "-o",
        { &download_string },
        "--add-metadata",
        "--embed-thumbnail",
        "--ffmpeg-location",
        "./ffmpeg/ffmpeg",
        "--progress-template",
        "download:%(progress)j",
        "--progress-template",
        "postprocess:%(progress)j",
        { &url },
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
            // let progress: serde_json::Value = serde_json::from_str(&line)?;
            // println!("{}", progress);
        }
    }
    if let Some(stderr) = output.stderr.take() {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            println!("{line}");
        }
    }

    rx.send(share::WorkerMessage::Done(item_id)).await.unwrap();
    Ok(())
}

fn result_widget<R>(
    ui: &mut Ui,
    id: impl Hash,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.push_id(id, |ui| {
        let frame = egui::Frame::new()
            .fill(Color32::from_gray(30))
            .corner_radius(8)
            .inner_margin(10.0)
            .stroke(egui::Stroke::new(1.0, Color32::from_gray(60)));

        let inner_response = frame.show(ui, |ui| add_contents(ui));
        let rect = inner_response.response.rect;
        let response = ui.interact(rect, ui.id(), egui::Sense::click());

        if response.hovered() {
            ui.painter()
                .rect_filled(rect, 8, Color32::from_black_alpha(30));
        }
        InnerResponse::new(inner_response.inner, response)
    })
    .inner
}

async fn test_io() -> Result<(), Box<dyn std::error::Error>> {
    println!("starting test_io");
    let mut child = tokio::process::Command::new("ping")
        .arg("google.com")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    // Stdout lesen
    if let Some(stdout) = child.stdout.take() {
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            println!("STDOUT: {}", line);
        }
    }
    Ok(())
}

fn check_setup_ok() -> bool {
    // check for env:
    if let Ok(_api_key) = env::var("YT_API") {
        true
    } else {
        false
    }
}
