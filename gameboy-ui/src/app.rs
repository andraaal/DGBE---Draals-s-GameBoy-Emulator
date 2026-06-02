use std::{ffi::OsStr, path::Path};

use eframe::egui;
use egui_file::FileDialog;
use gameboy_emu::DebugView;

pub(crate) struct EmulatorApp {
    cycles: u64,
    registers: gameboy_emu::registers::Registers,
    opcodes: Vec<(u16, u8)>,
    errors: Vec<String>,
    emulator: gameboy_emu::Emulator,
    open_file_dialog: Option<FileDialog>,
    opened_rom: String,
}

impl Default for EmulatorApp {
    fn default() -> Self {
        Self {
            cycles: 0,
            registers: gameboy_emu::registers::Registers::new(),
            opcodes: Vec::new(),
            errors: Vec::new(),
            emulator: gameboy_emu::Emulator::new(),
            open_file_dialog: None,
            opened_rom: "No ROM loaded".to_string(),
        }
    }
}

impl eframe::App for EmulatorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Run").clicked() {
                    let view = self.emulator.run();
                    self.parse_view(view);

                    ui.request_repaint();
                }

                if ui.button("Pause").clicked() {}

                if ui.button("Step").clicked() {
                    let view = self.emulator.step();
                    self.parse_view(view);

                    ui.request_repaint();
                }

                ui.separator();

                ui.label(format!("Cycles: {}", self.cycles / 4));

                ui.separator();

                if ui.button("Load ROM").clicked() {
                    let filter = Box::new({
                        let ext = Some(OsStr::new("gb"));
                        move |path: &Path| path.extension() == ext
                    });

                    let mut dialog = FileDialog::open_file().show_files_filter(filter);
                    dialog.open();
                    self.open_file_dialog = Some(dialog);
                }

                ui.label(self.opened_rom.clone());
            });
        });

        if let Some(dialog) = &mut self.open_file_dialog
            && dialog.show(ui).selected()
            && let Some(file) = dialog.path()
        {
            if let Ok(rom) = std::fs::read(file) {
                if let Err(e) = self.emulator.load_rom(rom) {
                    self.errors.push(e);
                }
                self.opened_rom = file
                    .file_name()
                    .map_or("Unknown name".into(), |n| n.to_string_lossy().to_string());
            } else {
                self.errors
                    .push(format!("Failed to read ROM: {}", file.display()));
            }
        }

        if self.errors.len() > 0 {
            egui::Panel::bottom("error_panel")
                .resizable(true)
                .show_inside(ui, |ui| {
                    ui.heading("Errors");

                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for e in &self.errors {
                            ui.monospace(e);
                        }

                        ui.allocate_space(ui.available_size());
                    });
                });
        }

        // Left debugger panel
        egui::Panel::left("debug_panel")
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.heading("CPU");

                ui.separator();

                ui.label(format!("Register A: 0x{:02X}", self.registers.a));
                ui.label(format!("Register B: 0x{:02X}", self.registers.b));
                ui.label(format!("Register C: 0x{:02X}", self.registers.c));
                ui.label(format!("Register D: 0x{:02X}", self.registers.d));
                ui.label(format!("Register E: 0x{:02X}", self.registers.e));
                ui.label(format!("Register H: 0x{:02X}", self.registers.h));
                ui.label(format!("Register L: 0x{:02X}", self.registers.l));
                ui.label(format!("Register F: 0b{:08b}", self.registers.f));
                ui.label(format!("Program Counter: 0x{:04X}", self.registers.pc));
                ui.label(format!("Stack Pointer: 0x{:04X}", self.registers.sp));
                ui.label(format!(
                    "IME: {}",
                    if self.registers.ime {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                ));
                ui.label(format!(
                    "IME Next: {}",
                    if self.registers.ime_next {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                ));

                ui.separator();

                ui.heading("Disassembly");

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_rows(
                        ui,
                        ui.text_style_height(&egui::TextStyle::Body),
                        self.opcodes.len(),
                        |ui, row_index| {
                            for (address, opcode) in &self.opcodes[row_index] {
                                ui.label(format!("0x{:04X}: 0x{:02X}", address, opcode));
                            }
                        },
                    );
            });

        // Main screen area
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Screen");

            ui.separator();

            // Fake emulator screen
            let size = egui::vec2(512.0, 480.0);

            let (_id, rect) = ui.allocate_space(size);

            ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Framebuffer goes here",
                egui::FontId::proportional(24.0),
                egui::Color32::WHITE,
            );
        });
    }
}

impl EmulatorApp {
    fn parse_view(&mut self, mut view: DebugView) {
        self.cycles = view.cycles;
        self.registers = view.registers;
        self.opcodes.append(&mut view.opcodes);
        self.errors.append(&mut view.errors);
    }
}
