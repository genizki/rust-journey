use crate::share::*;
use crate::worker::{call_yt_api, download_from_dlp, set_video_durration, test_io};
use eframe::egui::{self, Button, Color32, InnerResponse, Rect, Ui, vec2};
use std::hash::Hash;

pub struct YtGUI {
    pub data: SearchResponse,
    pub search_item: Vec<SearchResponseMeta>,
    pub search_text: String,
    pub side_width: f32,
    pub settings_state: SettingsState,
    pub image_loader_installed: bool,
    pub app_state: AppState,
    pub tokio_worker: TokioWorker,
}

impl Default for YtGUI {
    fn default() -> Self {
        Self {
            data: SearchResponse::default(),
            search_item: Vec::new(),
            search_text: String::new(),
            side_width: 0.0,
            settings_state: SettingsState::default(),
            image_loader_installed: false,
            app_state: AppState::default(),
            tokio_worker: TokioWorker::default(),
        }
    }
}

impl YtGUI {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings_state: SettingsState = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        Self {
            settings_state,
            ..Default::default()
        }
    }

    pub fn search_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::Frame::default().show(ui, |ui| {
            ui.style_mut().spacing.item_spacing = egui::Vec2 { x: 0.0, y: 0.0 };
            ui.vertical_centered(|ui| {
                ui.horizontal_top(|ui| {
                    let avaibale_width = ui.available_width();
                    let searchfield_width = avaibale_width * 0.40;
                    let spacing = (avaibale_width - (searchfield_width)) / 2.0;

                    ui.add_space(spacing);
                    let searchfield = ui.add(
                        egui::TextEdit::singleline(&mut self.search_text)
                            .hint_text("🔍")
                            .desired_width(searchfield_width)
                            .min_size(vec2(330.0, 20.0)),
                    );

                    if !searchfield.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
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

                            rx.send(WorkerMessage::Data(data)).await.unwrap();
                            ctx_giver.request_repaint();
                        });

                        self.search_text.clear();
                    }

                    let settings_button_margin: f32 = 10.0;
                    ui.add_space(spacing - settings_button_margin);
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
                                self.search_item.push(SearchResponseMeta {
                                    is_enabled: true,
                                    download_progress: 0,
                                });

                                if result_widget(
                                    ui,
                                    self.search_item[index].is_enabled,
                                    index,
                                    |ui| {
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
                                    },
                                )
                                .response
                                .clicked()
                                    && self.search_item[index].is_enabled
                                {
                                    self.search_item[index].is_enabled = false;
                                    if let Some(video_id) = &item.id.video_id {
                                        let yt_link =
                                            format!("https://www.youtube.com/watch?v={}", video_id);
                                        println!("{}", &self.settings_state.download_path);
                                        let path = self.settings_state.download_path.clone();
                                        let yt_link = yt_link.clone();
                                        let item_id = index.clone();

                                        let tx = self.tokio_worker.tx.clone();
                                        tokio::spawn(async move {
                                            let error_handle = download_from_dlp(
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
                        });
                    ui.allocate_space(ui.available_size());
                });
            });
        });
    }

    pub fn render_settings(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("settings_header")
            .num_columns(3)
            .spacing([ui.available_width() / 3.0, 0.0])
            .show(ui, |ui| {
                if ui.button("back").clicked() {
                    self.app_state = AppState::App;
                }
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
    }

    pub fn render_warning(&mut self, ui: &mut egui::Ui) {
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
    }

    pub fn render_test(&mut self, ui: &mut egui::Ui) {
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

            let stdout_str = String::from_utf8_lossy(&output.stdout);
            if !stdout_str.trim().is_empty() {
                println!("stdout:\n{}", stdout_str);
            }

            let stderr_str = String::from_utf8_lossy(&output.stderr);
            if !stderr_str.trim().is_empty() {
                println!("stderr:\n{}", stderr_str);
            }
        }
        if ui.button("test").clicked() {
            println!("test clicked");
            tokio::spawn(async move {
                let _ = test_io().await;
            });
        }
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
            self.settings_state.download_path = DOWNLOAD_PATH.to_string();
        }
        if "" == self.settings_state.personal_yt_api {
            self.app_state = AppState::Warning;
        }
        let screen_rect = ctx.screen_rect();
        let panel_size = calc_grid_size(&screen_rect, None);
        self.side_width = panel_size.side_width;

        if !self.image_loader_installed {
            egui_extras::install_image_loaders(ctx);
            self.image_loader_installed = true
        }

        if let Ok(msg) = self.tokio_worker.rx.try_recv() {
            match msg {
                WorkerMessage::Done(index) => {
                    self.search_item[index].is_enabled = true;
                }
                WorkerMessage::Progress(_progress_value) => {}
                WorkerMessage::Error(_error_msg) => {}
                WorkerMessage::Data(data) => {
                    self.data = data;
                }
            }
        }

        match self.app_state {
            AppState::App => {
                layout(self.side_width, ctx, |ui| self.search_bar(ctx, ui), false);
            }
            AppState::Settings => {
                layout(self.side_width, ctx, |ui| self.render_settings(ui), true);
            }
            AppState::Warning => {
                layout(self.side_width, ctx, |ui| self.render_warning(ui), false);
            }
            AppState::Test => {
                layout(self.side_width, ctx, |ui| self.render_test(ui), true);
            }
        }
    }
}

pub fn global_fontsize(ctx: &egui::Context) {
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

pub fn calc_grid_size(screen_rect: &Rect, scaling_factor: Option<f32>) -> PanelSize {
    const WIDTH_THRESHOLD: f32 = 1000.0;

    let screen_max = screen_rect.max;
    let mut side_width: f32 = 0.0;
    let mut central_width: f32;
    let max_width: f32 = screen_max.x;

    central_width = max_width;

    if central_width >= WIDTH_THRESHOLD {
        side_width = (max_width - WIDTH_THRESHOLD) / scaling_factor.unwrap_or(2.5);
        central_width = central_width - side_width;
    }

    PanelSize {
        side_width,
        _central_width: central_width,
    }
}

pub fn layout<Central>(
    side_width: f32,
    ctx: &egui::Context,
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

    egui::CentralPanel::default().show(ctx, |ui| {
        central_content(ui);
    });
}

pub fn result_widget<R>(
    ui: &mut Ui,
    button_state: bool,
    id: impl Hash,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.push_id(id, |ui| {
        let frame = egui::Frame::new()
            .fill(Color32::from_black_alpha(30))
            .corner_radius(8)
            .inner_margin(10.0)
            .stroke(egui::Stroke::new(1.0, Color32::from_black_alpha(60)));

        let inner_response = frame.show(ui, |ui| add_contents(ui));
        let rect = inner_response.response.rect;
        let response = ui.interact(rect, ui.id(), egui::Sense::click());
        if button_state {
            if response.hovered() {
                ui.painter()
                    .rect_filled(rect, 8, Color32::from_black_alpha(30));
            }
        }
        InnerResponse::new(inner_response.inner, response)
    })
    .inner
}
