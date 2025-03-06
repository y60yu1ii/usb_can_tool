mod can;
use crate::can::canbus::*;
use crate::can::cantypes::*;
use eframe::egui;
use flume::{unbounded, RecvTimeoutError};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(PartialEq)]
enum CanApi {
    ControlCan,
    Pcan,
}

const CONTROL_CAN_BAUD_RATES: [u32; 17] = [
    10, 20, 33, 40, 50, 66, 80, 83, 100, 125, 200, 250, 400, 500, 666, 800, 1000,
];
const PCAN_BAUD_RATES: [u32; 14] = [5, 10, 20, 33, 47, 50, 83, 95, 100, 125, 250, 500, 800, 1000];

const DATA_BUFFER_CAPACITY: usize = 1000;
const LOG_BUFFER_CAPACITY: usize = 1000;

/// GUI 程式邏輯，使用 trait 物件儲存不同 CAN API 實作
struct CanGui {
    api: CanApi,
    controlcan_ch1: u32,
    controlcan_baud1: u32,
    controlcan_ch2: u32,
    controlcan_baud2: u32,
    pcan_baud: u32,
    is_receiving: Arc<Mutex<bool>>,
    can_app: Arc<Mutex<Option<Box<dyn CanInterface + Send>>>>,
    logs: Arc<Mutex<VecDeque<String>>>,
    data: Arc<Mutex<VecDeque<String>>>,
}

impl Default for CanGui {
    fn default() -> Self {
        Self {
            api: CanApi::ControlCan,
            controlcan_ch1: 0,
            controlcan_baud1: 250,
            controlcan_ch2: 1,
            controlcan_baud2: 500,
            pcan_baud: 250,
            is_receiving: Arc::new(Mutex::new(false)),
            can_app: Arc::new(Mutex::new(None)),
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(LOG_BUFFER_CAPACITY))),
            data: Arc::new(Mutex::new(VecDeque::with_capacity(DATA_BUFFER_CAPACITY))),
        }
    }
}

impl CanGui {
    fn start_can(&self) {
        {
            let mut rec = self.is_receiving.lock().unwrap();
            if *rec {
                eprintln!("CAN communication is already running.");
                return;
            }
            *rec = true;
        }

        let (log_tx, log_rx) = unbounded();
        let (data_tx, data_rx) = unbounded();

        let log_rx = Arc::new(log_rx);
        let data_rx = Arc::new(data_rx);

        let is_receiving_clone = Arc::clone(&self.is_receiving);
        let logs_store = Arc::clone(&self.logs);
        let data_store = Arc::clone(&self.data);

        {
            let log_rx = Arc::clone(&log_rx);
            let is_receiving = Arc::clone(&is_receiving_clone);
            let logs_store = Arc::clone(&logs_store);
            thread::spawn(move || {
                let timeout = Duration::from_millis(100);
                while *is_receiving.lock().unwrap() {
                    match log_rx.recv_timeout(timeout) {
                        Ok(msg) => {
                            let mut logs = logs_store.lock().unwrap();
                            if logs.len() >= LOG_BUFFER_CAPACITY {
                                logs.pop_front();
                            }
                            logs.push_back(format!("[LOG] {}", msg));
                        }
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
        }

        {
            let data_rx = Arc::clone(&data_rx);
            let is_receiving = Arc::clone(&is_receiving_clone);
            let data_store = Arc::clone(&data_store);
            thread::spawn(move || {
                let timeout = Duration::from_millis(100);
                while *is_receiving.lock().unwrap() {
                    match data_rx.recv_timeout(timeout) {
                        Ok(data_msg) => {
                            let mut data_buf = data_store.lock().unwrap();
                            if data_buf.len() >= DATA_BUFFER_CAPACITY {
                                data_buf.pop_front();
                            }
                            data_buf.push_back(format!("[DATA] {}", data_msg));
                        }
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
        }

        // 設定裝置參數（dev_type 與 dev_index 固定為 4 與 0）
        let dev_type: u32 = 4;
        let dev_index: u32 = 0;

        match self.api {
            CanApi::ControlCan => {
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
                // 建立 ControlCAN 實作
                let can_app = CanApp::new(dev_type, dev_index, channels);
                if let Err(err) = can_app.open_device(log_tx.clone()) {
                    eprintln!("ControlCAN open device failed: {}", err);
                    *is_receiving_clone.lock().unwrap() = false;
                    return;
                }
                can_app.start_receiving(log_tx.clone(), data_tx.clone());
                let mut can_app_guard = self.can_app.lock().unwrap();
                *can_app_guard = Some(Box::new(can_app));
            }
            CanApi::Pcan => {
                // 建立 PCAN 實作，channel 固定為 0x51
                let channel: u32 = 0x51;
                let pcan_baud =
                    PcanBaudRate::from_u32(self.pcan_baud).unwrap_or(PcanBaudRate::Baud250K);
                let can_app = PcanApp::new(channel, pcan_baud);
                if let Err(err) = can_app.open_device(log_tx.clone()) {
                    eprintln!("PCAN open device failed: {}", err);
                    *is_receiving_clone.lock().unwrap() = false;
                    return;
                }
                can_app.start_receiving(log_tx.clone(), data_tx.clone());
                let mut can_app_guard = self.can_app.lock().unwrap();
                *can_app_guard = Some(Box::new(can_app));
            }
        }
    }

    fn stop_can(&self) {
        {
            let mut rec = self.is_receiving.lock().unwrap();
            if !*rec {
                eprintln!("CAN communication is not running.");
                return;
            }
            *rec = false;
        }
        // 利用一組空的 log_tx 呼叫 close_device
        let (log_tx, _) = unbounded();
        if let Some(ref can_app) = *self.can_app.lock().unwrap() {
            can_app.stop_receiving();
            can_app.close_device(log_tx.clone());
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
        egui::TopBottomPanel::top("config_panel").show(ctx, |ui| {
            ui.heading("CAN Bus Configuration");
            ui.horizontal(|ui| {
                ui.label("Select CAN API:");
                ui.radio_value(&mut self.api, CanApi::ControlCan, "ControlCAN");
                ui.radio_value(&mut self.api, CanApi::Pcan, "PCAN");
            });
            match self.api {
                CanApi::ControlCan => {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Channel 1:");
                        ui.add(egui::DragValue::new(&mut self.controlcan_ch1));
                        ui.label("Baud Rate:");
                        egui::ComboBox::from_id_salt("baud1")
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
                        ui.label("Channel 2:");
                        ui.add(egui::DragValue::new(&mut self.controlcan_ch2));
                        ui.label("Baud Rate:");
                        egui::ComboBox::from_id_salt("baud2")
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
                        ui.label("PCAN Baud Rate:");
                        egui::ComboBox::from_id_salt("pcan_baud")
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
            ui.horizontal(|ui| {
                if ui.button("Start CAN").clicked() {
                    self.start_can();
                }
                if ui.button("Stop CAN").clicked() {
                    self.stop_can();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |cols| {
                // Log 區
                cols[0].vertical(|ui| {
                    ui.heading("Log");
                    egui::ScrollArea::vertical()
                        .id_source("logs_scroll_area")
                        .stick_to_bottom(true)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            let logs = self.logs.lock().unwrap();
                            for log in logs.iter() {
                                ui.label(log);
                            }
                        });
                });
                // Data 區
                cols[1].vertical(|ui| {
                    ui.heading("Data");
                    egui::ScrollArea::vertical()
                        .id_source("data_scroll_area")
                        .stick_to_bottom(true)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            let data = self.data.lock().unwrap();
                            for line in data.iter() {
                                ui.label(line);
                            }
                        });
                });
            });
        });
        ctx.request_repaint();
    }
}
