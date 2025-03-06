use crate::can::cantypes::*;
use flume::Sender;
use libloading::Library;
use std::ffi::c_void;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::{thread, time::Duration};

const SUCCESS: i32 = 1;
const PCAN_ERROR_OK: u32 = 0;

pub trait CanInterface {
    fn open_device(&self, log_tx: Sender<String>) -> Result<(), String>;
    fn close_device(&self, log_tx: Sender<String>);
    fn start_receiving(&self, log_tx: Sender<String>, data_tx: Sender<String>);
    fn stop_receiving(&self);
    fn read_board_info(&self, log_tx: Sender<String>);
}

/// 封裝 ControlCAN 動態函式庫
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
    dev_type: u32,
    dev_index: u32,
    can_channels: Vec<(u32, VciCanBaudRate)>,
}

impl CanApp {
    pub fn new(dev_type: u32, dev_index: u32, can_channels: Vec<(u32, VciCanBaudRate)>) -> Self {
        let can_lib = CanLibrary::new("ControlCAN.dll");
        Self {
            can_lib,
            receiving: Arc::new(AtomicBool::new(false)),
            is_can_initialized: Arc::new(AtomicBool::new(false)),
            dev_type,
            dev_index,
            can_channels,
        }
    }

    unsafe fn open_device_unsafe(&self) -> Result<(), String> {
        let status = (self.can_lib.vci_open_device)(self.dev_type, self.dev_index, 0);
        if status != SUCCESS {
            Err(format!("Device open failed, Error Code: {}", status))
        } else {
            Ok(())
        }
    }

    unsafe fn init_channel(&self, channel: u32, baud_rate: VciCanBaudRate) -> Result<(), String> {
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
            (self.can_lib.vci_init_can)(self.dev_type, self.dev_index, channel, &config);
        if init_status != SUCCESS {
            Err(format!("CAN Ch {} initialization failed", channel))
        } else {
            Ok(())
        }
    }

    unsafe fn read_board_info_unsafe(&self) -> Result<VciBoardInfo, String> {
        let mut board_info = VciBoardInfo::default();
        let board_status =
            (self.can_lib.vci_read_board_info)(self.dev_type, self.dev_index, &mut board_info);
        if board_status != SUCCESS {
            Err("Read board failed".to_string())
        } else {
            Ok(board_info)
        }
    }
}

impl CanInterface for CanApp {
    fn open_device(&self, log_tx: Sender<String>) -> Result<(), String> {
        unsafe {
            self.open_device_unsafe().map_err(|e| {
                let _ = log_tx.send(e.clone());
                e
            })?;
            let _ = log_tx.send("Device opened successfully".to_string());
        }

        for &(channel, baud_rate) in &self.can_channels {
            unsafe {
                self.init_channel(channel, baud_rate).map_err(|e| {
                    let _ = log_tx.send(e.clone());
                    self.close_device(log_tx.clone());
                    e
                })?;
                let _ = log_tx.send(format!(
                    "CAN Ch {} initialized (BaudRate: {:?})",
                    channel, baud_rate
                ));
            }
        }

        self.is_can_initialized.store(true, Ordering::SeqCst);

        unsafe {
            match self.read_board_info_unsafe() {
                Ok(board_info) => {
                    let serial_number = String::from_utf8_lossy(&board_info.str_serial_num)
                        .trim_matches('\0')
                        .to_string();
                    let _ = log_tx.send(format!(
                        "Board info: Serial={}, Firmware={}",
                        serial_number, board_info.fw_version
                    ));
                }
                Err(e) => {
                    let _ = log_tx.send(e);
                    return Err("Failed to read board info".to_string());
                }
            }
        }

        Ok(())
    }

    fn close_device(&self, log_tx: Sender<String>) {
        unsafe {
            let status = (self.can_lib.vci_close_device)(self.dev_type, self.dev_index);
            let _ = log_tx.send(format!("Device closed, Status: {}", status));
            self.is_can_initialized.store(false, Ordering::SeqCst);
        }
    }

    fn start_receiving(&self, log_tx: Sender<String>, data_tx: Sender<String>) {
        let receiving_flag = Arc::clone(&self.receiving);
        let can_lib = Arc::clone(&self.can_lib);
        let dev_type = self.dev_type;
        let dev_index = self.dev_index;

        for &(channel, _) in &self.can_channels {
            let log_tx_clone = log_tx.clone();
            let data_tx_clone = data_tx.clone();
            let receiving_flag_clone = Arc::clone(&receiving_flag);
            let can_lib_clone = Arc::clone(&can_lib);

            unsafe {
                let start_status = (can_lib_clone.vci_start_can)(dev_type, dev_index, channel);
                if start_status != SUCCESS {
                    let _ = log_tx_clone.send(format!(
                        "CAN start failed on channel {}, Error Code: {}",
                        channel, start_status
                    ));
                    continue;
                }
                let _ = log_tx_clone.send(format!("CAN Ch {} started", channel));
            }

            receiving_flag_clone.store(true, Ordering::SeqCst);

            thread::spawn(move || {
                while receiving_flag_clone.load(Ordering::SeqCst) {
                    let mut can_obj = VciCanObj::default();
                    let received_frames = unsafe {
                        (can_lib_clone.vci_receive)(
                            dev_type,
                            dev_index,
                            channel,
                            &mut can_obj,
                            1,
                            500,
                        )
                    };
                    if received_frames > 0 {
                        let data = &can_obj.data[..(can_obj.data_len as usize)];
                        let msg = format!("CH={} ID=0x{:X}, Data={:?}", channel, can_obj.id, data);
                        let _ = data_tx_clone.send(msg);
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                let _ = log_tx_clone.send(format!("CAN Ch {} stopped receiving", channel));
            });
        }
    }

    fn stop_receiving(&self) {
        self.receiving.store(false, Ordering::SeqCst);
    }

    fn read_board_info(&self, log_tx: Sender<String>) {
        if !self.is_can_initialized.load(Ordering::SeqCst) {
            let _ = log_tx.send("Error: CAN not initialized; cannot read board info".to_string());
            return;
        }
        unsafe {
            match self.read_board_info_unsafe() {
                Ok(board_info) => {
                    let serial_number = String::from_utf8_lossy(&board_info.str_serial_num)
                        .trim_matches('\0')
                        .to_string();
                    let _ = log_tx.send(format!(
                        "Board info: Serial={}, Firmware={}",
                        serial_number, board_info.fw_version
                    ));
                }
                Err(e) => {
                    let _ = log_tx.send(e);
                }
            }
        }
    }
}

pub struct PcanLibrary {
    _lib: Arc<Library>,
    pub can_initialize: unsafe extern "C" fn(u32, u32, u32, u32, u32) -> u32,
    pub can_uninitialize: unsafe extern "C" fn(u32) -> u32,
    pub can_read: unsafe extern "C" fn(u32, *mut PcanMsg) -> u32,
    pub can_get_value: unsafe extern "C" fn(u32, u32, *mut c_void, u32) -> u32,
    pub can_set_value: unsafe extern "C" fn(u32, u32, *const c_void, u32) -> u32,
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
                can_get_value: *lib
                    .get(b"CAN_GetValue\0")
                    .expect("Failed to get CAN_GetValue"),
                can_set_value: *lib
                    .get(b"CAN_SetValue\0")
                    .expect("Failed to get CAN_SetValue"),
            })
        }
    }
}

pub struct PcanApp {
    pub can_lib: Arc<PcanLibrary>,
    pub receiving: Arc<AtomicBool>,
    pub is_can_initialized: Arc<AtomicBool>,
    channel: u32,
    baud_rate: PcanBaudRate,
}

impl PcanApp {
    pub fn new(channel: u32, baud_rate: PcanBaudRate) -> Self {
        let can_lib = PcanLibrary::new("PCANBasic.dll");
        Self {
            can_lib,
            receiving: Arc::new(AtomicBool::new(false)),
            is_can_initialized: Arc::new(AtomicBool::new(false)),
            channel,
            baud_rate,
        }
    }

    unsafe fn initialize_channel(&self) -> Result<(), String> {
        self.force_close_internal();
        let baudrate_value = self.baud_rate.to_u16() as u32;
        let status = (self.can_lib.can_initialize)(self.channel, baudrate_value, 0, 0, 0);
        if status != PCAN_ERROR_OK {
            Err(format!(
                "PCAN initialization failed, error code: 0x{:X}",
                status
            ))
        } else {
            Ok(())
        }
    }

    unsafe fn configure_channel(&self, log_tx: &Sender<String>) {
        // 啟用接收所有訊息
        const PCAN_MESSAGE_FILTER: u32 = 0x04;
        const PCAN_FILTER_OPEN: u32 = 1;
        let filter_status = (self.can_lib.can_set_value)(
            self.channel,
            PCAN_MESSAGE_FILTER,
            &PCAN_FILTER_OPEN as *const _ as *const c_void,
            4,
        );
        if filter_status != PCAN_ERROR_OK {
            let _ = log_tx.send("Failed to enable message filter.".to_string());
        } else {
            let _ = log_tx.send("PCAN message filter enabled.".to_string());
        }

        // 關閉 Listen-Only 模式
        const PCAN_LISTEN_ONLY: u32 = 0x08;
        const PCAN_PARAMETER_OFF: u32 = 0;
        let listen_status = (self.can_lib.can_set_value)(
            self.channel,
            PCAN_LISTEN_ONLY,
            &PCAN_PARAMETER_OFF as *const _ as *const c_void,
            4,
        );
        if listen_status != PCAN_ERROR_OK {
            let _ = log_tx.send("Failed to disable listen-only mode.".to_string());
        } else {
            let _ = log_tx.send("PCAN listen-only mode disabled.".to_string());
        }

        // 啟用 Bus-Off 自動重置
        const PCAN_BUSOFF_AUTORESET: u32 = 0x07;
        const PCAN_PARAMETER_ON: u32 = 1;
        let reset_status = (self.can_lib.can_set_value)(
            self.channel,
            PCAN_BUSOFF_AUTORESET,
            &PCAN_PARAMETER_ON as *const _ as *const c_void,
            4,
        );
        if reset_status != PCAN_ERROR_OK {
            let _ = log_tx.send("Failed to enable Bus-Off auto-reset.".to_string());
        } else {
            let _ = log_tx.send("Bus-Off auto-reset enabled.".to_string());
        }
    }

    /// 強制關閉所有 PCAN 頻道（內部呼叫）
    fn force_close_internal(&self) {
        const PCAN_NONEBUS: u32 = 0x00;
        unsafe {
            let _ = (self.can_lib.can_uninitialize)(PCAN_NONEBUS);
        }
    }
}

impl CanInterface for PcanApp {
    fn open_device(&self, log_tx: Sender<String>) -> Result<(), String> {
        unsafe {
            self.initialize_channel().map_err(|e| {
                let _ = log_tx.send(e.clone());
                e
            })?;
            let _ = log_tx.send(format!(
                "PCAN channel 0x{:X} initialized with baud rate: {:?}",
                self.channel, self.baud_rate
            ));
            self.is_can_initialized.store(true, Ordering::SeqCst);
            self.configure_channel(&log_tx);
        }
        Ok(())
    }

    fn close_device(&self, log_tx: Sender<String>) {
        unsafe {
            let status = (self.can_lib.can_uninitialize)(self.channel);
            let _ = log_tx.send(format!("PCAN device closed, status: {}", status));
            self.is_can_initialized.store(false, Ordering::SeqCst);
        }
    }

    fn start_receiving(&self, log_tx: Sender<String>, data_tx: Sender<String>) {
        let receiving_flag = Arc::clone(&self.receiving);
        let can_lib = Arc::clone(&self.can_lib);
        let channel = self.channel;
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

    fn stop_receiving(&self) {
        self.receiving.store(false, Ordering::SeqCst);
    }

    fn read_board_info(&self, log_tx: Sender<String>) {
        if !self.is_can_initialized.load(Ordering::SeqCst) {
            let _ = log_tx
                .send("Error: PCAN device not initialized; cannot read board info".to_string());
            return;
        }
        const PCAN_PARAMETER_API_VERSION: u32 = 0x00000005;
        let mut buffer = [0u8; 24];
        let status = unsafe {
            (self.can_lib.can_get_value)(
                self.channel,
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
