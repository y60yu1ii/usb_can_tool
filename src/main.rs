mod canbus; // 假設 canbus.rs 放在同一個 crate 中

use canbus::CanApp;
use flume::unbounded;
use std::{thread, time::Duration};

fn main() {
    // 建立 log 與資料的 channel
    let (log_tx, log_rx) = unbounded();
    let (data_tx, data_rx) = unbounded();

    // 啟動執行緒印出 log 訊息
    thread::spawn(move || {
        while let Ok(log) = log_rx.recv() {
            println!("[LOG] {}", log);
        }
    });

    // 啟動執行緒印出接收的 CAN 資料
    thread::spawn(move || {
        while let Ok(data) = data_rx.recv() {
            println!("[DATA] {}", data);
        }
    });

    // 建立 CanApp 實例 (內部會載入 ControlCAN.dll)
    let can_app = CanApp::new();

    // 設定參數：假設 dev_type 為 4 (例如 VCI_USBCAN2)、dev_index 為 0，使用第 0 路 CAN 通道
    let dev_type: u32 = 4;
    let dev_index: u32 = 0;
    let can_channel: u32 = 0;

    // 開啟裝置與初始化 CAN
    if !can_app.open_device(dev_type, dev_index, can_channel, log_tx.clone()) {
        eprintln!("開啟裝置失敗");
        return;
    }

    // 啟動接收資料（在新執行緒中持續接收）
    can_app.start_receiving(
        dev_type,
        dev_index,
        can_channel,
        log_tx.clone(),
        data_tx.clone(),
    );

    println!("開始接收資料，等待 10 秒鐘...");
    thread::sleep(Duration::from_secs(10));

    // 停止接收資料
    can_app.stop_receiving();

    // 關閉裝置
    can_app.close_device(dev_type, dev_index, log_tx.clone());

    // 等待一會兒以確保所有 log 訊息都能印出
    thread::sleep(Duration::from_secs(2));
}
