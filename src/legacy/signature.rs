use super::{ParamType, ValueType, Error};
use super::decode::decode;
use super::encode::encode;

pub struct Signature {
    params: Vec<ParamType>,
    result: Option<ParamType>,
}

impl Signature {

    pub fn new(params: Vec<ParamType>, result: Option<ParamType>) -> Signature {
        Signature {
            params: params,
            result: result,
        }
    }

    pub fn new_void(params: Vec<ParamType>) -> Signature {
        Signature {
            params: params,
            result: None,
        }
    }

    pub fn decode_invoke(&self, payload: &[u8]) -> Vec<ValueType> {
        decode(&self.params, payload).expect("Failed signature paring is a valid panic")
    }

    pub fn encode_result(&self, result: Option<ValueType>) -> Result<Vec<u8>, Error> {
        match (result, &self.result) {
            (Some(val), &Some(_)) => {
                Ok(encode(&[val]))
            },
            (None, &None) => Ok(Vec::new()),
            _ => Err(Error)
        }
    }

    pub fn params(&self) -> &[ParamType] {
        &self.params
    }
}
