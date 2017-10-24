#![feature(alloc)]
#![feature(proc_macro)]
#![recursion_limit="128"]

extern crate proc_macro;
extern crate pwasm_abi as abi;
extern crate syn;
#[macro_use]
extern crate quote;

#[cfg(not(feature="std"))]
extern crate alloc;

mod items;

use alloc::vec::Vec;
use proc_macro::TokenStream;

use items::Item;

#[proc_macro_attribute]
pub fn eth_abi(args: TokenStream, input: TokenStream) -> TokenStream {
	let args_str = args.to_string();
	let mut args: Vec<String> = args_str
		.split(',')
		.map(|w| w.trim_matches(&['(', ')', '"', ' '][..]).to_string())
		.collect();

	let client_arg = args.pop().expect("Should be 2 elements in attribute");
	let endpoint_arg = args.pop().expect("Should be 2 elements in attribute");

	let source = input.to_string();
	let ast = syn::parse_item(&source).expect("Failed to parse derive input");

	let generated = impl_eth_dispatch(ast, endpoint_arg, client_arg);

	generated.parse().expect("Failed to parse generated input")
}

fn ty_to_param_type(ty: &syn::Ty) -> abi::eth::ParamType {
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

fn parse_rust_signature(method_sig: &syn::MethodSig) -> abi::eth::Signature {
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

fn item_to_signature(item: &Item) -> Option<abi::eth::NamedSignature> {
	match *item {
		Item::Signature(ref ident, ref method_sig) => {
			let name = ident.as_ref().to_string();
			Some(
				abi::eth::NamedSignature::new(
					name,
					parse_rust_signature(method_sig),
				)
			)
		},
		_ => None,
	}
}

fn param_type_to_ident(param_type: &abi::eth::ParamType) -> quote::Tokens {
	use abi::eth::ParamType;
	match *param_type {
		ParamType::U32 => quote! { ::pwasm_abi::eth::ParamType::U32 },
		ParamType::I32 => quote! { ::pwasm_abi::eth::ParamType::U32 },
		ParamType::U64 => quote! { ::pwasm_abi::eth::ParamType::U32 },
		ParamType::I64 => quote! { ::pwasm_abi::eth::ParamType::U32 },
		ParamType::Bool => quote! { ::pwasm_abi::eth::ParamType::Bool },
		ParamType::U256 => quote! { ::pwasm_abi::eth::ParamType::U256 },
		ParamType::H256 => quote! { ::pwasm_abi::eth::ParamType::H256 },
		ParamType::Address => quote! { ::pwasm_abi::eth::ParamType::Address },
		ParamType::Bytes => quote! { ::pwasm_abi::eth::ParamType::Bytes },
		ParamType::Array(ref t) => {
			let nested = param_type_to_ident(t.as_ref());
			quote! {
				::pwasm_abi::eth::ParamType::Array(::pwasm_abi::eth::ArrayRef::Static(&#nested))
			}
		},
		ParamType::String => quote! { ::pwasm_abi::eth::ParamType::String },
	}
}

fn produce_signature<T: quote::ToTokens>(
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

fn impl_eth_dispatch(
	item: syn::Item,
	endpoint_name: String,
	client_name: String,
) -> quote::Tokens {

	let intf = items::Interface::from_item(item)
		.client(client_name)
		.endpoint(endpoint_name);

	let signatures: Vec<abi::eth::NamedSignature> =
		intf.items().iter().filter_map(item_to_signature).collect();

	let ctor_branch = signatures.iter().find(|ns| ns.name() == "ctor").map(|ns| {
		let args_line = std::iter::repeat(
			quote! { args.next().expect("Failed to fetch next argument").into() }
		).take(ns.signature().params().len());

		quote! {
			inner.ctor(
				#(#args_line),*
			);
		}
	});

	let hashed_signatures: Vec<abi::eth::HashSignature> =
		signatures.clone().into_iter()
			.map(From::from)
			.collect();

	let table_signatures = hashed_signatures.clone().into_iter().map(|hs| {
		let hash_literal = syn::Lit::Int(hs.hash() as u64, syn::IntTy::U32);

		let param_types = hs.signature().params().iter().map(|p| {
			let ident = param_type_to_ident(&p);
			quote! {
				#ident
			}
		});

		if let Some(result_type) = hs.signature().result() {
			let return_type = param_type_to_ident(result_type);
			quote! {
				::pwasm_abi::eth::HashSignature {
					hash: #hash_literal,
					signature: ::pwasm_abi::eth::Signature {
						params: Cow::Borrowed(&[#(#param_types),*]),
						result: Some(#return_type),
					}
				}
			}
		} else {
			quote! {
				::pwasm_abi::eth::HashSignature {
					hash: #hash_literal,
					signature: ::pwasm_abi::eth::Signature {
						params: Cow::Borrowed(&[#(#param_types),*]),
						result: None,
					}
				}
			}
		}
	});


	let calls: Vec<quote::Tokens> = intf.items().iter().filter_map(|item| {
		match *item {
			Item::Signature(ref ident, ref method_sig)  => {
				let signature_index = signatures.iter().position(|s| s.name() == ident.as_ref()).expect("signature with this name known to exist");
				let hash = *&hashed_signatures[signature_index].hash();
				let hash_literal = syn::Lit::Int(hash as u64, syn::IntTy::U32);

				let args = method_sig.decl.inputs.iter().filter_map(|arg| {
					match *arg {
						syn::FnArg::Captured(ref pat, _) => Some(pat),
						_ => None,
					}
				});

				let body_appendix = match method_sig.decl.output {
					syn::FunctionRetTy::Default => quote!{;},
					syn::FunctionRetTy::Ty(_) => quote!{.expect("abi should return value").into()},
				};

				Some(produce_signature(
					ident,
					method_sig,
					quote!{
						let values: &[::pwasm_abi::eth::ValueType] = &[
							#(#args.into()),*
						];
						self.table
							.call(#hash_literal, values, |payload| {
								call(&self.address, self.value.clone().unwrap_or(U256::zero()), &payload, &mut[])
									.expect("call failed");
								None
							})
							.expect("abi dispatch failed")
							#body_appendix
					}
				))
			},
			Item::Event(ref ident, ref method_sig)  => {
				Some(produce_signature(
					ident,
					method_sig,
					quote!{
						panic!("cannot use event in client interface");
					}
				))
			},
			_ => None,
		}
	}).collect();

	let branches = hashed_signatures.into_iter()
		.zip(signatures.into_iter())
		.filter_map(|(hs, ns)| {
			if ns.name() == "ctor" {
				return None;
			}

			let hash_literal = syn::Lit::Int(hs.hash() as u64, syn::IntTy::U32);
			let ident: syn::Ident = ns.name().into();

			let args_line = std::iter::repeat(
				quote! { args.next().expect("Failed to fetch next argument").into() }
			).take(hs.signature().params().len());

			if let Some(_) = hs.signature().result() {
				Some(quote! {
					#hash_literal => {
						Some(
							inner.#ident(
								#(#args_line),*
							).into()
						)
					}
				})
			} else {
				Some(quote! {
					#hash_literal => {
						inner.#ident(
							#(#args_line),*
						);
						None
					}
				})
			}
		}
	);

	let dispatch_table = quote! {
		{
			const TABLE: &'static ::pwasm_abi::eth::Table = &::pwasm_abi::eth::Table {
				inner: Cow::Borrowed(&[#(#table_signatures),*]),
				fallback: None,
			};
			TABLE
		}
	};

	let endpoint_ident: syn::Ident = intf.endpoint_name().clone().into();
	let client_ident: syn::Ident = intf.client_name().clone().into();
	let name_ident: syn::Ident = intf.name().clone().into();

	quote! {
		#intf

		pub struct #client_ident {
			address: Address,
			value: Option<U256>,
			table: &'static ::pwasm_abi::eth::Table,
		}

		pub struct #endpoint_ident<T: #name_ident> {
			inner: T,
			table: &'static ::pwasm_abi::eth::Table,
		}

		impl #client_ident {
			pub fn new(address: Address) -> Self {
				#client_ident {
					address: address,
					table: #dispatch_table,
					value: None,
				}
			}

			pub fn value(mut self, val: U256) -> Self {
				self.value = Some(val);
				self
			}
		}

		impl #name_ident for #client_ident {
			#(#calls)*
		}

		impl<T: #name_ident> #endpoint_ident<T> {
			pub fn new(inner: T) -> Self {
				#endpoint_ident {
					inner: inner,
					table: #dispatch_table,
				}
			}

			pub fn dispatch(&mut self, payload: &[u8]) -> Vec<u8> {
				let inner = &mut self.inner;
				self.table.dispatch(payload, |method_id, args| {
					let mut args = args.into_iter();
					match method_id {
				 		#(#branches),*,
						_ => panic!("Invalid method signature"),
					}
				}).expect("Failed abi dispatch")
			}

			#[allow(unused_variables)]
			pub fn dispatch_ctor(&mut self, payload: &[u8]) {
				let inner = &mut self.inner;
				self.table.fallback_dispatch(payload, |args| {
					let mut args = args.into_iter();
					#ctor_branch
				}).expect("Failed fallback abi dispatch");
			}
		}
	}
}
