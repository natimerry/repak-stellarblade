use crate::egui::RichText;
use eframe::egui::cache::FrameCache;
use eframe::egui::{
    self, style::Selection, Align, Button, CollapsingHeader, Color32, Frame, Grid, Label, Layout,
    ScrollArea, SelectableLabel, Stroke, Style, TextEdit, TextStyle, Theme, Visuals, Widget,
};
use egui_flex::{item, Flex, FlexAlign};
use log::debug;
use repak::PakReader;
use rfd::FileDialog;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;
use std::usize::MAX;
// use eframe::egui::WidgetText::RichText;

struct RepakModManager {
    game_path: PathBuf,
    default_font_size: f32,
    current_pak_file_idx: Option<usize>,
    pak_files: Vec<(PakReader, PathBuf)>,
}
fn use_dark_red_accent(style: &mut Style) {
    style.visuals.hyperlink_color = Color32::from_hex("#f71034").expect("Invalid color");
    style.visuals.text_cursor.stroke.color = Color32::from_hex("#941428").unwrap();
    style.visuals.selection = Selection {
        bg_fill: Color32::from_rgba_unmultiplied(241, 24, 14, 60),
        stroke: Stroke::new(1.0, Color32::from_hex("#000000").unwrap()),
    };
}

fn setup_custom_style(ctx: &egui::Context) {
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

#[derive(Debug, Clone)]
struct AesKey(aes::Aes256);
impl std::str::FromStr for AesKey {
    type Err = repak::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use aes::cipher::KeyInit;
        use base64::{engine::general_purpose, Engine as _};
        let try_parse = |mut bytes: Vec<_>| {
            bytes.chunks_mut(4).for_each(|c| c.reverse());
            aes::Aes256::new_from_slice(&bytes).ok().map(AesKey)
        };
        hex::decode(s.strip_prefix("0x").unwrap_or(s))
            .ok()
            .and_then(try_parse)
            .or_else(|| {
                general_purpose::STANDARD_NO_PAD
                    .decode(s.trim_end_matches('='))
                    .ok()
                    .and_then(try_parse)
            })
            .ok_or(repak::Error::Aes)
    }
}
fn get_current_pak_characteristics(mod_contents: Vec<String>) -> String {
    let character_map: HashMap<&str, &str> = [
        ("1011", "Hulk"),
        ("1014", "Punisher"),
        ("1015", "Storm"),
        ("1016", "Loki"),
        ("1018", "Dr.Strange"),
        ("1020", "Mantis"),
        ("1021", "Hawkeye"),
        ("1022", "Captain America"),
        ("1023", "Raccoon"),
        ("1024", "Hela"),
        ("1025", "CND"),
        ("1026", "Black Panther"),
        ("1027", "Groot"),
        ("1029", "Magik"),
        ("1030", "Moonknight"),
        ("1031", "Luna Snow"),
        ("1032", "Squirrel Girl"),
        ("1033", "Black Widow"),
        ("1034", "Iron Man"),
        ("1035", "Venom"),
        ("1036", "Spider Man"),
        ("1037", "Magneto"),
        ("1038", "Scarlet Witch"),
        ("1039", "Thor"),
        ("1040", "Mr Fantastic"),
        ("1041", "Winter Soldier"),
        ("1042", "Peni Parker"),
        ("1043", "Starlord"),
        ("1045", "Namor"),
        ("1046", "Adam Warlock"),
        ("1047", "Jeff"),
        ("1048", "Psylocke"),
        ("1049", "Wolverine"),
        ("1050", "Invisible Woman"),
        ("1052", "Iron Fist"),
        ("4017", "Announcer (Galacta)"),
        ("8021", "Loki's extra yapping"),
        ("8031", "Random NPCs"),
        ("8032", "Random NPCs"),
        ("8041", "Random NPCs"),
        ("8042", "Random NPCs"),
        ("8043", "Random NPCs"),
        ("8063", "Male NPC"),
    ]
    .iter()
    .cloned()
    .collect();

    for file in &mod_contents {
        if let Some(stripped) = file.strip_prefix("Marvel/Content/Marvel/") {
            let category = stripped.split('/').into_iter().next().unwrap_or_default();

            if category == "Characters" {
                // Extract the ID from the file path
                let parts: Vec<&str> = stripped.split('/').collect();
                if parts.len() > 1 {
                    let id = parts[1]; // Assuming ID is in second position
                    if let Some(character_name) = character_map.get(id) {
                        return format!("Character ({})", character_name);
                    }
                }
                return "Character (Unknown)".to_string();
            } else if category == "UI" {
                return "UI".to_string();
            }
        }
    }
    "Unknown".to_string()
}

impl RepakModManager {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_style(&cc.egui_ctx);

        let x = Self {
            game_path: PathBuf::new(),
            default_font_size: 18.0,
            pak_files: vec![],
            current_pak_file_idx: None,
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
    fn list_pak_files(&self, ui: &mut egui::Ui) -> Result<(), repak::Error> {
        if let None = self.current_pak_file_idx {
            return Ok(());
        }
        let mut builder = repak::PakBuilder::new();
        let aes_key =
            AesKey::from_str("0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74")
                .unwrap();
        builder = builder.key(aes_key.0);
        let pak = &self.pak_files[self.current_pak_file_idx.unwrap()].0;
        let full_paths = pak.files().into_iter().collect::<Vec<_>>();

        ui.vertical(|ui| {
            ui.label("Files");
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, file) in full_paths.iter().enumerate() {
                        Frame::default()
                            .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke) // Border style
                            .outer_margin(egui::Margin::same(1)) // Space around the frame
                            .inner_margin(egui::Margin::same(1)) // Space inside the frame
                            .show(ui, |ui| {
                                ui.style_mut().override_text_style = Some(egui::TextStyle::Small);

                                ui.add(SelectableLabel::new(false, file.to_string()));
                                // Selectable text
                            });
                    }
                });
        });

        Ok(())
    }

    fn show_pak_details(&mut self, ui: &mut egui::Ui) {
        if let None = self.current_pak_file_idx {
            return;
        }
        use egui::{Label, RichText};
        let pak = &self.pak_files[self.current_pak_file_idx.unwrap()].0;
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

            ui.horizontal(|ui| {
                ui.add(Label::new(RichText::new("Mod type: ").strong()));
                ui.add(Label::new(format!(
                    "{}",
                    get_current_pak_characteristics(full_paths.clone())
                )));
            });
        });
    }
    fn show_pak_files_in_dir(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    for (i, pak_file) in self.pak_files.iter().enumerate() {
                        let selected = true;
                        if let Some(idx) = self.current_pak_file_idx {}
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
}

impl eframe::App for RepakModManager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                        ui.label(mode);
                        egui::widgets::global_theme_preference_switch(ui);
                    });
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("RepakRivals Mod Manager");
            });
            ui.separator();
            ui.horizontal(|ui| {
                Flex::horizontal().w_full().show(ui, |flex_ui| {
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
                })
            });
            ui.separator();

            Flex::horizontal().w_auto().h_auto().show(ui, |flex_ui| {
                flex_ui.add_ui(item(), |ui| {
                    ui.group(|ui| {
                        ui.set_height(ui.available_height());

                        ui.vertical(|ui| {
                            ui.label("Pak files");
                            ui.group(|ui| {
                                ui.set_width(ui.available_width() * 0.2);
                                ui.set_height(ui.available_height() * 0.6);
                                self.show_pak_files_in_dir(ui);
                            });
                            ui.label("Pak details");
                            ui.group(|ui| {
                                ui.set_height(ui.available_height());
                                ui.set_width(ui.available_width() * 0.2);

                                self.show_pak_details(ui);
                            });
                        });
                    });
                });

                flex_ui.add_ui(item().grow(1.).align_self(FlexAlign::End), |ui| {
                    ui.group(|ui| {
                        ui.set_width(ui.available_width() * 0.8);
                        ui.set_height(ui.available_height());
                        self.list_pak_files(ui).expect("TODO: panic message");
                    });
                });
            });
        });
    }
}

fn main() {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_max_inner_size([1280.0, 720.0])
            .with_min_inner_size([1280.0, 720.]),

        ..Default::default()
    };
    eframe::run_native(
        "Repak GUI",
        options,
        Box::new(|cc| {
            cc.egui_ctx
                .style_mut(|style| style.visuals.dark_mode = true);
            Ok(Box::new(RepakModManager::new(cc)))
        }),
    )
    .expect("Unable to spawn windows");
}
