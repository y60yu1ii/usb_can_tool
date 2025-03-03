mod can;
use crate::can::canbus::*;
use crate::can::cantypes::*;

use flume::unbounded;
use std::env;
use std::{thread, time::Duration};

enum CanApi {
    ControlCan,
    Pcan,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let api = if args.len() > 1 && args[1].to_lowercase() == "pcan" {
        CanApi::Pcan
    } else {
        CanApi::ControlCan
    };

    let baud_rate = if args.len() > 2 {
        match args[2].parse::<u32>() {
            Ok(value) => VciCanBaudRate::from_u32(value).unwrap_or(VciCanBaudRate::Kbps250), // 預設 250Kbps
            Err(_) => {
                eprintln!("無效的波特率: {}，使用預設值 250Kbps", args[2]);
                VciCanBaudRate::Kbps250
            }
        }
    } else {
        println!("未指定波特率，使用預設 250Kbps");
        VciCanBaudRate::Kbps250
    };

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
            let can_channel: u32 = 0; //for canalyst II
            let can_app = CanApp::new();
            if !can_app.open_device(dev_type, dev_index, can_channel, baud_rate, log_tx.clone()) {
                eprintln!("ControlCAN open device failed");
                return;
            }
            can_app.start_receiving(
                dev_type,
                dev_index,
                can_channel,
                log_tx.clone(),
                data_tx.clone(),
            );
            //turned off after 10 seconds
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
            //turned off after 10 seconds
            thread::sleep(Duration::from_secs(10));
            can_app.stop_receiving();
            can_app.close_device(dev_type, dev_index, channel, log_tx.clone());
        }
    }

    // 等待 log 輸出完畢
    thread::sleep(Duration::from_secs(2));
}
