extern crate winres;
fn main() {
    winres::WindowsResource::new()
        .set_icon("icons/icon.ico")
        .compile()
        .unwrap();
}