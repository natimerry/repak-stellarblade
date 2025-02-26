mod file_table;
mod install_mod;
mod utils;

use crate::egui::RichText;
use crate::file_table::FileTable;
use crate::install_mod::{map_dropped_file_to_mods, ModInstallRequest};
use eframe::egui::cache::FrameCache;
use eframe::egui::{
    self, style::Selection, Align, Button, CollapsingHeader, Color32, Frame, Grid, Label, Layout,
    ScrollArea, SelectableLabel, Stroke, Style, TextEdit, TextStyle, Theme, Ui, Visuals, Widget,
};
use egui_flex::{item, Flex, FlexAlign};
use log::{debug, error, info, warn};
use repak::PakReader;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;
use std::usize::MAX;
use std::{fs, io};
use repak::utils::AesKey;
use crate::utils::get_current_pak_characteristics;
// use eframe::egui::WidgetText::RichText;

#[derive(Deserialize, Serialize, Default)]
struct RepakModManager {
    game_path: PathBuf,
    default_font_size: f32,
    #[serde(skip)]
    current_pak_file_idx: Option<usize>,
    #[serde(skip)]
    pak_files: Vec<(PakReader, PathBuf)>,
    #[serde(skip)]
    table: Option<FileTable>,
    #[serde(skip)]
    dropped_files: Vec<egui::DroppedFile>,
    #[serde(skip)]
    file_drop_viewport_open: bool,
    #[serde(skip)]
    install_mod_dialog: Option<ModInstallRequest>,
}

fn use_dark_red_accent(style: &mut Style) {
    style.visuals.hyperlink_color = Color32::from_hex("#f71034").expect("Invalid color");
    style.visuals.text_cursor.stroke.color = Color32::from_hex("#941428").unwrap();
    style.visuals.selection = Selection {
        bg_fill: Color32::from_rgba_unmultiplied(241, 24, 14, 60),
        stroke: Stroke::new(1.0, Color32::from_hex("#000000").unwrap()),
    };

    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(241, 24, 14, 60);
}

pub fn setup_custom_style(ctx: &egui::Context) {
    ctx.style_mut_of(Theme::Dark, use_dark_red_accent);
    ctx.style_mut_of(Theme::Light, use_dark_red_accent);
}

fn set_custom_font_size(ctx: &egui::Context, size: f32) {
    let mut style = (*ctx.style()).clone();
    for (text_style, font_id) in style.text_styles.iter_mut() {
        match text_style {
            TextStyle::Small => {
                font_id.size = size - 4.;
            }
            TextStyle::Body => {
                font_id.size = size - 3.;
            }
            TextStyle::Monospace => {
                font_id.size = size;
            }
            TextStyle::Button => {
                font_id.size = size - 1.;
            }
            TextStyle::Heading => {
                font_id.size = size + 4.;
            }
            TextStyle::Name(_) => {
                font_id.size = size;
            }
        }
    }
    ctx.set_style(style);
}




impl RepakModManager {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_style(&cc.egui_ctx);
        let x = Self {
            game_path: PathBuf::new(),
            default_font_size: 18.0,
            pak_files: vec![],
            current_pak_file_idx: None,
            table: None,
            dropped_files: vec![],
            ..Default::default()
        };
        set_custom_font_size(&cc.egui_ctx, x.default_font_size);
        x
    }

    fn collect_pak_files(&mut self) {
        if !self.game_path.exists() {
            return;
        } else {
            let mut vecs = vec![];
            let aes_key = AesKey::from_str(
                "0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74",
            )
            .unwrap();
            for entry in std::fs::read_dir(self.game_path.clone()).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }
                if path.extension().unwrap_or_default() != "pak" {
                    continue;
                }
                let mut disabled = false;
                if path.extension().unwrap_or_default() == "pak_disabled" {
                    disabled = true;
                }
                let mut builder = repak::PakBuilder::new();
                builder = builder.key(aes_key.0.clone());
                let pak = builder
                    .reader(&mut BufReader::new(File::open(path.clone()).unwrap()))
                    .unwrap();

                vecs.push((pak, path));
            }
            self.pak_files = vecs;
        }
    }
    fn list_pak_contents(&mut self, ui: &mut egui::Ui) -> Result<(), repak::Error> {
        if let None = self.current_pak_file_idx {
            return Ok(());
        }
        let mut builder = repak::PakBuilder::new();
        let aes_key =
            AesKey::from_str("0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74")
                .unwrap();
        builder = builder.key(aes_key.0);
        let pak = &self.pak_files[self.current_pak_file_idx.unwrap()].0;

        let pak_path = self.pak_files[self.current_pak_file_idx.unwrap()].1.clone();

        ui.label("Files");
        ui.separator();
        ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut table = &mut self.table;
                if let Some(ref mut table) = table {
                    table.table_ui(ui);
                }
            });

        Ok(())
    }

    fn show_pak_details(&mut self, ui: &mut egui::Ui) {
        if let None = self.current_pak_file_idx {
            return;
        }
        use egui::{Label, RichText};
        let pak = &self.pak_files[self.current_pak_file_idx.unwrap()].0;
        let pak_path = self.pak_files[self.current_pak_file_idx.unwrap()].1.clone();
        let full_paths = pak.files().into_iter().collect::<Vec<_>>();

        ui.collapsing("Encryption details", |ui| {
            ui.horizontal(|ui| {
                ui.add(Label::new(RichText::new("Encryption: ").strong()));
                ui.add(Label::new(format!("{}", pak.encrypted_index())));
            });

            ui.horizontal(|ui| {
                ui.add(Label::new(RichText::new("Encryption GUID: ").strong()));
                ui.add(Label::new(format!("{:?}", pak.encryption_guid())));
            });
        });

        ui.collapsing("Pak details", |ui| {
            ui.horizontal(|ui| {
                ui.add(Label::new(RichText::new("Mount Point: ").strong()));
                ui.add(Label::new(format!("{}", pak.mount_point())));
            });

            ui.horizontal(|ui| {
                ui.add(Label::new(RichText::new("Path Hash Seed: ").strong()));
                ui.add(Label::new(format!("{:?}", pak.path_hash_seed())));
            });

            ui.horizontal(|ui| {
                ui.add(Label::new(RichText::new("Version: ").strong()));
                ui.add(Label::new(format!("{:?}", pak.version())));
            });
        });
        ui.horizontal(|ui| {
            ui.add(Label::new(
                RichText::new("Mod type: ")
                    .strong()
                    .size(self.default_font_size + 1.),
            ));
            ui.add(Label::new(format!(
                "{}",
                get_current_pak_characteristics(full_paths.clone())
            )));
        });
        if let None = self.table {
            self.table = Some(FileTable::new(pak, &pak_path));
        }
    }
    fn show_pak_files_in_dir(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    for (i, pak_file) in self.pak_files.iter().enumerate() {
                        let selected = true;
                        if let Some(_idx) = self.current_pak_file_idx {}
                        let pakfile = ui.selectable_label(
                            i == self.current_pak_file_idx.unwrap_or(MAX),
                            pak_file
                                .1
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                        );
                        if pakfile.clicked() {
                            self.current_pak_file_idx = Some(i);
                        }
                    }
                });
            });
    }
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("repak_manager");
        if !path.exists() {
            fs::create_dir_all(&path).unwrap();
            info!("Created config directory: {}", path.to_string_lossy());
        }
        path.push("repak_mod_manager.json");
        path
    }

    fn load(ctx: &eframe::CreationContext) -> std::io::Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            info!("Loading config: {}", path.to_string_lossy());
            let data = fs::read_to_string(path)?;
            let mut config: Self = serde_json::from_str(&data)?;
            set_custom_font_size(&ctx.egui_ctx, config.default_font_size);
            config.collect_pak_files();
            Ok(config)
        } else {
            info!(
                "First Launch creating new directory: {}",
                path.to_string_lossy()
            );
            Ok(Self::new(ctx)) // If config doesn't exist, return default settings
        }
    }
    fn save_state(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        let json = serde_json::to_string_pretty(self)?;
        info!("Saving config: {}", path.to_string_lossy());
        fs::write(path, json)?;
        Ok(())
    }

    /// Preview hovering files:
    fn preview_files_being_dropped(ctx: &egui::Context, rect: egui::Rect) {
        use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
        use std::fmt::Write as _;

        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(241, 24, 14, 40));
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Add new mod files here",
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }
    }
}
impl eframe::App for RepakModManager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(ref mut install_mod) = self.install_mod_dialog {
            if self.file_drop_viewport_open{

                install_mod.new_mod_dialog(&ctx,&mut self.file_drop_viewport_open);
            }
        }

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let dropped_files = i.raw.dropped_files.clone();
                debug!("Dropped files: {:?}", dropped_files);
                // Check if all files are either directories or have the .pak extension
                let all_valid = dropped_files.iter().all(|file| {
                    let path = file.path.clone().unwrap();
                    path.is_dir() || path.extension().map(|ext| ext == "pak").unwrap_or(false)
                });

                if all_valid {
                    if let None = self.table {
                        let mods = map_dropped_file_to_mods(&dropped_files);

                        if mods.is_empty() {
                            error!("No mods found in dropped files.");
                            return;
                        }
                        self.file_drop_viewport_open=true;
                        debug!("Mods: {:?}", mods);
                        self.install_mod_dialog = Some(ModInstallRequest { mods });
                    }
                } else {
                    // Handle the case where not all dropped files are valid
                    // You can show an error or prompt the user here
                    println!(
                        "Not all files are valid. Only directories or .pak files are allowed."
                    );
                }
            }
        });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                // ui.add_space(16.0);
                ui.menu_button("Settings", |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.default_font_size, 12.0..=32.0)
                            .text("Font size"),
                    );
                    set_custom_font_size(ui.ctx(), self.default_font_size);
                    ui.horizontal(|ui| {
                        let mode = match ui.ctx().style().visuals.dark_mode {
                            true => "Switch to light mode",
                            false => "Switch to dark mode",
                        };
                        ui.add(egui::Label::new(mode).halign(Align::Center));
                        egui::widgets::global_theme_preference_switch(ui);
                    });
                });
            });
            ui.separator();
            Flex::horizontal()
                .w_full()
                .align_items(FlexAlign::Center)
                .show(ui, |flex_ui| {
                    flex_ui.add(item(), Label::new("Mod folder:"));
                    flex_ui.add(
                        item().grow(1.0),
                        TextEdit::singleline(&mut self.game_path.to_string_lossy().to_string()),
                    );
                    let browse_button = flex_ui.add(item(), Button::new("Browse"));
                    if browse_button.clicked() {
                        if let Some(path) = FileDialog::new().pick_folder() {
                            self.game_path = path;
                            self.collect_pak_files();
                        }
                    }
                    flex_ui.add_ui(item(), |ui| {
                        let x =
                            ui.add_enabled(self.game_path.exists(), Button::new("Open mod folder"));
                        if x.clicked() {
                            println!("Opening mod folder: {}", self.game_path.to_string_lossy());
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("explorer")
                                    .arg(self.game_path.clone())
                                    .spawn();
                            }

                            #[cfg(target_os = "linux")]
                            {
                                debug!("Opening mod folder: {}", self.game_path.to_string_lossy());
                                let _ = std::process::Command::new("xdg-open")
                                    .arg(self.game_path.to_string_lossy().to_string())
                                    .spawn();
                            }
                        }
                    });
                });
            ui.separator();
        });

        egui::SidePanel::left("left_panel")
            .min_width(300.)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.set_height(ui.available_height());
                    ui.label("Mod files");
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height(ui.available_height() * 0.6);
                        Self::preview_files_being_dropped(&ctx, ui.available_rect_before_wrap());
                        self.show_pak_files_in_dir(ui);
                    });

                    ui.separator();

                    ui.label("Details");

                    ui.group(|ui| {
                        ui.set_height(ui.available_height());
                        ui.set_width(ui.available_width());
                        self.show_pak_details(ui);
                    });
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.list_pak_contents(ui).expect("TODO: panic message");
        });

        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_state().unwrap();
        }
    }
}

fn main() {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1366.0, 768.0])
            .with_min_inner_size([1280.0, 720.])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Repak GUI",
        options,
        Box::new(|cc| {
            cc.egui_ctx
                .style_mut(|style| style.visuals.dark_mode = true);
            Ok(Box::new(
                RepakModManager::load(cc).expect("Unable to load config"),
            ))
        }),
    )
    .expect("Unable to spawn windows");
}
