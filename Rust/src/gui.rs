use crate::{Config, CooperativeServer};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use std::thread;

pub struct CoopApp {
    config: Config,
    logs: Vec<String>,
    status: String,
    is_running: bool,
    log_rx: Receiver<String>,
    log_tx: Sender<String>,
}

impl CoopApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (log_tx, log_rx) = crossbeam_channel::unbounded();
        Self {
            config: Config::load(),
            logs: vec!["App started.".to_string()],
            status: "Idle".to_string(),
            is_running: false,
            log_rx,
            log_tx,
        }
    }

    fn start_server(&mut self) {
        let token = self.config.dropbox_token.clone().unwrap_or_default();
        let save_dir = self.config.save_dir.clone().unwrap_or_else(|| {
            "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Medieval II Total War\\mods\\crusades\\saves".to_string()
        });

        if token.is_empty() {
            self.logs
                .push("Error: No Dropbox token provided!".to_string());
            return;
        }

        self.is_running = true;
        self.status = "Running".to_string();

        let log_tx = self.log_tx.clone();
        let server = CooperativeServer::new(token, save_dir).with_logger(log_tx.clone());

        thread::spawn(move || {
            let _ = log_tx.send("Server thread started.".to_string());
            loop {
                if let Err(e) = server.run() {
                    let _ = log_tx.send(format!("Server Error: {}", e));
                }
                thread::sleep(std::time::Duration::from_secs(1));
            }
        });
    }
}

impl eframe::App for CoopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle incoming logs
        while let Ok(msg) = self.log_rx.try_recv() {
            self.logs.push(msg);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("M2TW Seamless Co-op Tool");

            ui.group(|ui| {
                ui.label("Configuration");
                ui.horizontal(|ui| {
                    ui.label("Dropbox Token:");
                    let mut token = self.config.dropbox_token.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut token).changed() {
                        self.config.dropbox_token = Some(token);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Save Directory:");
                    let mut dir = self.config.save_dir.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut dir).changed() {
                        self.config.save_dir = Some(dir);
                    }
                });
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Start Server").clicked() && !self.is_running {
                    self.start_server();
                }
                ui.label(format!("Status: {}", self.status));
            });

            ui.add_space(10.0);
            ui.label("Logs:");
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for log in &self.logs {
                        ui.label(log);
                    }
                });
        });

        // Repaint periodically to show new logs
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
