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

#[derive(Debug, Clone, Copy)]
pub enum VciCanBaudRate {
    Baud10K,
    Baud20K,
    Baud33_33K,
    Baud40K,
    Baud50K,
    Baud66_66K,
    Baud80K,
    Baud83_33K,
    Baud100K,
    Baud125K,
    Baud200K,
    Baud250K,
    Baud400K,
    Baud500K,
    Baud666K,
    Baud800K,
    Baud1M,
}

impl VciCanBaudRate {
    pub fn to_timing_values(self) -> (u8, u8) {
        match self {
            VciCanBaudRate::Baud10K => (0x31, 0x1C),
            VciCanBaudRate::Baud20K => (0x18, 0x1C),
            VciCanBaudRate::Baud33_33K => (0x09, 0x6F),
            VciCanBaudRate::Baud40K => (0x87, 0xFF),
            VciCanBaudRate::Baud50K => (0x09, 0x1C),
            VciCanBaudRate::Baud66_66K => (0x04, 0x6F),
            VciCanBaudRate::Baud80K => (0x83, 0xFF),
            VciCanBaudRate::Baud83_33K => (0x03, 0x6F),
            VciCanBaudRate::Baud100K => (0x04, 0x1C),
            VciCanBaudRate::Baud125K => (0x03, 0x1C),
            VciCanBaudRate::Baud200K => (0x81, 0xFA),
            VciCanBaudRate::Baud250K => (0x01, 0x1C),
            VciCanBaudRate::Baud400K => (0x80, 0xFA),
            VciCanBaudRate::Baud500K => (0x00, 0x1C),
            VciCanBaudRate::Baud666K => (0x80, 0xB6),
            VciCanBaudRate::Baud800K => (0x00, 0x16),
            VciCanBaudRate::Baud1M => (0x00, 0x14),
        }
    }

    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            10 => Some(VciCanBaudRate::Baud10K),
            20 => Some(VciCanBaudRate::Baud20K),
            33 => Some(VciCanBaudRate::Baud33_33K),
            40 => Some(VciCanBaudRate::Baud40K),
            50 => Some(VciCanBaudRate::Baud50K),
            66 => Some(VciCanBaudRate::Baud66_66K),
            80 => Some(VciCanBaudRate::Baud80K),
            83 => Some(VciCanBaudRate::Baud83_33K),
            100 => Some(VciCanBaudRate::Baud100K),
            125 => Some(VciCanBaudRate::Baud125K),
            200 => Some(VciCanBaudRate::Baud200K),
            250 => Some(VciCanBaudRate::Baud250K),
            400 => Some(VciCanBaudRate::Baud400K),
            500 => Some(VciCanBaudRate::Baud500K),
            666 => Some(VciCanBaudRate::Baud666K),
            800 => Some(VciCanBaudRate::Baud800K),
            1000 => Some(VciCanBaudRate::Baud1M),
            _ => None,
        }
    }
}

// PCAN 相關結構
#[repr(C)]
#[derive(Debug, Default)]
pub struct PcanMsg {
    pub id: u32,
    pub msgtype: u8,
    pub len: u8,
    pub data: [u8; 8],
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct PcanInitConfig {
    pub baud_rate: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct PcanBoardInfo {
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

impl Default for PcanBoardInfo {
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

#[derive(Debug, Clone, Copy)]
pub enum PcanBaudRate {
    Baud1M = 0x0014,
    Baud800K = 0x0016,
    Baud500K = 0x001C,
    Baud250K = 0x011C,
    Baud125K = 0x031C,
    Baud100K = 0x432F,
    Baud95K = 0xC34E,
    Baud83K = 0x852B,
    Baud50K = 0x472F,
    Baud47K = 0x1414,
    Baud33K = 0x8B2F,
    Baud20K = 0x532F,
    Baud10K = 0x672F,
    Baud5K = 0x7F7F,
}

impl PcanBaudRate {
    /// **將 `PcanBaudRate` 轉換成 `u16` (適用於 PCAN API)**
    pub fn to_u16(self) -> u16 {
        self as u16
    }

    /// **從 `u32` 轉換成 `PcanBaudRate` (用戶輸入數字)**
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            1000 => Some(PcanBaudRate::Baud1M),
            800 => Some(PcanBaudRate::Baud800K),
            500 => Some(PcanBaudRate::Baud500K),
            250 => Some(PcanBaudRate::Baud250K),
            125 => Some(PcanBaudRate::Baud125K),
            100 => Some(PcanBaudRate::Baud100K),
            95 => Some(PcanBaudRate::Baud95K),
            83 => Some(PcanBaudRate::Baud83K),
            50 => Some(PcanBaudRate::Baud50K),
            47 => Some(PcanBaudRate::Baud47K),
            33 => Some(PcanBaudRate::Baud33K),
            20 => Some(PcanBaudRate::Baud20K),
            10 => Some(PcanBaudRate::Baud10K),
            5 => Some(PcanBaudRate::Baud5K),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CanBaudRate {
    ControlCan(VciCanBaudRate),
    Pcan(PcanBaudRate),
}
