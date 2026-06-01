mod app;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Emulator UI",
        options,
        Box::new(|_cc| Ok(Box::new(app::EmulatorApp::default()))),
    )
}
