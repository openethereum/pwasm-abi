//! Typed value module

use lib::*;

/// Typed value
#[derive(Debug, PartialEq)]
pub enum ValueType {
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    Address([u8; 20]),
    U256([u8; 32]),
    H256([u8; 32]),
    Bytes(Vec<u8>),
    Array(Vec<ValueType>),
    Bool(bool),
    String(String),
}

impl From<bool> for ValueType {
    fn from(val: bool) -> Self {
        ValueType::Bool(val)
    }
}

impl From<u32> for ValueType {
    fn from(val: u32) -> Self {
        ValueType::U32(val)
    }
}

impl From<ValueType> for u32 {
    fn from(val: ValueType) -> Self {
        match val {
            ValueType::U32(v) => v,
            // This panics here and below can only occur if something is wrong with abi generation (at compile time)
            _ => panic!("invalid abi generated for u32 argument"),
        }
    }
}

impl From<ValueType> for bool {
    fn from(val: ValueType) -> Self {
        match val {
            ValueType::Bool(v) => v,
            _ => panic!("invalid abi generated for bool argument"),
        }
    }
}