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
