//! Log module

use byteorder::{BigEndian, ByteOrder};
use super::types::*;

/// As log trait for how primitive types are represented as indexed arguments
/// of the event log
pub trait AsLog {
	/// Convert type to hash representation for the event log.
	fn as_log(&self) -> H256;
}

impl AsLog for u32 {
	fn as_log(&self) -> H256 {
		let mut result = H256::zero();
		BigEndian::write_u32(&mut result.as_mut()[28..32], *self);
		result
	}
}

impl AsLog for u64 {
	fn as_log(&self) -> H256 {
		let mut result = H256::zero();
		BigEndian::write_u64(&mut result.as_mut()[24..32], *self);
		result
	}
}

impl AsLog for i64 {
	fn as_log(&self) -> H256 {
		let mut result = H256::zero();
		BigEndian::write_i64(&mut result.as_mut()[24..32], *self);
		result
	}
}

impl AsLog for i32 {
	fn as_log(&self) -> H256 {
		let mut result = H256::zero();
		BigEndian::write_i32(&mut result.as_mut()[28..32], *self);
		result
	}
}


impl AsLog for bool {
	fn as_log(&self) -> H256 {
		let mut result = H256::zero();
		result.as_mut()[32] = if *self { 1 } else { 0 };
		result
	}
}

impl AsLog for U256 {
	fn as_log(&self) -> H256 {
		let mut result = H256::zero();
		self.to_big_endian(result.as_mut());
		result
	}
}

impl AsLog for H256 {
	fn as_log(&self) -> H256 {
		self.clone()
	}
}

impl AsLog for Address {
	fn as_log(&self) -> H256 {
		(*self).into()
	}
}
