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

fn parse_args(args: TokenStream) -> Vec<String> {
	args.to_string()
		.split(',')
		.map(|w| w.trim_matches(&['(', ')', '"', ' '][..]).to_string())
		.collect()
}

/// Derive abi for given trait. Should provide one or two arguments:
/// dispatch structure name and client structure name.
///
/// # Example
///
/// #[eth_abi(Endpoint)]
/// trait Contract { }
///
/// # Example
///
/// #[eth_abi(Endpoint2, Client2)]
/// trait Contract2 { }
#[proc_macro_attribute]
pub fn eth_abi(args: TokenStream, input: TokenStream) -> TokenStream {
	let source = input.to_string();
	let ast = syn::parse_item(&source).expect("Failed to parse derive input");

	let args = parse_args(args);
	let endpoint_name = args.get(0).expect("Failed to parse an endpoint name argument");
	let intf = items::Interface::from_item(ast);

	match args.len() {
		1 => {
			let endpoint = generate_eth_endpoint(&endpoint_name, &intf);
			let generated = quote! {
				#intf
				#endpoint
			};
			generated.parse().expect("Failed to parse generated input")
		}
		2 => {
			let client_name = args.get(1).expect("Failed to parse an client name argument");
			let endpoint = generate_eth_endpoint(&endpoint_name, &intf);
			let client = generate_eth_client(client_name, &intf);
			let generated = quote! {
				#intf
				#endpoint
				#client
			};
			write_json_abi(&intf);
			generated.parse().expect("Failed to parse generated input")
		}
		len => {
			panic!("eth_abi marco takes one or two comma-separated arguments, passed {}", len);
		}
	}
}

fn write_json_abi(intf: &items::Interface) {
	use std::fs;
	use std::path::PathBuf;
	use std::env;

	let mut target = PathBuf::from(env::var("CARGO_TARGET_DIR").unwrap_or(".".to_owned()));
	target.push("target");
	target.push("json");
	fs::create_dir_all(&target).expect("failed to create json directory");
	target.push(&format!("{}.json", intf.name()));

	let mut f = fs::File::create(target).expect("failed to write json");
	let abi: json::Abi = intf.into();
	serde_json::to_writer_pretty(&mut f, &abi).expect("failed to write json");
}

fn generate_eth_client(client_name: &str, intf: &items::Interface) -> quote::Tokens {
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

						::pwasm_ethereum::call(self.gas.unwrap_or(::pwasm_ethereum::gas_limit().into()), &self.address, self.value.clone().unwrap_or(::bigint::U256::zero()), &payload, &mut result[..])
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

	let client_ident: syn::Ident = client_name.to_string().into();
	let name_ident: syn::Ident = intf.name().clone().into();

	quote! {
		pub struct #client_ident {
			gas: Option<u64>,
			address: ::parity_hash::Address,
			value: Option<::bigint::U256>,
		}

		impl #client_ident {
			pub fn new(address: ::parity_hash::Address) -> Self {
				#client_ident {
					gas: None,
					address: address,
					value: None,
				}
			}

			pub fn gas(mut self, gas: u64) -> Self {
				self.gas = Some(gas);
				self
			}

			pub fn value(mut self, val: ::bigint::U256) -> Self {
				self.value = Some(val);
				self
			}
		}

		impl #name_ident for #client_ident {
			#client_ctor
			#(#calls)*
		}
	}
}

fn generate_eth_endpoint(endpoint_name: &str, intf: &items::Interface) -> quote::Tokens {
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

	let endpoint_ident: syn::Ident = endpoint_name.to_string().into();
	let name_ident: syn::Ident = intf.name().clone().into();

	quote! {
		pub struct #endpoint_ident<T: #name_ident> {
			inner: T,
		}

		impl<T: #name_ident> From<T> for #endpoint_ident<T> {
			fn from(inner: T) -> #endpoint_ident<T> {
				#endpoint_ident {
					inner: inner,
				}
			}
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
					#(#branches,)*
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
