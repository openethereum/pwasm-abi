#![feature(alloc)]
#![feature(proc_macro)]
#![recursion_limit="128"]

extern crate proc_macro;
extern crate pwasm_abi as abi;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate tiny_keccak;
extern crate byteorder;
extern crate parity_hash;

#[cfg(not(feature="std"))]
extern crate alloc;

mod items;
mod utils;

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

fn item_to_signature(item: &Item) -> Option<abi::eth::NamedSignature> {
	match *item {
		Item::Signature(ref signature) => {
			let name = signature.name.as_ref().to_string();
			Some(
				abi::eth::NamedSignature::new(
					name,
					utils::parse_rust_signature(&signature.method_sig),
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

	let (ctor_branch, ctor_signature) = {

		let ctor_signature = signatures.iter().find(|ns| ns.name() == "ctor");

		let ctor_branch = ctor_signature.map(|ns| {
			let param_types = ns.signature().params().iter().map(|p| {
				let ident = param_type_to_ident(&p);
				quote! {
					#ident
				}
			});

			let args_line = std::iter::repeat(
				quote! { args.next().expect("Failed to fetch next argument").into() }
			).take(ns.signature().params().len());

			quote! {
				let mut args = ::pwasm_abi::eth::decode_values(&[#(#param_types),*], payload)
					.expect("abi decode failed")
					.into_iter();

				self.inner.ctor(
					#(#args_line),*
				);
			}
		});

		let ctor_dispatch_effective = ctor_signature.map(|ns|
			{
				let param_types = ns.signature().params().iter().map(|p| {
					let ident = param_type_to_ident(&p);
					quote! { #ident }
				});

				quote! {
					Some(::pwasm_abi::eth::Signature {
						params: Cow::Borrowed(&[#(#param_types),*]),
						result: None,
					})
				}
			}
		).unwrap_or(quote! { None });

		(ctor_branch, ctor_dispatch_effective)
	};

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
			Item::Signature(ref signature)  => {
				let hash_literal = syn::Lit::Int(signature.hash as u64, syn::IntTy::U32);
				let argument_push: Vec<quote::Tokens> = utils::iter_signature(&signature.method_sig)
					.map(|(pat, _)| quote! { sink.push(#pat); })
					.collect();
				let argument_count_literal = syn::Lit::Int(argument_push.len() as u64, syn::IntTy::Usize);

				let result_instance = match signature.method_sig.decl.output {
					syn::FunctionRetTy::Default => quote!{
						let mut result = Vec::new();
					},
					syn::FunctionRetTy::Ty(_) => quote!{
						let mut result = [0u8; 32];
					},
				};

				let result_pop = match signature.method_sig.decl.output {
					syn::FunctionRetTy::Default => None,
					syn::FunctionRetTy::Ty(_) => Some(quote!{
						let mut stream = ::pwasm_abi::eth::Stream::new(&result);
						stream.pop().expect("failed decode call output")
					}),
				};

				Some(utils::produce_signature(
					&signature.name,
					&signature.method_sig,
					quote!{
						let mut payload = Vec::with_capacity(4 + #argument_count_literal * 32);
						payload.push((#hash_literal >> 24) as u8);
						payload.push((#hash_literal >> 16) as u8);
						payload.push((#hash_literal >> 8) as u8);
						payload.push(#hash_literal as u8);

						let mut sink = ::pwasm_abi::eth::Sink::new(#argument_count_literal);
						#(#argument_push)*

						sink.drain_to(&mut payload);

						#result_instance

						call(&self.address, self.value.clone().unwrap_or(U256::zero()), &payload, &mut result[..]);

						#result_pop
					}
				))
			},
			Item::Event(ref event)  => {
				Some(utils::produce_signature(
					&event.name,
					&event.method_sig,
					quote!{
						#![allow(unused_variables)]
						panic!("cannot use event in client interface");
					}
				))
			},
			_ => None,
		}
	}).collect();

	let branches: Vec<quote::Tokens> = intf.items().iter().filter_map(|item| {
		match *item {
			Item::Signature(ref signature)  => {
				let hash_literal = syn::Lit::Int(signature.hash as u64, syn::IntTy::U32);
				let ident = &signature.name;
				let arg_types = signature.arguments.iter().map(|&(_, ref ty)| quote! { #ty });

				if let Some(_) = signature.return_type {
					Some(quote! {
						#hash_literal => {
							let mut stream = ::pwasm_abi::eth::Stream::new(method_payload);
							let result = inner.#ident(
								#(stream.pop::<#arg_types>().expect("argument decoding failed")),*
							);
							let mut sink = ::pwasm_abi::eth::Sink::new(1);
							sink.push(result);
							sink.finalize_panicking()
						}
					})
				} else {
					Some(quote! {
						#hash_literal => {
							let mut stream = ::pwasm_abi::eth::Stream::new(method_payload);
							inner.#ident(
								#(stream.pop::<#arg_types>().expect("argument decoding failed")),*
							);
							Vec::new()
						}
					})
				}
			},
			_ => None,
		}
	}).collect();

	let dispatch_table = quote! {
		{
			const TABLE: &'static ::pwasm_abi::eth::Table = &::pwasm_abi::eth::Table {
				inner: Cow::Borrowed(&[#(#table_signatures),*]),
				fallback: #ctor_signature,
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
				}
			}

			pub fn instance(&self) -> &T {
				&self.inner
			}
		}

		impl<T: #name_ident> ::pwasm_abi::eth::EndpointInterface for #endpoint_ident<T> {
			fn dispatch(&mut self, payload: &[u8]) -> Vec<u8> {
				let inner = &mut self.inner;
				if payload.len() < 4 {
					panic!("Invalid abi invoke");
				}
				let method_id = ((payload[0] as u32) << 24)
					+ ((payload[1] as u32) << 16)
					+ ((payload[2] as u32) << 8)
					+ (payload[3] as u32);

				let method_payload = &payload[4..];

				match method_id {
					#(#branches),*,
					_ => panic!("Invalid method signature"),
				}
			}

			#[allow(unused_variables)]
			fn dispatch_ctor(&mut self, payload: &[u8]) {
				#ctor_branch
			}
		}
	}
}
