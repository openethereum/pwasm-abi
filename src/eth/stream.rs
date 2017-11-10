use super::util;
use parity_hash::Address;
use bigint::U256;

#[derive(Debug)]
pub enum Error {
	InvalidBool,
	InvalidU32,
	UnexpectedEof,
	Other,
}

pub trait Decodable : Sized {
	fn decode(stream: &mut Stream) -> Result<Self, Error>;
	fn is_fixed() -> bool;
}

pub trait Encodable : Sized {
	fn encode(self, sink: &mut Sink);
	fn is_fixed() -> bool;
}

impl Decodable for u32 {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		stream.position += 32;
		if stream.position > stream.payload.len() {
			return Err(Error::UnexpectedEof);
		}

		let slice = &stream.payload[stream.position-32..stream.position];

		if !slice[..28].iter().all(|x| *x == 0) {
			return Err(Error::InvalidU32)
		}

		let result = ((slice[28] as u32) << 24) +
			((slice[29] as u32) << 16) +
			((slice[30] as u32) << 8) +
			(slice[31] as u32);

		Ok(result)
	}

	fn is_fixed() -> bool { true }
}

impl Encodable for u32 {
	fn encode(self, sink: &mut Sink) {
		sink.preamble.extend_from_slice(&util::pad_u32(self)[..]);
	}

	fn is_fixed() -> bool { true }
}

impl Decodable for Vec<u8> {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let len = u32::decode(stream)? as usize;

		let result = stream.payload[stream.position..stream.position + len].to_vec();
		stream.position += len;
		if stream.position % 32 > 0 { stream.position += (32 - (stream.position % 32)) };
		Ok(result)
	}

	fn is_fixed() -> bool { false }
}

impl Encodable for Vec<u8> {
	fn encode(self, sink: &mut Sink) {
		let mut val = self;
		let len = val.len();
		if len % 32 != 0 {
			val.resize(len + (32 - len % 32), 0);
		}
		sink.push(len as u32);
		sink.preamble.extend_from_slice(&val[..]);
	}

	fn is_fixed() -> bool { false }
}

impl Decodable for bool {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let decoded = u32::decode(stream)?;
		match decoded {
			0 => Ok(false),
			1 => Ok(true),
			_ => Err(Error::InvalidBool),
		}
	}

	fn is_fixed() -> bool { true }
}

impl Encodable for bool {
	fn encode(self, sink: &mut Sink) {
		sink.preamble.extend_from_slice(&util::pad_u32(match self { true => 1, false => 0})[..]);
	}

	fn is_fixed() -> bool { true }
}

impl Decodable for U256 {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		stream.position += 32;
		if stream.position > stream.payload.len() {
			return Err(Error::UnexpectedEof);
		}

		Ok(
			U256::from_big_endian(&stream.payload[stream.position-32..stream.position])
		)
	}

	fn is_fixed() -> bool { true }
}

impl<T: Decodable> Decodable for Vec<T> {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let len = u32::decode(stream)? as usize;
		let mut result = Vec::with_capacity(len);
		for _ in 0..len {
			result.push(stream.pop()?);
		}
		Ok(result)
	}

	fn is_fixed() -> bool { false }
}

impl<T: Encodable> Encodable for Vec<T> {
	fn encode(self, sink: &mut Sink) {
		sink.push(self.len() as u32);

		for member in self.into_iter() {
			sink.push(member);
		}
	}

	fn is_fixed() -> bool { false }
}

pub struct Stream<'a> {
    payload: &'a [u8],
    position: usize,
}

impl<'a> Stream<'a> {
	pub fn new(raw: &'a [u8]) -> Self {
		Stream {
			payload: raw,
			position: 0,
		}
	}

	pub fn pop<T: Decodable>(&mut self) -> Result<T, Error> {
		if T::is_fixed() {
			T::decode(self)
		} else {
			let offset = u32::decode(self)?;
			let mut nested_stream = Stream::new(&self.payload[offset as usize..]);
			T::decode(&mut nested_stream)
		}
	}
}

pub struct Sink {
	preamble: Vec<u8>,
	heap: Vec<u8>,
}

impl Sink {
	pub fn new(capacity: usize) -> Self {
		Sink {
			preamble: Vec::with_capacity(32 * capacity),
			heap: Vec::new(),
		}
	}

	fn top_ptr(&self) -> usize {
		self.preamble.capacity() + self.heap.len()
	}

	pub fn push<T: Encodable>(&mut self, val: T) {
		if T::is_fixed() {
			val.encode(self)
		} else {
			let mut nested_sink = Sink::new(1);
			val.encode(&mut nested_sink);
			let top_ptr = self.top_ptr() as u32;
			nested_sink.drain_to(&mut self.heap);
			self.push(top_ptr);
		}
	}

	fn drain_to(self, target: &mut Vec<u8>) {
		let preamble = self.preamble;
		let heap = self.heap;
		target.reserve(preamble.len() + heap.len());
		target.extend_from_slice(&preamble);
		target.extend_from_slice(&heap);
	}

	pub fn finalize_panicking(self) -> Vec<u8> {
		if self.preamble.len() != self.preamble.capacity() { panic!("Underflow of pushed parameters!"); }
		let mut result = self.preamble;
		let heap = self.heap;

		result.extend_from_slice(&heap);
		result
	}
}

#[cfg(test)]
mod tests {

	extern crate rustc_hex as hex;
	use self::hex::FromHex;

	use super::*;

	#[test]
	fn simple() {
		let payload: &[u8; 32] = &[
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x45
		];

		let mut stream = Stream::new(&payload[..]);

		let val: u32 = stream.pop().unwrap();

		assert_eq!(val, 69);
	}

	#[test]
	fn bytes() {
		let encoded = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"1234000000000000000000000000000000000000000000000000000000000000")
			.from_hex().unwrap();

		let mut stream = Stream::new(&encoded);

		let bytes: Vec<u8> = stream.pop().unwrap();

		assert_eq!(vec![0x12u8, 0x34], bytes);
	}

	#[test]
	fn two_bytes() {
		let encoded = ("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000040" +
			"0000000000000000000000000000000000000000000000000000000000000080" +
			"000000000000000000000000000000000000000000000000000000000000001f" +
			"1000000000000000000000000000000000000000000000000000000000000200" +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0010000000000000000000000000000000000000000000000000000000000002"
		).from_hex().unwrap();

		let mut stream = Stream::new(&encoded);

		let bytes1: Vec<u8> = stream.pop().unwrap();
		let bytes2: Vec<u8> = stream.pop().unwrap();

		assert_eq!(bytes1, "10000000000000000000000000000000000000000000000000000000000002".from_hex().unwrap());
		assert_eq!(bytes2, "0010000000000000000000000000000000000000000000000000000000000002".from_hex().unwrap());
	}

	fn double_decode<T1: super::Decodable, T2: super::Decodable>(payload: &[u8]) -> (T1, T2) {
		let mut stream = super::Stream::new(payload);
		(
			stream.pop().expect("argument type 1 should be decoded"),
			stream.pop().expect("argument type 2 should be decoded"),
		)
	}

	fn triple_decode<T1: super::Decodable, T2: super::Decodable, T3: super::Decodable>(payload: &[u8]) -> (T1, T2, T3) {
		let mut stream = super::Stream::new(payload);
		(
			stream.pop().expect("argument type 1 should be decoded"),
			stream.pop().expect("argument type 2 should be decoded"),
			stream.pop().expect("argument type 3 should be decoded"),
		)
	}

	fn single_encode<T: super::Encodable>(val: T) -> Vec<u8> {
		let mut sink = super::Sink::new(1);
		sink.push(val);
		sink.finalize_panicking()
	}

	fn double_encode<T1: super::Encodable, T2: super::Encodable>(val1: T1, val2: T2) -> Vec<u8> {
		let mut sink = super::Sink::new(1);
		sink.push(val1);
		sink.push(val2);
		sink.finalize_panicking()
	}

	#[test]
	fn u32_encode() {
		assert_eq!(
			single_encode(69),
			vec![
				0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
				0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x45
			]
		);
	}

	#[test]
	fn bytes_encode() {
		assert_eq!(
			single_encode(vec![0x12u8, 0x34]),
			("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"1234000000000000000000000000000000000000000000000000000000000000")
			.from_hex().unwrap()
		);
	}

	#[test]
	fn sample1_decode() {
		let payload: &[u8] = &[
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x45,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
		];

		let (v1, v2) = double_decode::<u32, bool>(&payload);

		assert_eq!(v1, 69);
		assert_eq!(v2, true);
	}

	#[test]
	fn sample1_encode() {
		let sample: &[u8] = &[
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x45,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
		];

		let mut sink = Sink::new(2);
		sink.push(69u32);
		sink.push(true);

		assert_eq!(&sink.finalize_panicking()[..], &sample[..]);
	}

	#[test]
	fn sample2_decode() {
		let sample: &[u8] = &[
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x60,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xa0,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04,
			0x64, 0x61, 0x76, 0x65, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
			0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
		];

		let (v1, v2, v3) = triple_decode::<Vec<u8>, bool, Vec<U256>>(&sample);

		assert_eq!(v1, vec![100, 97, 118, 101]);
		assert_eq!(v2, true);
		assert_eq!(v3, vec![U256::from(1), U256::from(2), U256::from(3)]);
	}
}