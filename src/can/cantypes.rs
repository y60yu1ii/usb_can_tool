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
    Kbps10,
    Kbps20,
    Kbps33_33,
    Kbps40,
    Kbps50,
    Kbps66_66,
    Kbps80,
    Kbps83_33,
    Kbps100,
    Kbps125,
    Kbps200,
    Kbps250,
    Kbps400,
    Kbps500,
    Kbps666,
    Kbps800,
    Kbps1000,
}

impl VciCanBaudRate {
    pub fn to_timing_values(self) -> (u8, u8) {
        match self {
            VciCanBaudRate::Kbps10 => (0x31, 0x1C),
            VciCanBaudRate::Kbps20 => (0x18, 0x1C),
            VciCanBaudRate::Kbps33_33 => (0x09, 0x6F),
            VciCanBaudRate::Kbps40 => (0x87, 0xFF),
            VciCanBaudRate::Kbps50 => (0x09, 0x1C),
            VciCanBaudRate::Kbps66_66 => (0x04, 0x6F),
            VciCanBaudRate::Kbps80 => (0x83, 0xFF),
            VciCanBaudRate::Kbps83_33 => (0x03, 0x6F),
            VciCanBaudRate::Kbps100 => (0x04, 0x1C),
            VciCanBaudRate::Kbps125 => (0x03, 0x1C),
            VciCanBaudRate::Kbps200 => (0x81, 0xFA),
            VciCanBaudRate::Kbps250 => (0x01, 0x1C),
            VciCanBaudRate::Kbps400 => (0x80, 0xFA),
            VciCanBaudRate::Kbps500 => (0x00, 0x1C),
            VciCanBaudRate::Kbps666 => (0x80, 0xB6),
            VciCanBaudRate::Kbps800 => (0x00, 0x16),
            VciCanBaudRate::Kbps1000 => (0x00, 0x14),
        }
    }

    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            10 => Some(VciCanBaudRate::Kbps10),
            20 => Some(VciCanBaudRate::Kbps20),
            33 => Some(VciCanBaudRate::Kbps33_33),
            40 => Some(VciCanBaudRate::Kbps40),
            50 => Some(VciCanBaudRate::Kbps50),
            66 => Some(VciCanBaudRate::Kbps66_66),
            80 => Some(VciCanBaudRate::Kbps80),
            83 => Some(VciCanBaudRate::Kbps83_33),
            100 => Some(VciCanBaudRate::Kbps100),
            125 => Some(VciCanBaudRate::Kbps125),
            200 => Some(VciCanBaudRate::Kbps200),
            250 => Some(VciCanBaudRate::Kbps250),
            400 => Some(VciCanBaudRate::Kbps400),
            500 => Some(VciCanBaudRate::Kbps500),
            666 => Some(VciCanBaudRate::Kbps666),
            800 => Some(VciCanBaudRate::Kbps800),
            1000 => Some(VciCanBaudRate::Kbps1000),
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
