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

fn parse_rust_signature(method_sig: &syn::MethodSig) -> abi::legacy::Signature {
	let mut params: Vec<abi::legacy::ParamType> = Vec::new();

	for fn_arg in method_sig.decl.inputs.iter() {
		println!("{:?}", fn_arg);
	}

	abi::legacy::Signature::new_void(Vec::new())
}

fn trait_item_to_signature(item: &syn::TraitItem) -> Option<abi::legacy::NamedSignature> {
	let name = item.ident.as_ref().to_string();
	match item.node {
		syn::TraitItemKind::Method(ref method_sig, None) => {
			Some(
				abi::legacy::NamedSignature::new(
					name,
					parse_rust_signature(method_sig),
				)
			)
		},
		_ => {
			None
		}
	}
}

fn impl_legacy_dispatch(item: &syn::Item) -> quote::Tokens {
	let name = &item.ident;

	println!("here");

	let trait_items = match item.node {
		syn::ItemKind::Trait(_, _, _, ref items) => items,
		_ => { panic!("Dispatch trait can work with trait declarations only!"); }
	};

	let signatures = trait_items.iter().filter_map(trait_item_to_signature);

	quote! {
		#item

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