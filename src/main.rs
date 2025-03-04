mod can;
use crate::can::canbus::*;
use crate::can::cantypes::*;

use eframe::egui;
use flume::unbounded;
use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};
#[derive(PartialEq)]
enum CanApi {
    ControlCan,
    Pcan,
}

const CONTROL_CAN_BAUD_RATES: [u32; 17] = [
    10, 20, 33, 40, 50, 66, 80, 83, 100, 125, 200, 250, 400, 500, 666, 800, 1000,
];
const PCAN_BAUD_RATES: [u32; 14] = [5, 10, 20, 33, 47, 50, 83, 95, 100, 125, 250, 500, 800, 1000];

struct CanGui {
    api: CanApi,
    controlcan_ch1: u32,
    controlcan_baud1: u32,
    controlcan_ch2: u32,
    controlcan_baud2: u32,
    pcan_baud: u32,
}

impl Default for CanGui {
    fn default() -> Self {
        Self {
            api: CanApi::ControlCan,
            controlcan_ch1: 0,
            controlcan_baud1: 250,
            controlcan_ch2: 1,
            controlcan_baud2: 1000,
            pcan_baud: 250,
        }
    }
}

impl CanGui {
    fn start_can(&self) {
        let (log_tx, log_rx) = unbounded();
        let (data_tx, data_rx) = unbounded();
        let data_rx = Arc::new(Mutex::new(data_rx));
        let dev_type: u32 = 4;
        let dev_index: u32 = 0;

        thread::spawn(move || {
            while let Ok(msg) = log_rx.recv() {
                println!("[LOG] {}", msg);
            }
        });

        match self.api {
            CanApi::ControlCan => {
                let can_app = CanApp::new();
                let channels = vec![
                    (
                        self.controlcan_ch1,
                        VciCanBaudRate::from_u32(self.controlcan_baud1)
                            .unwrap_or(VciCanBaudRate::Baud250K),
                    ),
                    (
                        self.controlcan_ch2,
                        VciCanBaudRate::from_u32(self.controlcan_baud2)
                            .unwrap_or(VciCanBaudRate::Baud1M),
                    ),
                ];
                if !can_app.open_device(dev_type, dev_index, &channels, log_tx.clone()) {
                    eprintln!("ControlCAN open device failed");
                    return;
                }
                can_app.start_receiving(
                    dev_type,
                    dev_index,
                    &channels.iter().map(|(ch, _)| *ch).collect::<Vec<u32>>(),
                    log_tx.clone(),
                    data_tx.clone(),
                );
                thread::sleep(Duration::from_secs(10));
                can_app.stop_receiving();
                can_app.close_device(dev_type, dev_index, log_tx.clone());
            }
            CanApi::Pcan => {
                let can_app = PcanApp::new();
                let channel: u32 = 0x51;
                if !can_app.open_device(
                    dev_type,
                    dev_index,
                    channel,
                    PcanBaudRate::from_u32(self.pcan_baud).unwrap_or(PcanBaudRate::Baud250K),
                    log_tx.clone(),
                ) {
                    eprintln!("PCAN open device failed");
                    return;
                }
                can_app.start_receiving(
                    dev_type,
                    dev_index,
                    channel,
                    log_tx.clone(),
                    data_tx.clone(),
                );
                thread::sleep(Duration::from_secs(10));
                can_app.stop_receiving();
                can_app.close_device(dev_type, dev_index, channel, log_tx.clone());
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "CAN Bus GUI",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(CanGui::default()))),
    )
}

impl eframe::App for CanGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("CAN Bus 設定");

            ui.horizontal(|ui| {
                ui.label("選擇 CAN API:");
                ui.radio_value(&mut self.api, CanApi::ControlCan, "ControlCAN");
                ui.radio_value(&mut self.api, CanApi::Pcan, "PCAN");
            });

            match self.api {
                CanApi::ControlCan => {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("通道 1:");
                        ui.add(egui::DragValue::new(&mut self.controlcan_ch1));
                        ui.label("波特率:");
                        egui::ComboBox::from_label("")
                            .selected_text(format!("{}K", self.controlcan_baud1))
                            .show_ui(ui, |ui| {
                                for &rate in CONTROL_CAN_BAUD_RATES.iter() {
                                    ui.selectable_value(
                                        &mut self.controlcan_baud1,
                                        rate,
                                        format!("{}K", rate),
                                    );
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("通道 2:");
                        ui.add(egui::DragValue::new(&mut self.controlcan_ch2));
                        ui.label("波特率:");
                        egui::ComboBox::from_label("")
                            .selected_text(format!("{}K", self.controlcan_baud2))
                            .show_ui(ui, |ui| {
                                for &rate in CONTROL_CAN_BAUD_RATES.iter() {
                                    ui.selectable_value(
                                        &mut self.controlcan_baud2,
                                        rate,
                                        format!("{}K", rate),
                                    );
                                }
                            });
                    });
                }
                CanApi::Pcan => {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("PCAN 波特率:");
                        egui::ComboBox::from_label("")
                            .selected_text(format!("{}K", self.pcan_baud))
                            .show_ui(ui, |ui| {
                                for &rate in PCAN_BAUD_RATES.iter() {
                                    ui.selectable_value(
                                        &mut self.pcan_baud,
                                        rate,
                                        format!("{}K", rate),
                                    );
                                }
                            });
                    });
                }
            }

            if ui.button("開始 CAN 通訊").clicked() {
                self.start_can();
            }
        });
    }
}
