use flume::Sender;
use libloading::Library;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::{thread, time::Duration};

#[repr(C)]
#[derive(Debug, Default)]
pub struct VciCanObj {
    pub id: u32,
    pub time_stamp: u32,
    pub time_flag: u8,
    pub send_type: u8,
    pub remote_flag: u8,
    pub extern_flag: u8,
    pub data_len: u8,
    pub data: [u8; 8],
    pub reserved: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct VciInitConfig {
    pub acc_code: u32,
    pub acc_mask: u32,
    pub reserved: u32,
    pub filter: u8,
    pub timing0: u8,
    pub timing1: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct VciBoardInfo {
    pub hw_version: u16,
    pub fw_version: u16,
    pub dr_version: u16,
    pub in_version: u16,
    pub irq_num: u16,
    pub can_num: u8,
    pub str_serial_num: [u8; 20],
    pub str_hw_type: [u8; 40],
    pub reserved: [u16; 4],
}

impl Default for VciBoardInfo {
    fn default() -> Self {
        Self {
            hw_version: 0,
            fw_version: 0,
            dr_version: 0,
            in_version: 0,
            irq_num: 0,
            can_num: 0,
            str_serial_num: [0; 20],
            str_hw_type: [0; 40],
            reserved: [0; 4],
        }
    }
}

pub struct CanLibrary {
    _lib: Arc<Library>,
    pub vci_open_device: unsafe extern "C" fn(u32, u32, u32) -> i32,
    pub vci_close_device: unsafe extern "C" fn(u32, u32) -> i32,
    pub vci_init_can: unsafe extern "C" fn(u32, u32, u32, *const VciInitConfig) -> i32,
    pub vci_start_can: unsafe extern "C" fn(u32, u32, u32) -> i32,
    pub vci_receive: unsafe extern "C" fn(u32, u32, u32, *mut VciCanObj, u32, i32) -> i32,
    pub vci_read_board_info: unsafe extern "C" fn(u32, u32, *mut VciBoardInfo) -> i32,
}

impl CanLibrary {
    pub fn new(dll_name: &str) -> Arc<Self> {
        let lib = Arc::new(unsafe { Library::new(dll_name) }.expect("DLL load failed"));
        unsafe {
            Arc::new(Self {
                _lib: lib.clone(),
                vci_open_device: *lib
                    .get(b"VCI_OpenDevice")
                    .expect("Failed to get VCI_OpenDevice"),
                vci_close_device: *lib
                    .get(b"VCI_CloseDevice")
                    .expect("Failed to get VCI_CloseDevice"),
                vci_init_can: *lib.get(b"VCI_InitCAN").expect("Failed to get VCI_InitCAN"),
                vci_start_can: *lib
                    .get(b"VCI_StartCAN")
                    .expect("Failed to get VCI_StartCAN"),
                vci_receive: *lib.get(b"VCI_Receive").expect("Failed to get VCI_Receive"),
                vci_read_board_info: *lib
                    .get(b"VCI_ReadBoardInfo")
                    .expect("Failed to get VCI_ReadBoardInfo"),
            })
        }
    }
}

pub struct CanApp {
    pub can_lib: Arc<CanLibrary>,
    pub receiving: Arc<AtomicBool>,
    pub is_can_initialized: Arc<AtomicBool>,
}

impl CanApp {
    pub fn new() -> Self {
        let can_lib = CanLibrary::new("ControlCAN.dll");
        Self {
            can_lib,
            receiving: Arc::new(AtomicBool::new(false)),
            is_can_initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn open_device(
        &self,
        dev_type: u32,
        dev_index: u32,
        can_channel: u32,
        log_tx: Sender<String>,
    ) -> bool {
        unsafe {
            // 1. **開啟裝置**
            let status = (self.can_lib.vci_open_device)(dev_type, dev_index, 0);
            if status != 1 {
                let _ = log_tx.send(format!("裝置打開失敗, 錯誤碼: {}", status));
                return false;
            }
            let _ = log_tx.send("裝置打開成功".to_string());

            // 2. **初始化 CAN**
            let config = VciInitConfig {
                acc_code: 0,
                acc_mask: 0xFFFFFFFF,
                reserved: 0,
                filter: 1,
                timing0: 0x01, // 預設 250kbps，你可以改為對應的值
                timing1: 0x1C,
                mode: 0,
            };
            let init_status =
                (self.can_lib.vci_init_can)(dev_type, dev_index, can_channel, &config);
            if init_status != 1 {
                let err_msg = "初始化 CAN 失敗".to_string();
                let _ = log_tx.send(err_msg);
                self.is_can_initialized.store(false, Ordering::SeqCst);
                return false;
            }
            let _ = log_tx.send("CAN 初始化成功".to_string());
            self.is_can_initialized.store(true, Ordering::SeqCst);

            // 3. **讀取板卡資訊**
            let mut board_info = VciBoardInfo::default();
            let board_status =
                (self.can_lib.vci_read_board_info)(dev_type, dev_index, &mut board_info);
            if board_status != 1 {
                let err_msg = "讀取板卡資訊失敗".to_string();
                let _ = log_tx.send(err_msg);
                return false;
            }
            let serial_number = String::from_utf8_lossy(&board_info.str_serial_num)
                .trim_matches('\0')
                .to_string();
            let board_msg = format!(
                "板卡資訊: Serial={}, Firmware={}",
                serial_number, board_info.fw_version
            );
            let _ = log_tx.send(board_msg);

            true
        }
    }

    pub fn close_device(&self, dev_type: u32, dev_index: u32, log_tx: Sender<String>) {
        unsafe {
            let status = (self.can_lib.vci_close_device)(dev_type, dev_index);
            let _ = log_tx.send(format!("裝置已關閉, 狀態: {}", status));
            self.is_can_initialized.store(false, Ordering::SeqCst);
        }
    }

    pub fn start_receiving(
        &self,
        dev_type: u32,
        dev_index: u32,
        can_channel: u32,
        log_tx: Sender<String>,
        data_tx: Sender<String>,
    ) {
        let receiving_flag = Arc::clone(&self.receiving);
        let can_lib = Arc::clone(&self.can_lib);

        unsafe {
            let start_status = (can_lib.vci_start_can)(dev_type, dev_index, can_channel);
            if start_status != 1 {
                let err_msg = format!(
                    "無法啟動 CAN 通道 {}, 錯誤碼: {}",
                    can_channel, start_status
                );
                let _ = log_tx.send(err_msg);
                return;
            }
            let _ = log_tx.send(format!("CAN 通道 {} 啟動成功", can_channel));
        }

        receiving_flag.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            while receiving_flag.load(Ordering::SeqCst) {
                let mut can_obj = VciCanObj::default();
                let received_frames = unsafe {
                    (can_lib.vci_receive)(dev_type, dev_index, can_channel, &mut can_obj, 1, 500)
                };

                if received_frames > 0 {
                    let data = &can_obj.data[..(can_obj.data_len as usize)];
                    let msg = format!("ID=0x{:X}, Data={:?}", can_obj.id, data);
                    let _ = data_tx.send(msg);
                }

                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    pub fn stop_receiving(&self) {
        self.receiving.store(false, Ordering::SeqCst);
    }

    pub fn reconnect_device(
        &self,
        dev_type: u32,
        dev_index: u32,
        can_channel: u32,
        timing0: u8,
        timing1: u8,
        log_tx: Sender<String>,
    ) {
        let _ = log_tx.send("開始重設波特率...".to_string());

        // 1. **關閉裝置**
        self.close_device(dev_type, dev_index, log_tx.clone());
        thread::sleep(Duration::from_millis(100));

        // 2. **重新開啟裝置**
        if !self.open_device(dev_type, dev_index, can_channel, log_tx.clone()) {
            let _ = log_tx.send("重設波特率失敗: 無法重新開啟裝置".to_string());
            return;
        }

        // 3. **設定新波特率**
        let config = VciInitConfig {
            acc_code: 0,
            acc_mask: 0xFFFFFFFF,
            reserved: 0,
            filter: 1,
            timing0,
            timing1,
            mode: 0,
        };

        unsafe {
            let init_status =
                (self.can_lib.vci_init_can)(dev_type, dev_index, can_channel, &config);
            if init_status != 1 {
                let _ = log_tx.send("重設波特率失敗: CAN 初始化失敗".to_string());
                return;
            }
            let _ = log_tx.send(format!(
                "波特率已更新: timing0=0x{:X}, timing1=0x{:X}",
                timing0, timing1
            ));
        }

        // 4. **讀取板卡資訊**
        self.read_board_info(dev_type, dev_index, log_tx.clone());

        let _ = log_tx.send("裝置重設成功".to_string());
    }

    pub fn read_board_info(&self, dev_type: u32, dev_index: u32, log_tx: Sender<String>) {
        if !self.is_can_initialized.load(Ordering::SeqCst) {
            let _ = log_tx.send("錯誤: CAN 尚未初始化，無法讀取板卡資訊".to_string());
            return;
        }

        let mut board_info = VciBoardInfo::default();
        unsafe {
            let status = (self.can_lib.vci_read_board_info)(dev_type, dev_index, &mut board_info);
            if status != 1 {
                let _ = log_tx.send("讀取板卡資訊失敗".to_string());
                return;
            }
        }

        let serial_number = String::from_utf8_lossy(&board_info.str_serial_num)
            .trim_matches('\0')
            .to_string();

        let msg = format!(
            "板卡資訊: Serial={}, Firmware={}",
            serial_number, board_info.fw_version
        );

        let _ = log_tx.send(msg);
    }
}
