use lib::*;

use super::{ParamType, ValueType, Error};
use super::decode::decode;
use super::encode::encode;

#[derive(Clone)]
pub struct Signature {
    pub params: Cow<'static, [ParamType]>,
    pub result: Option<ParamType>,
}

impl Signature {

    pub fn new<T>(params: T, result: Option<ParamType>) -> Self
        where T: Into<Cow<'static, [ParamType]>>
    {
        Signature {
            params: params.into(),
            result: result,
        }
    }

    pub fn new_void<T>(params: T) -> Self
        where T: Into<Cow<'static, [ParamType]>>
    {
        Signature {
            params: params.into(),
            result: None,
        }
    }

    pub fn encode_invoke(&self, args: &[ValueType]) -> Vec<u8> {
        encode(args)
    }

    pub fn decode_result(&self, payload: &[u8]) -> Result<Option<ValueType>, Error> {
        let mut result = decode(self.params.as_ref(), payload)?;
        match (&self.result, result.pop()) {
            (&Some(_), Some(val)) => {
                Ok(Some(val))
            },
            (&None, None) => Ok(None),
            _ => Err(Error::ResultCantFit),
        }
    }

    pub fn decode_invoke(&self, payload: &[u8]) -> Vec<ValueType> {
        decode(&self.params.as_ref(), payload).expect("Failed signature paring is a valid panic")
    }

    pub fn encode_result(&self, result: Option<ValueType>) -> Result<Vec<u8>, Error> {
        match (result, &self.result) {
            (Some(val), &Some(_)) => {
                Ok(encode(&[val]))
            },
            (None, &None) => Ok(Vec::new()),
            _ => Err(Error::ResultCantFit)
        }
    }

    pub fn params(&self) -> &[ParamType] {
        self.params.as_ref()
    }

    pub fn result(&self) -> Option<&ParamType> {
        self.result.as_ref()
    }
}
