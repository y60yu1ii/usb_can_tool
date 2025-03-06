use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::io::BufReader;

/// 整個 YAML 設定檔結構
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub components: Vec<Component>,
    pub canbus_config: Vec<CanbusConfigEntry>,
}

/// YAML 中 components 區塊，描述 UI 元件（例如 Label）
#[derive(Debug, Serialize, Deserialize)]
pub struct Component {
    #[serde(rename = "type")]
    pub comp_type: String,
    pub key: String,
    pub text: Option<String>,
    pub unit: Option<String>,
}

/// YAML 中 canbus_config 區塊，描述 CAN bus 資料萃取設定
#[derive(Debug, Serialize, Deserialize)]
pub struct CanbusConfigEntry {
    pub key: String,
    #[serde(deserialize_with = "deserialize_hex_or_decimal")]
    pub id: u32,
    pub index: u8,
    pub len: u8,
    pub endian: u8,
    #[serde(rename = "type")]
    pub data_type: String,
}

/// 自訂 Visitor 用以解析 u32，支援十進位與十六進位格式（例如 "0xF2"）
struct HexOrDecimalVisitor;

impl<'de> Visitor<'de> for HexOrDecimalVisitor {
    type Value = u32;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a u32 integer in decimal or hex format")
    }

    fn visit_u64<E>(self, value: u64) -> Result<u32, E>
    where
        E: de::Error,
    {
        Ok(value as u32)
    }

    fn visit_str<E>(self, value: &str) -> Result<u32, E>
    where
        E: de::Error,
    {
        if let Some(hex) = value.strip_prefix("0x") {
            u32::from_str_radix(hex, 16).map_err(E::custom)
        } else {
            value.parse::<u32>().map_err(E::custom)
        }
    }
}

/// 自訂反序列化函式，解析 u32 數值
pub fn deserialize_hex_or_decimal<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_any(HexOrDecimalVisitor)
}

/// 載入 YAML 設定檔，並反序列化成 Config 結構
pub fn load_config(file_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let config = serde_yaml::from_reader(reader)?;
    Ok(config)
}
