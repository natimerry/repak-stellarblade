use crate::setup_custom_style;
use crate::utils::get_current_pak_characteristics;
use eframe::egui;
use eframe::egui::{Align, Checkbox, ComboBox, Context, Label, TextEdit};
use egui_extras::{Column, TableBuilder};
use egui_flex::{item, Flex, FlexAlign};
use repak::utils::AesKey;
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;
use repak::Compression;

#[derive(Clone, Debug, Default)]
pub struct InstallableMod {
    mod_name: String,
    mod_type: String,
    repak: bool,
    fix_mesh: bool,
    is_dir: bool,
    editing: bool,

    path_hash_seed: String,
    mount_point: String,
    compression: Compression,
}


#[derive(Debug)]
pub struct ModInstallRequest {
    pub(crate) mods: Vec<InstallableMod>,
}

impl ModInstallRequest {
    pub fn new_mod_dialog(&mut self, ctx: &egui::Context, show_callback: &mut bool) {
        let viewport_options = egui::ViewportBuilder::default()
            .with_title("Deferred Viewport")
            .with_inner_size([1000.0, 800.0])
            .with_always_on_top();

        Context::show_viewport_immediate(
            &ctx,
            egui::ViewportId::from_hash_of("immediate_viewport"),
            viewport_options,
            |ctx, class| {
                assert!(
                    class == egui::ViewportClass::Immediate,
                    "This egui backend doesn't support multiple viewports"
                );

                setup_custom_style(ctx);
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("Mods to install");
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height(ui.available_height());
                    // ScrollArea::vertical()
                    //     .auto_shrink([false, false])
                    //     .show(ui, |ui| {
                    self.table_ui(ui);
                    // });
                });

                egui::TopBottomPanel::bottom("bottom_panel")
                    .min_height(50.)
                    .show(ctx, |ui| {
                        // ui.heading("WTF");
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(ui.available_height());
                        Flex::horizontal()
                            .align_items(FlexAlign::Center)
                            .w_full()
                            .h_full()
                            .show(ui, |ui| {
                                let selection_bg_color = ctx.style().visuals.selection.bg_fill;
                                ui.add(
                                    item(),
                                    egui::Button::new("Install mod").fill(selection_bg_color),
                                );
                                ui.add(item(), egui::Button::new("Cancel"));
                            })
                    });
                if ctx.input(|i| i.viewport().close_requested()) {
                    // Tell parent viewport that we should not show next frame:
                    *show_callback = false;
                }
            },
        );
    }

    fn table_ui(&mut self, ui: &mut egui::Ui) {
        let available_height = ui.available_height();
        ui.separator();

        let table = TableBuilder::new(ui)
            .striped(true)
            // .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
            .column(Column::auto())
            .column(Column::remainder().at_least(400.)) // mod name
            .column(Column::auto()) // Character
            .column(Column::auto()) // type
            .column(Column::remainder()) // options
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height);

        table
            .header(20., |mut header| {
                header.col(|ui| {
                    ui.label("Row");
                });

                header.col(|ui| {
                    ui.label("Name");
                });

                header.col(|ui| {
                    ui.label("Category");
                });

                header.col(|ui| {
                    ui.label("Mod type");
                });
                header.col(|ui| {
                    ui.label("Options");
                });
            })
            .body(|mut body| {
                for (rowidx, mods) in self.mods.iter_mut().enumerate() {
                    body.row(20., |mut row| {
                        row.col(|ui| {
                            ui.add(Label::new(format!("{})", rowidx + 1)).halign(Align::RIGHT));
                        });
                        // name field
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if mods.editing {
                                    ui.set_width(ui.available_width() * 0.8); // padding for edit
                                    let text_edit = ui.add(
                                        egui::TextEdit::singleline(&mut mods.mod_name)
                                            .clip_text(true),
                                    );
                                    if text_edit.lost_focus()
                                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        mods.editing = false;
                                    }
                                } else {
                                    ui.add(
                                        Label::new(&mods.mod_name).halign(Align::LEFT).truncate(),
                                    );
                                }
                                // align button right
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button(if mods.editing { "✔" } else { "✏" }).clicked()
                                        {
                                            mods.editing = !mods.editing;
                                        }
                                    },
                                );
                            });
                        });
                        row.col(|ui| {
                            ui.label(&mods.mod_type);
                        });
                        row.col(|ui| {
                            let label = match mods.is_dir {
                                true => "Directory",
                                false => "Pak",
                            };
                            ui.label(label);
                        });
                        row.col(|ui| {
                            ui.collapsing("Options", |ui| {
                                ui.add_enabled(
                                    !mods.is_dir,
                                    Checkbox::new(&mut mods.repak, "To repak"),
                                );
                                ui.add_enabled(
                                    mods.repak,
                                    Checkbox::new(&mut mods.fix_mesh, "Fix mesh"),
                                );
                                let text_edit = TextEdit::singleline(&mut mods.mount_point);
                                ui.add(text_edit.hint_text("Enter mount point..."));

                                // Text edit for path_hash_seed with hint
                                let text_edit = TextEdit::singleline(&mut mods.path_hash_seed);
                                ui.add(text_edit.hint_text("Enter path hash seed..."));

                                ComboBox::new("comp_level","Compression Algorithm")
                                    .selected_text(format!("{:?}", mods.compression))
                                    .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut mods.compression, Compression::Zlib, "Zlib");
                                    ui.selectable_value(&mut mods.compression, Compression::Gzip, "Gzip");
                                    ui.selectable_value(&mut mods.compression, Compression::Oodle, "Oodle");
                                    ui.selectable_value(&mut mods.compression, Compression::Zstd, "Zstd");
                                    ui.selectable_value(&mut mods.compression, Compression::LZ4, "LZ4");
                                });
                            });
                        });
                    })
                }
            });
    }
}
pub fn map_dropped_file_to_mods(dropped_files: &Vec<egui::DroppedFile>) -> Vec<InstallableMod> {
    let files = dropped_files
        .into_iter()
        .map(|dropped_file| {
            let aes_key = AesKey::from_str(
                "0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74",
            )
            .unwrap();

            let is_dir = dropped_file.path.clone().unwrap().is_dir();
            let mut modtype = "Unknown".to_string();

            let pakfile = dropped_file.path.clone().unwrap();
            if !is_dir {
                let mut builder = repak::PakBuilder::new();
                builder = builder.key(aes_key.0.clone());

                let pak =
                    builder.reader(&mut BufReader::new(File::open(pakfile.clone()).unwrap()))?;

                modtype = get_current_pak_characteristics(pak.files());
            }

            Ok(InstallableMod {
                mod_name: pakfile.file_stem().unwrap().to_str().unwrap().to_string(),
                mod_type: modtype,
                repak: !is_dir,
                fix_mesh: false,
                is_dir,
                ..Default::default()
            })
        })
        .filter_map(|x: Result<InstallableMod, repak::Error>| x.ok())
        .collect::<Vec<_>>();
    files
}
