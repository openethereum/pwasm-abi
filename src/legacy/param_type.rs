
/// Param type subset generatable by WASM contract
#[derive(Debug)]
pub enum ParamType {
    // Unsigned integer (mapped from u32)
    U32,
    // Unsigned integer (mapped from u64)
    U64,
    // Signed integer (mapped from i32)
    I32,
    // Signed integer (mapped from i64)
    I64,
    // Address (mapped from H160/Address)
    Address,
    // 256-bit unsigned integer (mapped from U256)
    U256,
    // 256-bit hash (mapped from H256)
    H256,
    // Byte array (mapped from Vec<u8>)
    Bytes,
    // Variable-length array (mapped from Vec<T>)
    Array(Box<ParamType>),
    // Boolean (mapped from bool)
    Bool,
    // String (mapped from String/str)
    String,
}