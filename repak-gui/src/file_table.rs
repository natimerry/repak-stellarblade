use eframe::egui;
use eframe::egui::OutputCommand::CopyText;
use eframe::egui::RichText;
use egui_extras::{Column, TableBuilder};
use repak::PakReader;
use rfd::FileDialog;
use sha2::Digest;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

pub struct FileTable {
    striped: bool,
    resizable: bool,
    clickable: bool,
    // scroll_to_row_slider: usize,
    // scroll_to_row: Option<usize>,
    file_contents: Vec<FileEntry>,
    selection: usize,
}

#[derive(Clone, Debug)]
struct FileEntry {
    file_path: String,
    pak_path: PathBuf,
    // entry: Entry,
    pak_reader: PakReader,
    compressed: String,
    uncompressed: String,
    offset: String,
}
impl Default for FileTable {
    fn default() -> Self {
        Self {
            striped: true,
            resizable: true,
            clickable: true,
            file_contents: vec![],
            selection: usize::MAX,
        }
    }
}

impl FileTable {
    pub fn new(pak_reader: &PakReader, pak_path: &Path) -> Self {
        let entries = pak_reader
            .files().to_vec();

        let file_entries = entries
            .iter()
            .map(|entry| {
                let entry_pak = pak_reader.get_file_entry(entry).unwrap();
                FileEntry {
                    file_path: entry.clone(),
                    pak_path: PathBuf::from(pak_path),
                    pak_reader: pak_reader.clone(),
                    // entry: pak_reader.get_file_entry(entry).unwrap(),
                    compressed: entry_pak.compressed.to_string(),
                    uncompressed: entry_pak.uncompressed.to_string(),
                    offset: format!("{:#x}", entry_pak.offset),
                }
            })
            .collect::<Vec<_>>();
        Self {
            file_contents: file_entries,
            ..Default::default()
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
                // header.col(|ui| {
                //     ui.label("Compression Slot");
                // });
            })
            .body(|body| {
                // let mut file = self.file_contents.clone();
                body.rows(20.0, self.file_contents.len(),|mut row| {
                    let row_idx = row.index();


                    let entry = &mut self.file_contents[row_idx];
                    row.set_selected(self.selection == row_idx);
                    row.col(|ui| {
                        ui.visuals_mut().widgets.hovered = ui.visuals().widgets.inactive;
                        if ui.label(RichText::new(&entry.file_path).strong()).clicked() {
                            self.selection = row_idx;
                        };
                    })
                    .1
                    .context_menu(|ui| show_ctx_menu(ui, entry));

                    row.col(|ui| {
                        ui.label(&entry.offset);
                    });
                    row.col(|ui| {
                        ui.label(&entry.compressed);
                    });
                    row.col(|ui| {
                        ui.label(&entry.uncompressed);
                    });
                    self.toggle_row_selection(row_idx, &row.response());
                });
            });
    }
    fn toggle_row_selection(&mut self, row_index: usize, row_response: &egui::Response) {
        if row_response.clicked() {
            self.selection = row_index;
        }
    }
}
fn show_ctx_menu(ui: &mut egui::Ui, entry: &FileEntry) {
    if ui.button("Extract").clicked() {
        let name = PathBuf::from(&entry.file_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let dialog = FileDialog::new().set_file_name(name).save_file();
        if let Some(path) = dialog {
            let pak_reader = &entry.pak_reader;
            let mut reader =
                BufReader::new(File::open(&entry.pak_path).expect("Failed to open pak file"));

            let buffer = pak_reader
                .get(entry.file_path.as_str(), &mut reader)
                .expect("Failed to read file");

            let mut file = File::create(path).expect("Failed to create file");
            file.write_all(&buffer).expect("Failed to write file");
            ui.close_menu();
        }
    }
    if ui.button("Copy Path").clicked() {
        ui.output_mut(|o| o.commands = vec![CopyText(entry.file_path.clone())]);
        ui.close_menu();
    }
    if ui.button("Copy Offset").clicked() {
        ui.output_mut(|o| o.commands = vec![CopyText(entry.offset.clone().to_string())]);
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
            hex::encode(hasher.clone().finalize())
        )))
        .clicked()
    {
        ui.output_mut(|o| o.commands = vec![CopyText(hex::encode(hasher.finalize()))]);
    }
}