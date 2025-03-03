mod can;
use crate::can::canbus::*;

use flume::unbounded;
use std::env;
use std::{thread, time::Duration};

/// 定義可選 API 枚舉
enum CanApi {
    ControlCan,
    Pcan,
}

fn main() {
    // 根據命令行參數決定使用哪一個 API，若參數為 "pcan" 則使用 PCANBasic，其餘則預設使用 ControlCAN
    let args: Vec<String> = env::args().collect();
    let api = if args.len() > 1 && args[1].to_lowercase() == "pcan" {
        CanApi::Pcan
    } else {
        CanApi::ControlCan
    };

    // 建立 log 與 data 通道
    let (log_tx, log_rx) = unbounded();
    let (data_tx, data_rx) = unbounded();

    // Log 印出執行緒
    thread::spawn(move || {
        while let Ok(msg) = log_rx.recv() {
            println!("[LOG] {}", msg);
        }
    });

    // Data 印出執行緒
    thread::spawn(move || {
        while let Ok(data) = data_rx.recv() {
            println!("[DATA] {}", data);
        }
    });

    // 假設參數，dev_type = 4, dev_index = 0, 通道 = 0
    let dev_type: u32 = 4;
    let dev_index: u32 = 0;

    match api {
        CanApi::ControlCan => {
            println!("Using ControlCAN API");
            let channel: u32 = 0; //for canalyst II
            let can_app = CanApp::new();
            if !can_app.open_device(dev_type, dev_index, channel, log_tx.clone()) {
                eprintln!("ControlCAN open device failed");
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
            can_app.close_device(dev_type, dev_index, log_tx.clone());
        }
        CanApi::Pcan => {
            println!("Using PCANBasic API");
            let channel: u32 = 0x51; //for pcan
            let can_app = PcanApp::new();
            if !can_app.open_device(dev_type, dev_index, channel, log_tx.clone()) {
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

    // 等待 log 輸出完畢
    thread::sleep(Duration::from_secs(2));
}
