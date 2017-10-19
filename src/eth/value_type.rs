//! Typed value module

use lib::*;
use bigint::U256;
use parity_hash::H256;
use parity_hash::Address;

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

impl<T: From<ValueType>> Into<Vec<T>> for ValueType {
    fn into(self) -> Vec<T> {
        match self {
            ValueType::Array(v) => v.into_iter().map(From::from).collect(),
             _ => panic!("invalid abi generated for Vec<T> argument"),
        }
    }
}

impl Into<Vec<u8>> for ValueType {
    fn into(self) -> Vec<u8> {
        match self {
             ValueType::Bytes(b) => b,
             _ => panic!("invalid abi generated for Vec<u8> argument"),
        }
    }
}

impl From<ValueType> for [u8; 32] {
    fn from(val: ValueType) -> Self {
        match val {
            ValueType::U256(v) | ValueType::H256(v) => v,
            _ => panic!("invalid abi generated for bool argument"),
        }
    }
}

impl From<ValueType> for U256 {
    fn from(val: ValueType) -> U256 {
        match val {
            ValueType::U256(v) => v.into(),
            _ => panic!("invalid abi generated for U256 argument"),
        }
    }
}

impl From<ValueType> for H256 {
    fn from(val: ValueType) -> H256 {
        match val {
            ValueType::H256(v) => v.into(),
            _ => panic!("invalid abi generated for H256 argument"),
        }
    }
}

impl From<ValueType> for Address {
    fn from(val: ValueType) -> Address {
        match val {
            ValueType::Address(v) => v.into(),
            _ => panic!("invalid abi generated for Address argument"),
        }
    }
}
