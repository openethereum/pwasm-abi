use super::util;
use parity_hash::Address;
use bigint::U256;

#[derive(Debug)]
struct Error;

trait Decodable : Sized {
	fn decode(stream: &mut Stream) -> Result<Self, Error>;
	fn is_fixed() -> bool;
}

trait Encodable : Sized {
	fn encode(self, sink: &mut Sink);
	fn is_fixed() -> bool;
}

impl Decodable for u32 {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		stream.position += 32;
		if stream.position > stream.payload.len() {
			return Err(Error);
		}

		let slice = &stream.payload[stream.position-32..stream.position];

		if !slice[..28].iter().all(|x| *x == 0) {
			return Err(Error)
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
		let slc = [0u8; 32];
		sink.preamble.extend_from_slice(&util::pad_u32(self)[..]);
	}

	fn is_fixed() -> bool { true }
}

impl Decodable for Vec<u8> {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let len = u32::decode(stream)? as usize;

		let result = stream.payload[stream.position..stream.position + len].to_vec();
		stream.position = stream.position + len;
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

struct Stream<'a> {
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

struct Sink {
	preamble: Vec<u8>,
	heap: Vec<u8>,
}

impl Sink {
	fn new(capacity: usize) -> Self {
		Sink {
			preamble: Vec::with_capacity(32 * capacity),
			heap: Vec::new(),
		}
	}

	fn top_ptr(&self) -> usize {
		self.preamble.capacity() + self.heap.len()
	}

	fn push<T: Encodable>(&mut self, val: T) {
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

	fn finalize_panicking(self) -> Vec<u8> {
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

		assert_eq!(vec![0x12, 0x34], bytes);
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
			single_encode(vec![0x12, 0x34]),
			("".to_owned() +
			"0000000000000000000000000000000000000000000000000000000000000020" +
			"0000000000000000000000000000000000000000000000000000000000000002" +
			"1234000000000000000000000000000000000000000000000000000000000000")
			.from_hex().unwrap()
		);

	}
}