use evdev::KeyCode;
use serde::{Deserialize, Deserializer, Serializer};
use std::str::FromStr;

pub fn serialize<S>(key: &KeyCode, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("{:?}", key))
}

pub fn deserialize<'de, D>(d: D) -> Result<KeyCode, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    KeyCode::from_str(&s).map_err(serde::de::Error::custom)
}
