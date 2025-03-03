use crate::can::cantypes::*;
use flume::Sender;
use libloading::Library;
use std::ffi::c_void;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::{thread, time::Duration};

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
        can_channels: &[(u32, VciCanBaudRate)], // 接收多個通道與波特率
        log_tx: Sender<String>,
    ) -> bool {
        unsafe {
            // **1. 開啟裝置**
            let status = (self.can_lib.vci_open_device)(dev_type, dev_index, 0);
            if status != 1 {
                let _ = log_tx.send(format!("裝置打開失敗, 錯誤碼: {}", status));
                return false;
            }
            let _ = log_tx.send("裝置打開成功".to_string());

            // **2. 初始化每個 CAN 通道**
            for &(can_channel, baud_rate) in can_channels {
                let (timing0, timing1) = baud_rate.to_timing_values();
                let config = VciInitConfig {
                    acc_code: 0,
                    acc_mask: 0xFFFFFFFF,
                    reserved: 0,
                    filter: 1,
                    timing0,
                    timing1,
                    mode: 0,
                };

                let init_status =
                    (self.can_lib.vci_init_can)(dev_type, dev_index, can_channel, &config);
                if init_status != 1 {
                    let _ = log_tx.send(format!("CAN 通道 {} 初始化失敗", can_channel));

                    // **如果有任一通道初始化失敗，關閉所有已開啟的通道**
                    self.close_device(dev_type, dev_index, log_tx.clone());
                    return false;
                }
                let _ = log_tx.send(format!(
                    "CAN 通道 {} 初始化成功 (BaudRate: {:?})",
                    can_channel, baud_rate
                ));
            }

            self.is_can_initialized.store(true, Ordering::SeqCst);

            // **3. 讀取板卡資訊**
            let mut board_info = VciBoardInfo::default();
            let board_status =
                (self.can_lib.vci_read_board_info)(dev_type, dev_index, &mut board_info);
            if board_status != 1 {
                let _ = log_tx.send("讀取板卡資訊失敗".to_string());
                return false;
            }

            let serial_number = String::from_utf8_lossy(&board_info.str_serial_num)
                .trim_matches('\0')
                .to_string();
            let _ = log_tx.send(format!(
                "板卡資訊: Serial={}, Firmware={}",
                serial_number, board_info.fw_version
            ));

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
    #[allow(dead_code)]
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

const PCAN_ERROR_OK: u32 = 0;

pub struct PcanLibrary {
    _lib: Arc<Library>,
    pub can_initialize: unsafe extern "C" fn(u32, u32, u32, u32, u32) -> u32,
    pub can_uninitialize: unsafe extern "C" fn(u32) -> u32,
    pub can_read: unsafe extern "C" fn(u32, *mut PcanMsg) -> u32,
    // pub can_write: unsafe extern "C" fn(u32, *const PcanMsg) -> u32,
    pub can_get_value: unsafe extern "C" fn(u32, u32, *mut c_void, u32) -> u32,
    pub can_set_value: unsafe extern "C" fn(u32, u32, *const c_void, u32) -> u32, // <-- 新增這行
}

impl PcanLibrary {
    pub fn new(dll_name: &str) -> Arc<Self> {
        let lib = Arc::new(unsafe { Library::new(dll_name) }.expect("DLL load failed"));
        unsafe {
            Arc::new(Self {
                _lib: lib.clone(),
                can_initialize: *lib
                    .get(b"CAN_Initialize\0")
                    .expect("Failed to get CAN_Initialize"),
                can_uninitialize: *lib
                    .get(b"CAN_Uninitialize\0")
                    .expect("Failed to get CAN_Uninitialize"),
                can_read: *lib.get(b"CAN_Read\0").expect("Failed to get CAN_Read"),
                // can_write: *lib.get(b"CAN_Write\0").expect("Failed to get CAN_Write"),
                can_get_value: *lib
                    .get(b"CAN_GetValue\0")
                    .expect("Failed to get CAN_GetValue"),
                can_set_value: *lib
                    .get(b"CAN_SetValue\0") // <-- 新增這行
                    .expect("Failed to get CAN_SetValue"),
            })
        }
    }
}

/// PCAN 應用程式封裝，提供與 ControlCAN 相同的上層 API
pub struct PcanApp {
    pub can_lib: Arc<PcanLibrary>,
    pub receiving: Arc<AtomicBool>,
    pub is_can_initialized: Arc<AtomicBool>,
}

impl PcanApp {
    pub fn new() -> Self {
        let can_lib = PcanLibrary::new("PCANBasic.dll");
        Self {
            can_lib,
            receiving: Arc::new(AtomicBool::new(false)),
            is_can_initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn open_device(
        &self,
        _dev_type: u32,
        _dev_index: u32,
        channel: u32,
        baud_rate: PcanBaudRate, // 使用者選擇的 PCAN 波特率
        log_tx: Sender<String>,
    ) -> bool {
        // **先確保 PCAN 通道已釋放**
        self.force_close(log_tx.clone());

        unsafe {
            // **轉換波特率為 PCAN API 格式**
            let baudrate_value = baud_rate.to_u16();

            // **開始初始化 PCAN**
            let status = (self.can_lib.can_initialize)(channel, baudrate_value as u32, 0, 0, 0);
            if status != PCAN_ERROR_OK {
                let _ = log_tx.send(format!(
                    "PCAN device initialization failed, error code: 0x{:X}",
                    status
                ));
                return false;
            }

            let _ = log_tx.send(format!(
                "PCAN device initialized successfully with baud rate: {:?} (0x{:X})",
                baud_rate, baudrate_value
            ));
            self.is_can_initialized.store(true, Ordering::SeqCst);

            // **設置 CAN 設備參數**
            self.configure_pcan(channel, log_tx.clone());

            true
        }
    }

    fn configure_pcan(&self, channel: u32, log_tx: Sender<String>) {
        unsafe {
            // **啟用接收所有 CAN 訊息**
            const PCAN_MESSAGE_FILTER: u32 = 0x04;
            const PCAN_FILTER_OPEN: u32 = 1;
            let filter_status = (self.can_lib.can_set_value)(
                channel,
                PCAN_MESSAGE_FILTER,
                &PCAN_FILTER_OPEN as *const _ as *mut c_void,
                4,
            );
            if filter_status != PCAN_ERROR_OK {
                let _ = log_tx.send("Failed to enable message filter.".to_string());
            } else {
                let _ = log_tx.send("PCAN message filter enabled.".to_string());
            }

            // **確保 PCAN 不在 Listen-Only 模式**
            const PCAN_LISTEN_ONLY: u32 = 0x08;
            const PCAN_PARAMETER_OFF: u32 = 0;
            let listen_status = (self.can_lib.can_set_value)(
                channel,
                PCAN_LISTEN_ONLY,
                &PCAN_PARAMETER_OFF as *const _ as *mut c_void,
                4,
            );
            if listen_status != PCAN_ERROR_OK {
                let _ = log_tx.send("Failed to disable listen-only mode.".to_string());
            } else {
                let _ = log_tx.send("PCAN listen-only mode disabled.".to_string());
            }

            // **啟用 Bus-Off 自動重置**
            const PCAN_BUSOFF_AUTORESET: u32 = 0x07;
            const PCAN_PARAMETER_ON: u32 = 1;
            let reset_status = (self.can_lib.can_set_value)(
                channel,
                PCAN_BUSOFF_AUTORESET,
                &PCAN_PARAMETER_ON as *const _ as *mut c_void,
                4,
            );
            if reset_status != PCAN_ERROR_OK {
                let _ = log_tx.send("Failed to enable Bus-Off auto-reset.".to_string());
            } else {
                let _ = log_tx.send("Bus-Off auto-reset enabled.".to_string());
            }
        }
    }

    /// 關閉裝置
    pub fn close_device(
        &self,
        _dev_type: u32,
        _dev_index: u32,
        channel: u32,
        log_tx: Sender<String>,
    ) {
        unsafe {
            let status = (self.can_lib.can_uninitialize)(channel);
            let _ = log_tx.send(format!("PCAN device closed, status: {}", status));
            self.is_can_initialized.store(false, Ordering::SeqCst);
        }
    }

    pub fn force_close(&self, log_tx: Sender<String>) {
        const PCAN_NONEBUS: u32 = 0x00;
        unsafe {
            let status = (self.can_lib.can_uninitialize)(PCAN_NONEBUS);
            if status == PCAN_ERROR_OK {
                let _ = log_tx.send("All PCAN channels uninitialized successfully".to_string());
            } else {
                let _ = log_tx.send("Failed to uninitialize all PCAN channels".to_string());
            }
        }
    }

    /// 啟動接收，spawn 一個新執行緒持續呼叫 CAN_Read
    pub fn start_receiving(
        &self,
        _dev_type: u32,
        _dev_index: u32,
        channel: u32,
        log_tx: Sender<String>,
        data_tx: Sender<String>,
    ) {
        let receiving_flag = Arc::clone(&self.receiving);
        let can_lib = Arc::clone(&self.can_lib);

        let _ = log_tx.send(format!("PCAN channel 0x{:X} ready for receiving", channel));
        receiving_flag.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            while receiving_flag.load(Ordering::SeqCst) {
                let mut pcan_msg = PcanMsg::default();
                let status = unsafe { (can_lib.can_read)(channel, &mut pcan_msg) };
                if status == PCAN_ERROR_OK {
                    let data = &pcan_msg.data[..(pcan_msg.len as usize)];
                    let msg = format!("PCAN: ID=0x{:X}, Data={:?}", pcan_msg.id, data);
                    let _ = data_tx.send(msg);
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    /// 停止接收
    pub fn stop_receiving(&self) {
        self.receiving.store(false, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn read_board_info(
        &self,
        _dev_type: u32,
        _dev_index: u32,
        channel: u32,
        log_tx: Sender<String>,
    ) {
        if !self.is_can_initialized.load(Ordering::SeqCst) {
            let _ = log_tx
                .send("Error: PCAN device not initialized; cannot read board info".to_string());
            return;
        }
        // 此處以讀取 API 版本作示範
        const PCAN_PARAMETER_API_VERSION: u32 = 0x00000005;
        let mut buffer = [0u8; 24];
        let status = unsafe {
            (self.can_lib.can_get_value)(
                channel,
                PCAN_PARAMETER_API_VERSION,
                buffer.as_mut_ptr() as *mut c_void,
                24,
            )
        };
        if status == PCAN_ERROR_OK {
            let version = String::from_utf8_lossy(&buffer)
                .trim_matches('\0')
                .to_string();
            let _ = log_tx.send(format!("PCAN API Version: {}", version));
        } else {
            let _ = log_tx.send("Failed to read PCAN board info".to_string());
        }
    }
}
