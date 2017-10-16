use lib::*;

#[derive(Debug)]
pub enum Error {
	UnknownSignature,
	NoLengthForSignature,
	NoFallback,
	ResultCantFit,
	UnexpectedEnd,
	InvalidPadding,
	InvalidUtf8,
}

pub type Hash = [u8; 32];

/// Converts u32 to right aligned array of 32 bytes.
pub fn pad_u32(value: u32) -> Hash {
	let mut padded = [0u8; 32];
	padded[28] = (value >> 24) as u8;
	padded[29] = (value >> 16) as u8;
	padded[30] = (value >> 8) as u8;
	padded[31] = value as u8;
	padded
}

/// Converts u64 to right aligned array of 32 bytes.
pub fn pad_u64(value: u64) -> Hash {
	let mut padded = [0u8; 32];
	padded[24] = (value >> 56) as u8;
	padded[25] = (value >> 48) as u8;
	padded[26] = (value >> 40) as u8;
	padded[27] = (value >> 32) as u8;
	padded[28] = (value >> 24) as u8;
	padded[29] = (value >> 16) as u8;
	padded[30] = (value >> 8) as u8;
	padded[31] = value as u8;
	padded
}

/// Converts i64 to right aligned array of 32 bytes.
pub fn pad_i64(value: i64) -> Hash {
	if value >= 0 {
		return pad_u64(value as u64);
	}

	let mut padded = [0xffu8; 32];
	padded[24] = (value >> 56) as u8;
	padded[25] = (value >> 48) as u8;
	padded[26] = (value >> 40) as u8;
	padded[27] = (value >> 32) as u8;
	padded[28] = (value >> 24) as u8;
	padded[29] = (value >> 16) as u8;
	padded[30] = (value >> 8) as u8;
	padded[31] = value as u8;
	padded
}

/// Converts i32 to right aligned array of 32 bytes.
pub fn pad_i32(value: i32) -> Hash {
	if value >= 0 {
		return pad_u32(value as u32);
	}

	let mut padded = [0xffu8; 32];
	padded[28] = (value >> 24) as u8;
	padded[29] = (value >> 16) as u8;
	padded[30] = (value >> 8) as u8;
	padded[31] = value as u8;
	padded
}

pub fn as_u32(slice: &Hash) -> Result<u32, Error> {
	if !slice[..28].iter().all(|x| *x == 0) {
		return Err(Error::InvalidPadding);
	}

	let result = ((slice[28] as u32) << 24) +
		((slice[29] as u32) << 16) +
		((slice[30] as u32) << 8) +
		(slice[31] as u32);

	Ok(result)
}

pub fn as_i32(slice: &Hash) -> Result<i32, Error> {
	let is_negative = slice[0] & 0x80 != 0;

	if !is_negative {
		return Ok(as_u32(slice)? as i32);
	}

	// only negative path here

	if !slice[1..28].iter().all(|x| *x == 0xff) {
		return Err(Error::InvalidPadding);
	}

	let result = ((slice[28] as u32) << 24) +
		((slice[29] as u32) << 16) +
		((slice[30] as u32) << 8) +
		(slice[31] as u32);

	Ok(-(result as i32))
}

pub fn as_u64(slice: &Hash) -> Result<u64, Error> {
	if !slice[..24].iter().all(|x| *x == 0) {
		return Err(Error::InvalidPadding);
	}

	let result =
		((slice[24] as u64) << 56) +
		((slice[25] as u64) << 48) +
		((slice[26] as u64) << 40) +
		((slice[27] as u64) << 32) +
		((slice[28] as u64) << 24) +
		((slice[29] as u64) << 16) +
		((slice[30] as u64) << 8) +
		 (slice[31] as u64);

	Ok(result)
}

pub fn as_i64(slice: &Hash) -> Result<i64, Error> {
	let is_negative = slice[0] & 0x80 != 0;

	if !is_negative {
		return Ok(as_u64(slice)? as i64);
	}

	// only negative path here

	if !slice[1..28].iter().all(|x| *x == 0xff) {
		return Err(Error::InvalidPadding);
	}

	let result =
		((slice[24] as u64) << 56) +
		((slice[25] as u64) << 48) +
		((slice[26] as u64) << 40) +
		((slice[27] as u64) << 32) +
		((slice[28] as u64) << 24) +
		((slice[29] as u64) << 16) +
		((slice[30] as u64) << 8) +
		 (slice[31] as u64);

	Ok(-(result as i64))
}

pub fn as_bool(slice: &Hash) -> Result<bool, Error> {
	if !slice[..31].iter().all(|x| *x == 0) {
		return Err(Error::InvalidPadding);
	}

	Ok(slice[31] == 1)
}
