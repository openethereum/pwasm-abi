use super::{ParamType, ValueType};

pub struct Signature {
    params: Vec<ParamType>,
    result: Option<ParamType>,
}

impl Signature {

    pub fn decode_invoke(&self, payload: Vec<u8>) -> Vec<ValueType> {
        Vec::new()
    }

    pub fn encode_return(&self, result: ValueType) -> Vec<u8> {
        Vec::new()
    }
}