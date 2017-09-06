//! Typed value module

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