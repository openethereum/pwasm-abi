#![feature(alloc)]
#![feature(proc_macro)]

extern crate proc_macro;
extern crate pwasm_abi as abi;
extern crate syn;
#[macro_use]
extern crate quote;

#[cfg(not(feature="std"))]
#[macro_use]
extern crate alloc;

use alloc::vec::Vec;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn legacy_dispatch(args: TokenStream, input: TokenStream) -> TokenStream {	
	let source = input.to_string();   
	let ast = syn::parse_item(&source).expect("Failed to parse derive input");
	let generated = impl_legacy_dispatch(&ast);
	generated.parse().expect("Failed to parse generated input")
}

fn impl_legacy_dispatch(ast: &syn::Item) -> quote::Tokens {
	let name = &ast.ident;
	quote! {
		#ast

		struct Endpoint<T: #name> {
			inner: T,
		}

		impl<T: #name> Endpoint<T> {
			pub fn new(inner: T) -> Self {
				Endpoint { inner: inner }
			}

			pub fn dispatch(&mut self, payload: Vec<u8>) -> Vec<u8> {
				Vec::new()
			}
		}
	}
}