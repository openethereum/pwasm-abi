//! Payload decoder according to signature in legacy (ethereum) ABI
//! Original code is mostly by debris in ethabi

use super::util::{as_bool, as_i32, as_u32, as_u64, as_i64, Error, Hash};
use super::{ValueType, ParamType};

/// Decodes ABI compliant vector of bytes into vector of runtime values
pub fn decode(types: &[ParamType], data: &[u8]) -> Result<Vec<ValueType>, Error> {
	let slices = slice_data(data)?;
	let mut tokens = vec![];
	let mut offset = 0;
	for param in types {
		let res = decode_param(param, &slices, offset)?;
		offset = res.new_offset;
		tokens.push(res.token);
	}
	Ok(tokens)
}

struct DecodeResult {
	token: ValueType,
	new_offset: usize,
}

struct BytesTaken {
	bytes: Vec<u8>,
	#[allow(dead_code)] // will be used later probably
	new_offset: usize,
}

/// Convers vector of bytes with len equal n * 32, to a vector of slices.
fn slice_data(data: &[u8]) -> Result<Vec<Hash>, Error> {
	if data.len() % 32 != 0 {
		return Err(Error);
	}

	let times = data.len() / 32;
	let mut result = vec![];
	for i in 0..times {
		let mut slice = [0u8; 32];
		let offset = 32 * i;
		slice.copy_from_slice(&data[offset..offset + 32]);
		result.push(slice);
	}
	Ok(result)
}

fn peek(slices: &[Hash], position: usize) -> Result<&Hash, Error> {
	slices.get(position).ok_or(Error)
}

fn take_bytes(slices: &[Hash], position: usize, len: usize) -> Result<BytesTaken, Error> {
	let slices_len = (len + 31) / 32;

	let mut bytes_slices = vec![];
	for i in 0..slices_len {
		let slice = try!(peek(slices, position + i)).clone();
		bytes_slices.push(slice);
	}

	let bytes = bytes_slices.into_iter()
		.flat_map(|slice| slice.to_vec())
		.take(len)
		.collect();

	let taken = BytesTaken {
		bytes: bytes,
		new_offset: position + slices_len,
	};

	Ok(taken)
}

fn decode_param(param: &ParamType, slices: &[Hash], offset: usize) -> Result<DecodeResult, Error> {
	match *param {
		ParamType::Address => {
			let slice = try!(peek(slices, offset));
			let mut address = [0u8; 20];
			address.copy_from_slice(&slice[12..]);

			let result = DecodeResult {
				token: ValueType::Address(address),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::U32 => {
			let slice = try!(peek(slices, offset));

			let result = DecodeResult {
				token: ValueType::U32(as_u32(slice)?),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::U64 => {
			let slice = peek(slices, offset)?;

			let result = DecodeResult {
				token: ValueType::U64(as_u64(slice)?),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::I32 => {
			let slice = peek(slices, offset)?;

			let result = DecodeResult {
				token: ValueType::I32(as_i32(slice)?),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::I64 => {
			let slice = peek(slices, offset)?;

			let result = DecodeResult {
				token: ValueType::I64(as_i64(slice)?),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::U256 => {
			let slice = peek(slices, offset)?;

			let result = DecodeResult {
				token: ValueType::U256(slice.clone()),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::H256 => {
			let slice = peek(slices, offset)?;

			let result = DecodeResult {
				token: ValueType::U256(slice.clone()),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::Bool => {
			let slice = peek(slices, offset)?;

			let b = as_bool(slice)?;

			let result = DecodeResult {
				token: ValueType::Bool(b),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::Bytes => {
			let offset_slice = peek(slices, offset)?;
			let len_offset = (try!(as_u32(offset_slice)) / 32) as usize;

			let len_slice = try!(peek(slices, len_offset));
			let len = try!(as_u32(len_slice)) as usize;

			let taken = try!(take_bytes(slices, len_offset + 1, len));

			let result = DecodeResult {
				token: ValueType::Bytes(taken.bytes),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::String => {
			let offset_slice = try!(peek(slices, offset));
			let len_offset = (try!(as_u32(offset_slice)) / 32) as usize;

			let len_slice = try!(peek(slices, len_offset));
			let len = try!(as_u32(len_slice)) as usize;

			let taken = try!(take_bytes(slices, len_offset + 1, len));

			let result = DecodeResult {
				token: ValueType::String(String::from_utf8(taken.bytes).map_err(|_| Error)?),
				new_offset: offset + 1,
			};

			Ok(result)
		},
		ParamType::Array(ref t) => {
			let offset_slice = try!(peek(slices, offset));
			let len_offset = (try!(as_u32(offset_slice)) / 32) as usize;

			let len_slice = try!(peek(slices, len_offset));
			let len = try!(as_u32(len_slice)) as usize;

			let mut tokens = vec![];
			let mut new_offset = len_offset + 1;

			for _ in 0..len {
				let res = try!(decode_param(t, &slices, new_offset));
				new_offset = res.new_offset;
				tokens.push(res.token);
			}

			let result = DecodeResult {
				token: ValueType::Array(tokens),
				new_offset: offset + 1,
			};

			Ok(result)
		},
	}
}

#[cfg(test)]
mod tests {
	extern crate rustc_hex as hex;

	use self::hex::FromHex;
	use super::decode;
    use super::super::{ValueType, ParamType};

	#[test]
	fn decode_address() {
		let encoded = "0000000000000000000000001111111111111111111111111111111111111111".from_hex().unwrap();
		let address = ValueType::Address([0x11u8; 20]);
		let expected = vec![address];
		let decoded = decode(&[ParamType::Address], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_two_address() {
		let encoded = ("".to_owned() +
					   "0000000000000000000000001111111111111111111111111111111111111111" +
					   "0000000000000000000000002222222222222222222222222222222222222222").from_hex().unwrap();
		let address1 = ValueType::Address([0x11u8; 20]);
		let address2 = ValueType::Address([0x22u8; 20]);
		let expected = vec![address1, address2];
		let decoded = decode(&[ParamType::Address, ParamType::Address], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_uint() {
		let encoded = "1111111111111111111111111111111111111111111111111111111111111111".from_hex().unwrap();
		let uint = ValueType::U256([0x11u8; 32]);
		let expected = vec![uint];
		let decoded = decode(&[ParamType::U256], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_dynamic_array_of_addresses() {
		let encoded = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"0000000000000000000000001111111111111111111111111111111111111111" +
			"0000000000000000000000002222222222222222222222222222222222222222").from_hex().unwrap();
		let address1 = ValueType::Address([0x11u8; 20]);
		let address2 = ValueType::Address([0x22u8; 20]);
		let addresses = ValueType::Array(vec![address1, address2]);
		let expected = vec![addresses];
		let decoded = decode(&[ParamType::Array(Box::new(ParamType::Address))], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}


	#[test]
	fn decode_dynamic_array_of_dynamic_arrays() {
		let encoded  = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"0000000000000000000000000000000000000000000000000000000000000080" +
			"00000000000000000000000000000000000000000000000000000000000000c0" +
			"0000000000000000000000000000000000000000000000000000000000000001" +
			"0000000000000000000000001111111111111111111111111111111111111111" +
			"0000000000000000000000000000000000000000000000000000000000000001" +
			"0000000000000000000000002222222222222222222222222222222222222222").from_hex().unwrap();

		let address1 = ValueType::Address([0x11u8; 20]);
		let address2 = ValueType::Address([0x22u8; 20]);
		let array0 = ValueType::Array(vec![address1]);
		let array1 = ValueType::Array(vec![address2]);
		let dynamic = ValueType::Array(vec![array0, array1]);
		let expected = vec![dynamic];
		let decoded = decode(&[
			ParamType::Array(Box::new(
				ParamType::Array(Box::new(ParamType::Address))
			))
		], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_dynamic_array_of_dynamic_arrays2() {
		let encoded = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"0000000000000000000000000000000000000000000000000000000000000080" +
			"00000000000000000000000000000000000000000000000000000000000000e0" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"0000000000000000000000001111111111111111111111111111111111111111" +
			"0000000000000000000000002222222222222222222222222222222222222222" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"0000000000000000000000003333333333333333333333333333333333333333" +
			"0000000000000000000000004444444444444444444444444444444444444444").from_hex().unwrap();

		let address1 = ValueType::Address([0x11u8; 20]);
		let address2 = ValueType::Address([0x22u8; 20]);
		let address3 = ValueType::Address([0x33u8; 20]);
		let address4 = ValueType::Address([0x44u8; 20]);
		let array0 = ValueType::Array(vec![address1, address2]);
		let array1 = ValueType::Array(vec![address3, address4]);
		let dynamic = ValueType::Array(vec![array0, array1]);
		let expected = vec![dynamic];
		let decoded = decode(&[
			ParamType::Array(Box::new(
				ParamType::Array(Box::new(ParamType::Address))
			))
		], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_bytes() {
		let encoded = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"1234000000000000000000000000000000000000000000000000000000000000").from_hex().unwrap();
		let bytes = ValueType::Bytes(vec![0x12, 0x34]);
		let expected = vec![bytes];
		let decoded = decode(&[ParamType::Bytes], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_bytes2() {
		let encoded = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000040" +
			"1000000000000000000000000000000000000000000000000000000000000000" +
			"1000000000000000000000000000000000000000000000000000000000000000").from_hex().unwrap();
		let bytes = ValueType::Bytes(("".to_owned() +
			"1000000000000000000000000000000000000000000000000000000000000000" +
			"1000000000000000000000000000000000000000000000000000000000000000").from_hex().unwrap());
		let expected = vec![bytes];
		let decoded = decode(&[ParamType::Bytes], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_two_bytes() {
		let encoded = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000040" +
			"0000000000000000000000000000000000000000000000000000000000000080" +
			"000000000000000000000000000000000000000000000000000000000000001f" +
			"1000000000000000000000000000000000000000000000000000000000000200" +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0010000000000000000000000000000000000000000000000000000000000002").from_hex().unwrap();
		let bytes1 = ValueType::Bytes("10000000000000000000000000000000000000000000000000000000000002".from_hex().unwrap());
		let bytes2 = ValueType::Bytes("0010000000000000000000000000000000000000000000000000000000000002".from_hex().unwrap());
		let expected = vec![bytes1, bytes2];
		let decoded = decode(&[ParamType::Bytes, ParamType::Bytes], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_string() {
		let encoded = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000009" +
			"6761766f66796f726b0000000000000000000000000000000000000000000000").from_hex().unwrap();
		let s = ValueType::String("gavofyork".to_owned());
		let expected = vec![s];
		let decoded = decode(&[ParamType::String], &encoded).unwrap();
		assert_eq!(decoded, expected);
	}
}

