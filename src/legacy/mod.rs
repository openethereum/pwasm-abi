//! Legacy Ethereum-like ABI generator

mod param_type;
mod value_type;
mod signature;
mod encode;
mod decode;
mod util;
mod dispatch;

pub use self::param_type::ParamType;
pub use self::value_type::ValueType;
pub use self::signature::Signature;
pub use self::util::Error;
pub use self::dispatch::{HashSignature, NamedSignature, Table};

pub trait LegacyDispatch {
	fn dispatch(&mut self, payload: super::lib::Vec<u8>) -> super::lib::Vec<u8>;
}