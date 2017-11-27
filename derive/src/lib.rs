//! Ethereum (Solidity) derivation for rust contracts (compiled to wasm or otherwise)
#![feature(alloc)]
#![feature(proc_macro)]
#![recursion_limit="128"]

extern crate proc_macro;
extern crate pwasm_abi as abi;
extern crate syn;
#[macro_use] extern crate quote;
extern crate tiny_keccak;
extern crate byteorder;
extern crate parity_hash;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

#[cfg(not(feature="std"))]
extern crate alloc;

mod items;
mod utils;
mod json;

use alloc::vec::Vec;
use proc_macro::TokenStream;

use items::Item;

/// Derive abi for given trait. Should provide two arguments - dispatch structure name and
/// client structure name.
///
/// # Example
///
/// #[eth_abi(Endpoint, Client)]
/// trait Contract { }
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

fn impl_eth_dispatch(
	item: syn::Item,
	endpoint_name: String,
	client_name: String,
) -> quote::Tokens {

	let intf = items::Interface::from_item(item)
		.client(client_name)
		.endpoint(endpoint_name);

	let ctor_branch = intf.constructor().map(
		|signature| {
			let arg_types = signature.arguments.iter().map(|&(_, ref ty)| quote! { #ty });
			quote! {
				let mut stream = ::pwasm_abi::eth::Stream::new(payload);
				self.inner.constructor(
					#(stream.pop::<#arg_types>().expect("argument decoding failed")),*
				);
			}
		}
	);

	let client_ctor = intf.constructor().map(
		|signature| utils::produce_signature(
			&signature.name,
			&signature.method_sig,
			quote! {
				#![allow(unused_mut)]
				#![allow(unused_variables)]
				unimplemented!()
			}
		)
	);

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
						#![allow(unused_mut)]
						#![allow(unused_variables)]
						let mut payload = Vec::with_capacity(4 + #argument_count_literal * 32);
						payload.push((#hash_literal >> 24) as u8);
						payload.push((#hash_literal >> 16) as u8);
						payload.push((#hash_literal >> 8) as u8);
						payload.push(#hash_literal as u8);

						let mut sink = ::pwasm_abi::eth::Sink::new(#argument_count_literal);
						#(#argument_push)*

						sink.drain_to(&mut payload);

						#result_instance

						::pwasm_std::ext::call(&self.address, self.value.clone().unwrap_or(U256::zero()), &payload, &mut result[..])
							.expect("Call failed; todo: allow handling inside contracts");

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

	let endpoint_ident: syn::Ident = intf.endpoint_name().clone().into();
	let client_ident: syn::Ident = intf.client_name().clone().into();
	let name_ident: syn::Ident = intf.name().clone().into();

	quote! {
		#intf

		pub struct #client_ident {
			address: Address,
			value: Option<U256>,
		}

		pub struct #endpoint_ident<T: #name_ident> {
			inner: T,
		}

		impl #client_ident {
			pub fn new(address: Address) -> Self {
				#client_ident {
					address: address,
					value: None,
				}
			}

			pub fn value(mut self, val: U256) -> Self {
				self.value = Some(val);
				self
			}
		}

		impl #name_ident for #client_ident {
			#client_ctor
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
			#[allow(unused_mut)]
			#[allow(unused_variables)]
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
			#[allow(unused_mut)]
			fn dispatch_ctor(&mut self, payload: &[u8]) {
				#ctor_branch
			}
		}
	}
}
