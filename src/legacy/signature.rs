use super::{ParamType, ValueType};
use super::decode::decode;

pub struct Signature {
    params: Vec<ParamType>,
    result: Option<ParamType>,
}

impl Signature {

    pub fn decode_invoke(&self, payload: &[u8]) -> Vec<ValueType> {
        decode(&self.params, payload).expect("Failed signature paring is a valid panic")
    }

    pub fn encode_return(&self, result: ValueType) -> Vec<u8> {
        Vec::new()
    }
}