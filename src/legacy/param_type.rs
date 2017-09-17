
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

impl ParamType {
	pub fn to_member(&self, s: &mut String) {
		match *self {
			ParamType::I32 => s.push_str("int32"),
			ParamType::U32 => s.push_str("uint32"),
			ParamType::I64 => s.push_str("int64"),
			ParamType::U64 => s.push_str("uint64"),
			ParamType::Address => s.push_str("uint160"),
			ParamType::U256 => s.push_str("uint256"),
			ParamType::H256 => s.push_str("uint256"),
			ParamType::H256 => s.push_str("uint256"),
			ParamType::Bytes => s.push_str("bytes"),
			ParamType::Bool => s.push_str("bool"),
			ParamType::String => s.push_str("string"),
			ParamType::Array(ref p_n) => { p_n.to_member(s); s.push_str("[]"); },
		}
	}
}