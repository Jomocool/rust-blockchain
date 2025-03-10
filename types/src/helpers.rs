
use ethereum_types::U64;
use std::fmt::{Display, LowerHex};

use crate::error::TypeError;

pub fn hex_to_u64(hex: String) -> Result<U64, TypeError> {
    U64::from_str_radix(&hex, 16).map_err(|e| TypeError::HexToU64Error(e.to_string()))
}

pub fn to_hex<T>(num: T) -> String
where
    T: Display + LowerHex,
{
    format!("{:#x}", num)
}
