use byteorder::{BigEndian, ByteOrder};
use tiny_keccak::Keccak;

use super::{Signature, ParamType};

pub struct HashSignature {
    hash: u32,
    signature: Signature,
}

pub struct NamedSignature {
	name: &'static str,
	signature: Signature,
}

impl From<NamedSignature> for HashSignature {
	fn from(named: NamedSignature) -> HashSignature {
		let name = named.name;
		let signature = named.signature;
		let mut signature_str = String::from(name);
		signature_str.push('(');
		for (i, p) in signature.params().iter().enumerate() { 
			p.to_member(&mut signature_str);
			if i != signature.params().len()-1 { signature_str.push(','); }
		}
		signature_str.push(')');

		let mut keccak = Keccak::new_keccak256();
		let mut res = [0u8; 32];
		keccak.update(signature_str.as_bytes());
		keccak.finalize(&mut res);

		HashSignature {
			hash: BigEndian::read_u32(&res[0..4]),
			signature: signature
		}
	}
}

#[test]
fn match_signature() {

	let named = NamedSignature {
		name: "baz",
		signature: Signature::new_void(vec![ParamType::U32, ParamType::Bool]),
	};

	let hashed: HashSignature = named.into();

	assert_eq!(hashed.hash, 0xcdcd77c0);
}

#[test]
fn match_signature_2() {

	let named = NamedSignature {
		name: "sam",
		signature: Signature::new_void(vec![ParamType::Bytes, ParamType::Bool, ParamType::Array(Box::new(ParamType::U256))]),
	};

	let hashed: HashSignature = named.into();

	assert_eq!(hashed.hash, 0xa5643bf2);
}