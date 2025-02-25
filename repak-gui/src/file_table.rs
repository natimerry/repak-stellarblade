use eframe::egui;
use eframe::egui::accesskit::Role::ScrollBar;
use eframe::egui::OutputCommand::CopyText;
use eframe::egui::{CursorIcon, RichText, ScrollArea, Widget};
use egui_extras::{Column, TableBuilder};
use repak::entry::Entry;
use repak::PakReader;
use sha2::Digest;
use std::fs::File;
use std::hash::Hash;
use std::io::BufReader;
use std::path::PathBuf;
use std::usize::MAX;

pub struct FileTable {
    striped: bool,
    resizable: bool,
    clickable: bool,
    // scroll_to_row_slider: usize,
    // scroll_to_row: Option<usize>,
    checked: bool,
    file_contents: Vec<FileEntry>,
    selection: usize,
}

#[derive(Clone, Debug)]
struct FileEntry {
    file_path: String,
    pak_path: PathBuf,
    entry: Entry,
    pak_reader: PakReader,
}
impl Default for FileTable {
    fn default() -> Self {
        Self {
            striped: true,
            resizable: true,
            clickable: true,
            checked: true,
            file_contents: vec![],
            selection: MAX,
        }
    }
}

impl FileTable {
    pub fn new(pak_reader: &PakReader, pak_path: &PathBuf) -> Self {
        let entries = pak_reader
            .files()
            .iter()
            .map(|s| s.clone())
            .collect::<Vec<_>>();

        let file_entries = entries
            .iter()
            .map(|entry| FileEntry {
                file_path: entry.clone(),
                pak_path: pak_path.clone(),
                pak_reader: pak_reader.clone(),
                entry: pak_reader.get_file_entry(entry).unwrap(),
            })
            .collect::<Vec<_>>();
        println!("NEW TABLE");
        Self {
            file_contents: file_entries,
            ..Default::default()
        }
    }

    fn show_ctx_menu(&mut self, ui: &mut egui::Ui, entry: &FileEntry) {
        if ui.button("Extract").clicked() {
            // Handle extraction logic
            ui.close_menu();
        }
        if ui.button("Copy Path").clicked() {
            ui.output_mut(|o| o.commands = vec![CopyText(entry.file_path.clone())]);
            ui.close_menu();
        }
        if ui.button("Copy Offset").clicked() {
            ui.output_mut(|o| o.commands = vec![CopyText(entry.entry.offset.clone().to_string())]);
            ui.close_menu();
        }

        let mut hasher = sha2::Sha256::new();
        entry
            .pak_reader
            .read_file(
                entry.file_path.as_str(),
                &mut BufReader::new(File::open(&entry.pak_path).expect("Failed to open pak file")),
                &mut hasher,
            )
            .expect("Failed to read file");

        if ui
            .button("View Hash (Click to copy)")
            .on_hover_text(RichText::new(format!(
                "SHA256 hash: {}",
                hex::encode(hasher.clone().finalize().to_vec())
            )))
            .clicked()
        {
            ui.output_mut(|o| o.commands = vec![CopyText(hex::encode(hasher.finalize().to_vec()))]);
        }
    }
    pub fn table_ui(&mut self, ui: &mut egui::Ui) {
        let available_height = ui.available_height();
        let mut table = TableBuilder::new(ui)
            .striped(self.striped)
            .resizable(self.resizable)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(
                Column::initial(800.)
                    .at_least(500.0)
                    .clip(true)
                    .resizable(true)
                    .at_most(1000.),
            ) // PATH
            .column(Column::remainder()) // Offset
            .column(Column::remainder()) // Compressed Size
            .column(Column::remainder()) // Uncompressed Size
            .column(Column::remainder()) // Compression Slot
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height);

        if self.clickable {
            table = table.sense(egui::Sense::click());
        }
        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Path");
                });
                header.col(|ui| {
                    ui.label("Offset");
                });
                header.col(|ui| {
                    ui.label("Compressed");
                });
                header.col(|ui| {
                    ui.label("Uncompressed");
                });
                header.col(|ui| {
                    ui.label("Compression Slot");
                });
            })
            .body(|mut body| {
                let file = self.file_contents.clone();
                for (_row_index, entry) in file.iter().enumerate() {
                    body.row(20.0, |mut row| {
                        row.set_selected(self.selection == _row_index);

                        row.col(|ui| {
                            ui.visuals_mut().widgets.hovered = ui.visuals().widgets.inactive;
                            if ui
                                .label(RichText::new(entry.file_path.clone()).strong())
                                .clicked()
                            {
                                self.selection = _row_index;
                            };
                        })
                        .1
                        .context_menu(|ui| self.show_ctx_menu(ui, entry));

                        let entry_pak = &entry.entry;
                        row.col(|ui| {
                            ui.label(format!("{:#x}", entry_pak.offset));
                        });
                        row.col(|ui| {
                            ui.label(format!("{:#x} bytes", entry_pak.compressed));
                        });
                        row.col(|ui| {
                            ui.label(format!("{:#x} bytes", entry_pak.uncompressed));
                        });
                        row.col(|ui| {
                            ui.label(
                                entry_pak
                                    .compression_slot
                                    .map_or("-".to_string(), |v| v.to_string()),
                            );
                        });
                        self.toggle_row_selection(_row_index, &row.response());
                    });
                }
            });
    }
    fn toggle_row_selection(&mut self, row_index: usize, row_response: &egui::Response) {
        if row_response.clicked() {
            self.selection = row_index;
        }
    }
}
