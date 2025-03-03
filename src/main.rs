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
            Ok(value) => match api {
                CanApi::ControlCan => CanBaudRate::ControlCan(
                    VciCanBaudRate::from_u32(value).unwrap_or(VciCanBaudRate::Baud250K),
                ),
                CanApi::Pcan => CanBaudRate::Pcan(
                    PcanBaudRate::from_u32(value).unwrap_or(PcanBaudRate::Baud250K),
                ),
            },
            Err(_) => {
                eprintln!("無效的波特率: {}，使用預設值 250Kbps", args[2]);
                match api {
                    CanApi::ControlCan => CanBaudRate::ControlCan(VciCanBaudRate::Baud250K),
                    CanApi::Pcan => CanBaudRate::Pcan(PcanBaudRate::Baud250K),
                }
            }
        }
    } else {
        println!("未指定波特率，使用預設 250Kbps");
        match api {
            CanApi::ControlCan => CanBaudRate::ControlCan(VciCanBaudRate::Baud250K),
            CanApi::Pcan => CanBaudRate::Pcan(PcanBaudRate::Baud250K),
        }
    };

    match baud_rate {
        CanBaudRate::ControlCan(baud) => {
            println!("開啟 ControlCAN，波特率為 {:?}", baud);
        }
        CanBaudRate::Pcan(baud) => {
            println!("開啟 PCAN，波特率為 {:?}", baud);
        }
    }
    let (log_tx, log_rx) = unbounded();
    let (data_tx, data_rx) = unbounded();

    thread::spawn(move || {
        while let Ok(msg) = log_rx.recv() {
            println!("[LOG] {}", msg);
        }
    });

    thread::spawn(move || {
        while let Ok(data) = data_rx.recv() {
            println!("[DATA] {}", data);
        }
    });

    let dev_type: u32 = 4;
    let dev_index: u32 = 0;

    match api {
        CanApi::ControlCan => {
            println!("Using ControlCAN API");
            let can_app = CanApp::new();

            let mut can_channels = Vec::new();

            for arg in args.iter().skip(2) {
                // 跳過前 2 個參數 (`controlcan`)
                if let Some((ch, br)) = arg.split_once(':') {
                    if let (Ok(channel), Ok(baud)) = (ch.parse::<u32>(), br.parse::<u32>()) {
                        if let Some(valid_baud) = VciCanBaudRate::from_u32(baud) {
                            can_channels.push((channel, valid_baud));
                        } else {
                            eprintln!("無效的波特率: {} (通道: {})，跳過...", baud, channel);
                        }
                    } else {
                        eprintln!("無效的通道或波特率格式: {}，跳過...", arg);
                    }
                }
            }

            if can_channels.is_empty() {
                println!("未指定通道，預設使用 CAN 通道 0 (250Kbps)");
                can_channels.push((0, VciCanBaudRate::Baud250K));
                can_channels.push((1, VciCanBaudRate::Baud1M));
            }

            if !can_app.open_device(dev_type, dev_index, &can_channels, log_tx.clone()) {
                eprintln!("ControlCAN open device failed");
                return;
            }

            for &(channel, _) in &can_channels {
                can_app.start_receiving(
                    dev_type,
                    dev_index,
                    channel,
                    log_tx.clone(),
                    data_tx.clone(),
                );
            }

            thread::sleep(Duration::from_secs(10));

            can_app.stop_receiving();
            can_app.close_device(dev_type, dev_index, log_tx.clone());
        }

        CanApi::Pcan => {
            println!("Using PCANBasic API");
            let channel: u32 = 0x51; //for pcan
            let can_app = PcanApp::new();
            if let CanBaudRate::Pcan(baud) = baud_rate {
                if !can_app.open_device(dev_type, dev_index, channel, baud, log_tx.clone()) {
                    eprintln!("PCAN open device failed");
                    return;
                }
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

    thread::sleep(Duration::from_secs(2));
}
