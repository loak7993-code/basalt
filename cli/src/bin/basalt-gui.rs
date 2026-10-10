//! Basalt GUI: a small launcher window over the same engine flows the CLI
//! uses. Account, install with live progress, version list, play.

use eframe::egui;

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x34, 0xD3, 0x99);
const CANVAS: egui::Color32 = egui::Color32::from_rgb(0x05, 0x05, 0x05);
const PANEL: egui::Color32 = egui::Color32::from_rgb(0x10, 0x10, 0x12);
const TEXT: egui::Color32 = egui::Color32::from_rgb(0xF4, 0xF4, 0xF5);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(0xA7, 0xA9, 0xAE);

enum Job {
    Installing(Receiver<InstallEvent>),
}

enum InstallEvent {
    Log(String),
    Progress(ByteProgress),
    Done(Result<String, Vec<String>>),
}

use basalt_cli::download::ByteProgress;

#[derive(Default, Clone, Copy)]
struct ProgressState {
    done: usize,
    total: usize,
    bytes_done: u64,
    bytes_total: u64,
}

#[derive(Default)]
struct App {
    account_name: String,
    version_input: String,
    install_log: Vec<String>,
    progress: ProgressState,
    job: Option<Job>,
    versions: Vec<String>,
}

fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("BASALT_DATA") {
        return PathBuf::from(dir);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".local/share/basalt")
}

fn versions_dir() -> PathBuf {
    data_dir().join("versions")
}

fn accounts_file() -> PathBuf {
    data_dir().join("accounts.json")
}

fn stored_account() -> String {
    std::fs::read_to_string(accounts_file())
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| {
            v.get("playerName")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
}

fn scan_versions() -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(versions_dir()) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().join(format!("{name}.jar")).exists() {
                found.push(name);
            }
        }
    }
    found.sort();
    found.reverse();
    found
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        for (family, key, name) in [
            (egui::FontFamily::Proportional, "geist-regular", "Geist-Regular"),
            (egui::FontFamily::Proportional, "geist-medium", "Geist-Medium"),
            (egui::FontFamily::Proportional, "geist-semibold", "Geist-SemiBold"),
        ] {
            let data = std::fs::read(font_path(name)).unwrap_or_default();
            if !data.is_empty() {
                fonts.font_data.insert(
                    key.into(),
                    std::sync::Arc::new(egui::FontData::from_owned(data)),
                );
                fonts.families.entry(family).or_default().push(key.into());
            }
        }
        if let Ok(data) = std::fs::read(font_path("GeistMono-Regular")) {
            if !data.is_empty() {
                fonts.font_data.insert(
                    "geist-mono".into(),
                    std::sync::Arc::new(egui::FontData::from_owned(data)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("geist-mono".into());
            }
        }
        cc.egui_ctx.set_fonts(fonts);

        let account = stored_account();
        let versions = scan_versions();
        Self {
            account_name: account,
            version_input: String::new(),
            install_log: Vec::new(),
            progress: ProgressState::default(),
            job: None,
            versions,
        }
    }

    fn start_install(&mut self, version: Option<String>) {
        let (tx, rx) = channel::<InstallEvent>();
        let progress_tx = tx.clone();
        let progress = Arc::new(move |line: String| {
            let _ = progress_tx.send(InstallEvent::Log(line));
        });
        let bytes_tx = tx.clone();
        let bytes_cb = Arc::new(move |p: ByteProgress| {
            let _ = bytes_tx.send(InstallEvent::Progress(p));
        });
        let done_tx = tx.clone();
        std::thread::spawn(move || {
            let result =
                basalt_cli::install::run_flow(version.as_deref(), progress, bytes_cb);
            let _ = done_tx.send(InstallEvent::Done(result));
        });
        self.install_log.clear();
        self.progress = ProgressState::default();
        self.job = Some(Job::Installing(rx));
    }

    fn start_play(&mut self, version: String) {
        let (tx, rx) = channel::<InstallEvent>();
        let sink_tx = tx.clone();
        let done_tx = tx.clone();
        std::thread::spawn(move || {
            let sink = Arc::new(move |line: String| {
                let _ = sink_tx.send(InstallEvent::Log(line));
            });
            let result = basalt_cli::launch::run_flow(
                Some(&version),
                None,
                None,
                &[],
                None,
                sink,
            )
            .map(|code| format!("game exited with code {code}"))
            .map_err(|message| vec![message]);
            let _ = done_tx.send(InstallEvent::Done(result));
        });
        self.install_log.clear();
        self.job = Some(Job::Installing(rx));
    }
}

fn font_path(name: &str) -> PathBuf {
    // Bundled next to the binary by the AppImage; falls back to the repo layout
    // for development runs.
    for base in [
        std::env::var("APPDIR").map(PathBuf::from).unwrap_or_default().join("usr/share/fonts/basalt"),
        std::env::var("CARGO_MANIFEST_DIR").map(PathBuf::from).unwrap_or_default().join("assets/fonts"),
    ] {
        let path = base.join(name);
        let _ = &path;
        if path.exists() {
            return path;
        }
        let with_ext = base.join(format!("{name}.ttf"));
        if with_ext.exists() {
            return with_ext;
        }
    }
    PathBuf::from(name)
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain job events.
        if let Some(Job::Installing(rx)) = &self.job {
            loop {
                match rx.try_recv() {
                    Ok(InstallEvent::Log(line)) => {
                        self.install_log.push(line);
                        if self.install_log.len() > 200 {
                            self.install_log.drain(0..self.install_log.len() - 200);
                        }
                    }
                    Ok(InstallEvent::Progress(p)) => {
                        self.progress = ProgressState {
                            done: p.done,
                            total: p.total,
                            bytes_done: p.bytes_done,
                            bytes_total: p.bytes_total,
                        };
                    }
                    Ok(InstallEvent::Done(result)) => {
                        match result {
                            Ok(message) => self.install_log.push(message),
                            Err(lines) => self.install_log.extend(lines),
                        }
                        self.versions = scan_versions();
                        self.job = None;
                        break;
                    }
                    Err(_) => break,
                }
            }
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(CANVAS)
                    .inner_margin(egui::Margin::symmetric(16, 14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(ACCENT, "Basalt");
                    ui.weak("Minecraft Java Edition launcher");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(TEXT_DIM, format!("v{}", env!("CARGO_PKG_VERSION")));
                    });
                });
                ui.add_space(10.0);

                // Account row.
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.strong("Account");
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            let name_field = ui.add_sized(
                                [ui.available_width() - 110.0, 26.0],
                                egui::TextEdit::singleline(&mut self.account_name)
                                    .hint_text("offline account name"),
                            );
                            if ui
                                .add_sized([96.0, 26.0], egui::Button::new("Save"))
                                .clicked()
                                || name_field.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                let name = self.account_name.trim().to_string();
                                if !name.is_empty() {
                                    let _ = std::fs::create_dir_all(data_dir());
                                    let uuid = basalt_cli::accounts::offline_uuid(&name);
                                    let json = serde_json::json!({
                                        "playerName": name, "uuid": uuid
                                    });
                                    let _ = std::fs::write(
                                        accounts_file(),
                                        serde_json::to_string_pretty(&json).unwrap(),
                                    );
                                }
                            }
                        });
                    });
                });
                ui.add_space(8.0);

                // Versions + play.
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.strong("Versions");
                        ui.add_space(2.0);
                        if self.versions.is_empty() {
                            ui.colored_label(TEXT_DIM, "No versions installed. Install one below.");
                        } else {
                            for version in self.versions.clone() {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&version).monospace());
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if self.job.is_some() {
                                                ui.weak("busy");
                                            } else if ui
                                                .add(egui::Button::new(
                                                    egui::RichText::new("Play").color(CANVAS),
                                                ))
                                                .clicked()
                                            {
                                                self.start_play(version.clone());
                                            }
                                        },
                                    );
                                });
                            }
                        }
                    });
                });
                ui.add_space(8.0);

                // Install row.
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.strong("Install from the official manifest");
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            let field = ui.add_sized(
                                [ui.available_width() - 130.0, 26.0],
                                egui::TextEdit::singleline(&mut self.version_input)
                                    .hint_text("version id, e.g. 26.3 (empty = latest release)"),
                            );
                            let install_clicked = ui
                                .add_sized(
                                    [116.0, 26.0],
                                    egui::Button::new(
                                        egui::RichText::new(if self.job.is_some() {
                                            "Working..."
                                        } else {
                                            "Install"
                                        })
                                        .color(CANVAS),
                                    ),
                                )
                                .clicked();
                            if install_clicked && self.job.is_none() {
                                let version = self.version_input.trim().to_string();
                                let version = if version.is_empty() { None } else { Some(version) };
                                self.start_install(version);
                            }
                            let _ = field;
                        });
                        if self.job.is_some() && self.progress.total > 0 {
                            ui.add_space(6.0);
                            let fraction = if self.progress.bytes_total > 0 {
                                self.progress.bytes_done as f32 / self.progress.bytes_total as f32
                            } else {
                                self.progress.done as f32 / self.progress.total.max(1) as f32
                            };
                            ui.add(
                                egui::ProgressBar::new(fraction.clamp(0.0, 1.0))
                                    .show_percentage()
                                    .desired_height(14.0),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}/{} files · {} / {}",
                                    self.progress.done,
                                    self.progress.total,
                                    basalt_cli::download::human_bytes(self.progress.bytes_done),
                                    basalt_cli::download::human_bytes(self.progress.bytes_total),
                                ))
                                .monospace()
                                .size(11.5)
                                .color(TEXT_DIM),
                            );
                        }
                    });
                });
                ui.add_space(8.0);

                // Log pane.
                egui::Frame::new()
                    .fill(PANEL)
                    .corner_radius(6.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_min_height(140.0);
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                if self.install_log.is_empty() {
                                    ui.colored_label(TEXT_DIM, "Log will appear here.");
                                } else {
                                    for line in &self.install_log {
                                        ui.label(
                                            egui::RichText::new(line)
                                                .monospace()
                                                .size(12.5)
                                                .color(TEXT),
                                        );
                                    }
                                }
                            });
                    });
            });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 640.0])
            .with_min_inner_size([460.0, 520.0])
            .with_title("Basalt"),
        ..Default::default()
    };
    eframe::run_native(
        "Basalt",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
