use {syn, quote, abi};
use tiny_keccak::Keccak;
use parity_hash::H256;
use byteorder::{BigEndian, ByteOrder};

pub struct SignatureIterator<'a> {
	method_sig: &'a syn::MethodSig,
	position: usize,
}

impl<'a> Iterator for SignatureIterator<'a> {
	type Item = (syn::Pat, syn::Ty);

	fn next(&mut self) -> Option<Self::Item> {
		while self.position < self.method_sig.decl.inputs.len() {
			if let &syn::FnArg::Captured(ref pat, ref ty) = &self.method_sig.decl.inputs[self.position] {
				self.position += 1;
				return Some((pat.clone(), ty.clone()));
			} else {
				self.position += 1;
			}
		}
		None
	}
}

pub fn iter_signature(method_sig: &syn::MethodSig) -> SignatureIterator {
	SignatureIterator {
		method_sig: method_sig,
		position: 0,
	}
}

pub fn produce_signature<T: quote::ToTokens>(
	ident: &syn::Ident,
	method_sig: &syn::MethodSig,
	t: T,
) -> quote::Tokens
{
	let args = method_sig.decl.inputs.iter().filter_map(|arg| {
		match *arg {
			syn::FnArg::Captured(ref pat, ref ty) => Some(quote!{#pat: #ty}),
			_ => None,
		}
	});
	match method_sig.decl.output {
		syn::FunctionRetTy::Ty(ref output) => {
			quote!{
				fn #ident(&mut self, #(#args),*) -> #output {
					#t
				}
			}
		},
		syn::FunctionRetTy::Default => {
			quote!{
				fn #ident(&mut self, #(#args),*) {
					#t
				}
			}
		}
	}
}

pub fn push_canonical(target: &mut String, ty: &syn::Ty) {
	match *ty {
		syn::Ty::Path(None, ref path) => {
			let last_path = path.segments.last().unwrap();
			match last_path.ident.to_string().as_ref() {
				"u32" => target.push_str("uint32"),
				"i32" => target.push_str("int32"),
				"u64" => target.push_str("uint64"),
				"i64" => target.push_str("int64"),
				"U256" => target.push_str("uint256"),
				"H256" => target.push_str("uint256"),
				"Address" => target.push_str("address"),
				"Vec" => {
					match last_path.parameters {
						syn::PathParameters::AngleBracketed(ref param_data) => {
							let vec_arg = param_data.types.last().unwrap();
							if let syn::Ty::Path(None, ref nested_path) = *vec_arg {
								if "u8" == nested_path.segments.last().unwrap().ident.to_string() {
									target.push_str("bytes");
									return;
								}
							}
							push_canonical(target, vec_arg);
							target.push_str("[]")
						},
						_ => panic!("Unsupported vec arguments"),
					}
				},
				"String" => target.push_str("string"),
				"bool" => target.push_str("bool"),
				ref val @ _ => panic!("Unable to handle param of type {}: not supported by abi", val)
			}
		},
		ref val @ _ => panic!("Unable to handle param of type {:?}: not supported by abi", val),
	};
}

pub fn canonical(name: &syn::Ident, method_sig: &syn::MethodSig) -> String {
	let mut s = String::new();
	s.push('(');
	s.push_str(&name.to_string());
	let total_len = method_sig.decl.inputs.len();
	for (i, (_, ty)) in iter_signature(method_sig).enumerate() {
		push_canonical(&mut s, &ty);
		if i != total_len - 1 { s.push(','); }
	}
	s
}

pub fn hash(s: &str) -> u32 {
	let mut keccak = Keccak::new_keccak256();
	let mut res = H256::zero();
	keccak.update(s.as_bytes());
	keccak.finalize(res.as_mut());

	BigEndian::read_u32(&res.as_ref()[0..4])
}

pub fn ty_to_param_type(ty: &syn::Ty) -> abi::eth::ParamType {
	match *ty {
		syn::Ty::Path(None, ref path) => {
			let last_path = path.segments.last().unwrap();
			match last_path.ident.to_string().as_ref() {
				"u32" => abi::eth::ParamType::U32,
				"i32" => abi::eth::ParamType::I32,
				"u64" => abi::eth::ParamType::U64,
				"i64" => abi::eth::ParamType::I64,
				"U256" => abi::eth::ParamType::U256,
				"H256" => abi::eth::ParamType::H256,
				"Address" => abi::eth::ParamType::Address,
				"Vec" => {
					match last_path.parameters {
						syn::PathParameters::AngleBracketed(ref param_data) => {
							let vec_arg = param_data.types.last().unwrap();
							if let syn::Ty::Path(None, ref nested_path) = *vec_arg {
								if "u8" == nested_path.segments.last().unwrap().ident.to_string() {
									return abi::eth::ParamType::Bytes;
								}
							}
							abi::eth::ParamType::Array(ty_to_param_type(vec_arg).into())
						},
						_ => panic!("Unsupported vec arguments"),
					}
				},
				"String" => abi::eth::ParamType::String,
				"bool" => abi::eth::ParamType::Bool,
				ref val @ _ => panic!("Unable to handle param of type {}: not supported by abi", val)
			}
		},
		ref val @ _ => panic!("Unable to handle param of type {:?}: not supported by abi", val),
	}
}

pub fn parse_rust_signature(method_sig: &syn::MethodSig) -> abi::eth::Signature {
	let mut params = Vec::new();

	for fn_arg in method_sig.decl.inputs.iter() {
		match *fn_arg {
			syn::FnArg::Captured(_, ref ty) => {
				params.push(ty_to_param_type(ty));
			},
			syn::FnArg::SelfValue(_) => { panic!("cannot use self by value"); },
			_ => {},
		}
	}

	abi::eth::Signature::new(
		params,
		match method_sig.decl.output {
			syn::FunctionRetTy::Default => None,
			syn::FunctionRetTy::Ty(ref ty) => Some(ty_to_param_type(ty)),
		}
	)
}