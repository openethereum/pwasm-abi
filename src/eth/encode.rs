//! Encode utilities

use lib::*;
use super::ValueType;
use super::util::{pad_u32, pad_i32, pad_i64, pad_u64, Hash};

fn pad_bytes(bytes: &[u8]) -> Vec<Hash> {
	let mut result = vec![pad_u32(bytes.len() as u32)];
	result.extend(pad_fixed_bytes(bytes));
	result
}

fn pad_fixed_bytes(bytes: &[u8]) -> Vec<Hash> {
	let mut result = vec![];
	let len = (bytes.len() + 31) / 32;
	for i in 0..len {
		let mut padded = [0u8; 32];

		let to_copy = match i == len - 1 {
			false => 32,
			true => match bytes.len() % 32 {
				0 => 32,
				x => x,
			},
		};

		let offset = 32 * i;
		padded[..to_copy].copy_from_slice(&bytes[offset..offset + to_copy]);
		result.push(padded);
	}

	result
}

#[derive(Debug)]
enum Mediate {
	Raw(Vec<Hash>),
	Prefixed(Vec<Hash>),
    #[allow(dead_code)] // might be used later
	FixedArray(Vec<Mediate>),
	Array(Vec<Mediate>),
}

impl Mediate {
	fn init_len(&self) -> u32 {
		match *self {
			Mediate::Raw(ref raw) => 32 * raw.len() as u32,
			Mediate::Prefixed(_) => 32,
			Mediate::FixedArray(ref nes) => nes.iter().fold(0, |acc, m| acc + m.init_len()),
			Mediate::Array(_) => 32,
		}
	}

	fn closing_len(&self) -> u32 {
		match *self {
			Mediate::Raw(_) => 0,
			Mediate::Prefixed(ref pre) => pre.len() as u32 * 32,
			Mediate::FixedArray(ref nes) => nes.iter().fold(0, |acc, m| acc + m.closing_len()),
			Mediate::Array(ref nes) => nes.iter().fold(32, |acc, m| acc + m.init_len() + m.closing_len()),
		}
	}

	fn offset_for(mediates: &[Mediate], position: usize) -> u32 {
		assert!(position < mediates.len());

		let init_len = mediates.iter().fold(0, |acc, m| acc + m.init_len());
		mediates[0..position].iter().fold(init_len, |acc, m| acc + m.closing_len())
	}

	fn init(&self, suffix_offset: u32) -> Vec<Hash> {
		match *self {
			Mediate::Raw(ref raw) => raw.clone(),
			Mediate::FixedArray(ref nes) => {
				nes.iter()
					.enumerate()
					.flat_map(|(i, m)| m.init(Mediate::offset_for(nes, i)))
					.collect()
			},
			Mediate::Prefixed(_) | Mediate::Array(_) => {
				vec![pad_u32(suffix_offset)]
			}
		}
	}

	fn closing(&self, offset: u32) -> Vec<Hash> {
		match *self {
			Mediate::Raw(_) => vec![],
			Mediate::Prefixed(ref pre) => pre.clone(),
			Mediate::FixedArray(ref nes) => {
				// offset is not taken into account, cause it would be counted twice
				// fixed array is just raw representations of similar consecutive items
				nes.iter()
					.enumerate()
					.flat_map(|(i, m)| m.closing(Mediate::offset_for(nes, i)))
					.collect()
			},
			Mediate::Array(ref nes) => {
				// + 32 added to offset represents len of the array prepanded to closing
				let prefix = vec![pad_u32(nes.len() as u32)].into_iter();

				let inits = nes.iter()
					.enumerate()
					.flat_map(|(i, m)| m.init(offset + Mediate::offset_for(nes, i) + 32));

				let closings = nes.iter()
					.enumerate()
					.flat_map(|(i, m)| m.closing(offset + Mediate::offset_for(nes, i)));

				prefix.chain(inits).chain(closings).collect()
			},
		}
	}
}

/// Encodes vector of tokens into ABI compliant vector of bytes.
pub fn encode(tokens: &[ValueType]) -> Vec<u8> {
	let mediates: Vec<Mediate> = tokens.iter()
		.map(encode_token)
		.collect();

	let inits = mediates.iter()
		.enumerate()
		.flat_map(|(i, m)| m.init(Mediate::offset_for(&mediates, i)));

	let closings = mediates.iter()
		.enumerate()
		.flat_map(|(i, m)| m.closing(Mediate::offset_for(&mediates, i)));

	inits.chain(closings)
		.flat_map(|item| item.to_vec())
		.collect()
}

fn encode_token(token: &ValueType) -> Mediate {
	match *token {
		ValueType::Address(ref address) => {
			let mut padded = [0u8; 32];
			padded[12..].copy_from_slice(address);
			Mediate::Raw(vec![padded])
		},
        ValueType::U32(val) => Mediate::Raw(vec![pad_u32(val)]),
        ValueType::U64(val) => Mediate::Raw(vec![pad_u64(val)]),
        ValueType::I32(val) => Mediate::Raw(vec![pad_i32(val)]),
        ValueType::I64(val) => Mediate::Raw(vec![pad_i64(val)]),
		ValueType::Bytes(ref bytes) => Mediate::Prefixed(pad_bytes(bytes)),
		ValueType::String(ref s) => Mediate::Prefixed(pad_bytes(s.as_bytes())),
		ValueType::U256(ref h) => Mediate::Raw(vec![h.clone()]),
		ValueType::H256(ref h) => Mediate::Raw(vec![h.clone()]),
		ValueType::Bool(b) => {
			let value = if b { 1 } else { 0 };
			Mediate::Raw(vec![pad_u32(value)])
		},
		ValueType::Array(ref values) => {
			let mediates = values.iter()
				.map(encode_token)
				.collect();

			Mediate::Array(mediates)
		},
	}
}

#[cfg(test)]
mod tests {
	extern crate rustc_hex as hex;

	use self::hex::FromHex;
	use super::super::util::pad_u32;
	use super::super::ValueType;
	use super::encode;

	#[test]
	fn encode_address() {
		let address = ValueType::Address([0x11u8; 20]);
		let encoded = encode(&vec![address]);
		let expected = "0000000000000000000000001111111111111111111111111111111111111111".from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_dynamic_array_of_addresses() {
		let address1 = ValueType::Address([0x11u8; 20]);
		let address2 = ValueType::Address([0x22u8; 20]);
		let addresses = ValueType::Array(vec![address1, address2]);
		let encoded = encode(&vec![addresses]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"0000000000000000000000001111111111111111111111111111111111111111" +
			"0000000000000000000000002222222222222222222222222222222222222222").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_two_addresses() {
		let address1 = ValueType::Address([0x11u8; 20]);
		let address2 = ValueType::Address([0x22u8; 20]);
		let encoded = encode(&vec![address1, address2]);
		let expected = ("".to_owned() +
			"0000000000000000000000001111111111111111111111111111111111111111" +
			"0000000000000000000000002222222222222222222222222222222222222222").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_dynamic_array_of_dynamic_arrays() {
		let address1 = ValueType::Address([0x11u8; 20]);
		let address2 = ValueType::Address([0x22u8; 20]);
		let array0 = ValueType::Array(vec![address1]);
		let array1 = ValueType::Array(vec![address2]);
		let dynamic = ValueType::Array(vec![array0, array1]);
		let encoded = encode(&vec![dynamic]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"0000000000000000000000000000000000000000000000000000000000000080" +
			"00000000000000000000000000000000000000000000000000000000000000c0" +
			"0000000000000000000000000000000000000000000000000000000000000001" +
			"0000000000000000000000001111111111111111111111111111111111111111" +
			"0000000000000000000000000000000000000000000000000000000000000001" +
			"0000000000000000000000002222222222222222222222222222222222222222").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_dynamic_array_of_dynamic_arrays2() {
		let address1 = ValueType::Address([0x11u8; 20]);
		let address2 = ValueType::Address([0x22u8; 20]);
		let address3 = ValueType::Address([0x33u8; 20]);
		let address4 = ValueType::Address([0x44u8; 20]);
		let array0 = ValueType::Array(vec![address1, address2]);
		let array1 = ValueType::Array(vec![address3, address4]);
		let dynamic = ValueType::Array(vec![array0, array1]);
		let encoded = encode(&vec![dynamic]);
		let expected = ("".to_owned() +
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
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_bytes() {
		let bytes = ValueType::Bytes(vec![0x12, 0x34]);
		let encoded = encode(&vec![bytes]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"1234000000000000000000000000000000000000000000000000000000000000").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_string() {
		let s = ValueType::String("gavofyork".to_owned());
		let encoded = encode(&vec![s]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000009" +
			"6761766f66796f726b0000000000000000000000000000000000000000000000").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_bytes2() {
		let bytes = ValueType::Bytes("10000000000000000000000000000000000000000000000000000000000002".from_hex().unwrap());
		let encoded = encode(&vec![bytes]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"000000000000000000000000000000000000000000000000000000000000001f" +
			"1000000000000000000000000000000000000000000000000000000000000200").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_bytes3() {
		let bytes = ValueType::Bytes(("".to_owned() +
			"1000000000000000000000000000000000000000000000000000000000000000" +
			"1000000000000000000000000000000000000000000000000000000000000000").from_hex().unwrap());
		let encoded = encode(&vec![bytes]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000040" +
			"1000000000000000000000000000000000000000000000000000000000000000" +
			"1000000000000000000000000000000000000000000000000000000000000000").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_two_bytes() {
		let bytes1 = ValueType::Bytes("10000000000000000000000000000000000000000000000000000000000002".from_hex().unwrap());
		let bytes2 = ValueType::Bytes("0010000000000000000000000000000000000000000000000000000000000002".from_hex().unwrap());
		let encoded = encode(&vec![bytes1, bytes2]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000040" +
			"0000000000000000000000000000000000000000000000000000000000000080" +
			"000000000000000000000000000000000000000000000000000000000000001f" +
			"1000000000000000000000000000000000000000000000000000000000000200" +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0010000000000000000000000000000000000000000000000000000000000002").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_uint() {
		let mut uint = [0u8; 32];
		uint[31] = 4;
		let encoded = encode(&vec![ValueType::U256(uint)]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000004").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_bool() {
		let encoded = encode(&vec![ValueType::Bool(true)]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000001").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn encode_bool2() {
		let encoded = encode(&vec![ValueType::Bool(false)]);
		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000000").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn comprehensive_test() {
		let bytes = ("".to_owned() +
			"131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b" +
			"131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b").from_hex().unwrap();
		let encoded = encode(&vec![
			ValueType::U256(pad_u32(5)),
			ValueType::Bytes(bytes.clone()),
			ValueType::U256(pad_u32(3)),
			ValueType::Bytes(bytes)
		]);

		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000005" +
			"0000000000000000000000000000000000000000000000000000000000000080" +
			"0000000000000000000000000000000000000000000000000000000000000003" +
			"00000000000000000000000000000000000000000000000000000000000000e0" +
			"0000000000000000000000000000000000000000000000000000000000000040" +
			"131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b" +
			"131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b" +
			"0000000000000000000000000000000000000000000000000000000000000040" +
			"131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b" +
			"131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}

	#[test]
	fn test_pad_u32() {
		// this will fail if endianess is not supported
		assert_eq!(pad_u32(0x1)[31], 1);
		assert_eq!(pad_u32(0x100)[30], 1);
	}

	#[test]
	fn comprehensive_test2() {
		let encoded = encode(&vec![
			ValueType::U256(pad_u32(1)),
			ValueType::String("gavofyork".to_owned()),
			ValueType::U256(pad_u32(2)),
			ValueType::U256(pad_u32(3)),
			ValueType::U256(pad_u32(4)),
			ValueType::Array(vec![
				ValueType::U256(pad_u32(5)),
				ValueType::U256(pad_u32(6)),
				ValueType::U256(pad_u32(7))
			])
		]);

		let expected = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000001" +
			"00000000000000000000000000000000000000000000000000000000000000c0" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"0000000000000000000000000000000000000000000000000000000000000003" +
			"0000000000000000000000000000000000000000000000000000000000000004" +
			"0000000000000000000000000000000000000000000000000000000000000100" +
			"0000000000000000000000000000000000000000000000000000000000000009" +
			"6761766f66796f726b0000000000000000000000000000000000000000000000" +
			"0000000000000000000000000000000000000000000000000000000000000003" +
			"0000000000000000000000000000000000000000000000000000000000000005" +
			"0000000000000000000000000000000000000000000000000000000000000006" +
			"0000000000000000000000000000000000000000000000000000000000000007").from_hex().unwrap();
		assert_eq!(encoded, expected);
	}
}

