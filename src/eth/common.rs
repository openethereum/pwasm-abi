//! Common types encoding/decoding

use lib::*;
use super::{util, Stream, AbiType, Sink, Error};
use parity_hash::Address;
use bigint::U256;

impl AbiType for u32 {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let previous_position = stream.advance(32)?;

		let slice = &stream.payload()[previous_position..stream.position()];

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

	fn encode(self, sink: &mut Sink) {
		sink.preamble_mut().extend_from_slice(&util::pad_u32(self)[..]);
	}
}

impl AbiType for u64 {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let previous_position = stream.advance(32)?;

		let slice = &stream.payload()[previous_position..stream.position()];

		if !slice[..24].iter().all(|x| *x == 0) {
			return Err(Error::InvalidU64)
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

	fn is_fixed() -> bool { true }

	fn encode(self, sink: &mut Sink) {
		sink.preamble_mut().extend_from_slice(&util::pad_u64(self)[..]);
	}
}

impl AbiType for Vec<u8> {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let len = u32::decode(stream)? as usize;

		let result = stream.payload()[stream.position()..stream.position() + len].to_vec();
		stream.advance(len)?;
		stream.finish_advance();

		Ok(result)
	}

	fn is_fixed() -> bool { false }

	fn encode(self, sink: &mut Sink) {
		let mut val = self;
		let len = val.len();
		if len % 32 != 0 {
			val.resize(len + (32 - len % 32), 0);
		}
		sink.push(len as u32);
		sink.preamble_mut().extend_from_slice(&val[..]);
	}
}

impl AbiType for bool {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let decoded = u32::decode(stream)?;
		match decoded {
			0 => Ok(false),
			1 => Ok(true),
			_ => Err(Error::InvalidBool),
		}
	}

	fn is_fixed() -> bool { true }

	fn encode(self, sink: &mut Sink) {
		sink.preamble_mut().extend_from_slice(&util::pad_u32(match self { true => 1, false => 0})[..]);
	}
}

impl AbiType for U256 {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let previous = stream.advance(32)?;

		Ok(
			U256::from_big_endian(&stream.payload()[previous..stream.position()])
		)
	}

	fn encode(self, sink: &mut Sink) {
		let tail = sink.preamble_mut().len();
		sink.preamble_mut().resize(tail + 32, 0);
		self.to_big_endian(&mut sink.preamble_mut()[tail..tail+32]);
	}

	fn is_fixed() -> bool { true }
}

impl AbiType for Address {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		stream.advance(32)?;

		Ok(
			Address::from(&stream.payload()[stream.position()-20..stream.position()])
		)
	}

	fn encode(self, sink: &mut Sink) {
		let tail = sink.preamble_mut().len();
		sink.preamble_mut().resize(tail + 32, 0);
		sink.preamble_mut()[tail+12..tail+32].copy_from_slice(self.as_ref());
	}

	fn is_fixed() -> bool { true }
}

impl<T: AbiType> AbiType for Vec<T> {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {
		let len = u32::decode(stream)? as usize;
		let mut result = Vec::with_capacity(len);
		for _ in 0..len {
			result.push(stream.pop()?);
		}
		Ok(result)
	}

	fn is_fixed() -> bool { false }

	fn encode(self, sink: &mut Sink) {
		sink.push(self.len() as u32);

		for member in self.into_iter() {
			sink.push(member);
		}
	}
}

impl AbiType for i32 {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {

		let is_negative = stream.peek() & 0x80 != 0;

		if !is_negative {
			return Ok(u32::decode(stream)? as i32);
		}

		let previous_position = stream.advance(32)?;

		let slice = &stream.payload()[previous_position..stream.position()];

		// only negative path here
		if !slice[0..28].iter().all(|x| *x == 0xff) {
			return Err(Error::InvalidPadding);
		}

		let result = ((slice[28] as u32) << 24) +
			((slice[29] as u32) << 16) +
			((slice[30] as u32) << 8) +
			(slice[31] as u32);

		Ok(result as i32)
	}

	fn is_fixed() -> bool { true }

	fn encode(self, sink: &mut Sink) {
		sink.preamble_mut().extend_from_slice(&util::pad_i32(self)[..]);
	}
}


impl AbiType for i64 {
	fn decode(stream: &mut Stream) -> Result<Self, Error> {

		let is_negative = stream.peek() & 0x80 != 0;

		if !is_negative {
			return Ok(u64::decode(stream)? as i64);
		}

		let previous_position = stream.advance(32)?;

		let slice = &stream.payload()[previous_position..stream.position()];

		// only negative path here
		if !slice[0..24].iter().all(|x| *x == 0xff) {
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

		Ok(result as i64)
	}

	fn is_fixed() -> bool { true }

	fn encode(self, sink: &mut Sink) {
		sink.preamble_mut().extend_from_slice(&util::pad_i64(self)[..]);
	}
}