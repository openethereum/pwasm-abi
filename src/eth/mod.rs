//! Legacy Ethereum-like ABI generator

mod param_type;
mod value_type;
mod signature;
mod encode;
mod decode;
mod util;
mod dispatch;
mod log;

pub use self::param_type::{ParamType, ArrayRef};
pub use self::value_type::ValueType;
pub use self::signature::Signature;
pub use self::util::Error;
pub use self::dispatch::{HashSignature, NamedSignature, Table};
pub use self::log::AsLog;
pub use self::encode::encode as encode_values;

use lib::*;

pub trait EndpointInterface {
	fn dispatch(&mut self, payload: &[u8]) -> Vec<u8>;
	fn dispatch_ctor(&mut self, payload: &[u8]);
}
