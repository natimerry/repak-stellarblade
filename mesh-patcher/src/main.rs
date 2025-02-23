use eframe::egui;
use rfd::FileDialog;
use std::{ thread, time::Duration};

struct MyApp {
    exe_path: String,
    mesh_directory: String,
    output_buffer: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            exe_path: String::new(),
            mesh_directory: String::new(),
            output_buffer: String::from("Initialising libs...")
        }
    }
}

impl MyApp{
    fn doshit(&mut self){
        thread::sleep(Duration::from_secs(1));
        let _ = &self.output_buffer.push_str("APPENDING DATA");
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_width = ui.available_width();
            let button_width = available_width * 0.2;
            let text_width = available_width * 0.8;

            ui.add(egui::Label::new("Select Marvel Rivals .exe file to patch:").wrap());
            ui.horizontal(|ui| {
                ui.add_sized([text_width, 30.0], egui::TextEdit::singleline(&mut self.exe_path));
                if ui.add_sized([button_width, 30.0], egui::Button::new("Browse")).clicked() {
                    if let Some(path) = FileDialog::new().add_filter("Executable", &["exe"]).pick_file() {
                        self.exe_path = path.display().to_string();
                    }
                }
            });
            
            let mesh_width = text_width - button_width - 0.1;
            ui.add(egui::Label::new("Select Mesh Directory to Fix:").wrap());
            ui.horizontal(|ui| {
                ui.add_sized([mesh_width, 30.0], egui::TextEdit::singleline(&mut self.mesh_directory));
                if ui.add_sized([button_width, 30.0], egui::Button::new("Browse")).clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        self.mesh_directory = path.display().to_string();
                    }
                }
                if ui.add_sized([button_width, 30.0], egui::Button::new("Patch meshes")).clicked() {
                    
                    todo!()
                }
            });

            ui.separator();
            ui.vertical(|ui| {
                // ui.add_space(1.0);
                ui.group(|ui| {
                    let panel_width = available_width * 0.98;
                    ui.set_width(panel_width);
                    ui.label("Output");
                    ui.separator();
                    ui.add_sized([panel_width, ui.available_height()], egui::TextEdit::multiline(&mut self.output_buffer).desired_rows(5));
                });
            });
            
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions{
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.,600.])
            .with_resizable(true)
            .with_drag_and_drop(true),
        ..Default::default() 
    };
    eframe::run_native(
        "EXE Launcher",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}