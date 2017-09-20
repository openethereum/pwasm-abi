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

fn ty_to_param_type(ty: &syn::Ty) -> Vec<abi::legacy::ParamType> {
	let mut result = Vec::new();
	match *ty {
		syn::Ty::Path(None, ref path) => {
			match path.segments.last().unwrap().ident.to_string().as_ref() {
				"u32" => result.push(abi::legacy::ParamType::U32),
				"bool" => result.push(abi::legacy::ParamType::Bool),
				ref val @ _ => panic!("Unable to handle param of type {}: not supported by abi", val)
			}
		},
		syn::Ty::Tup(ref tys) => {
			for ty in tys {
				result.extend(ty_to_param_type(ty))
			}
		},
		ref val @ _ => panic!("Unable to handle param of type {:?}: not supported by abi", val),
	}
	result
}

fn parse_rust_signature(method_sig: &syn::MethodSig) -> abi::legacy::Signature {
	let mut params = Vec::new();
	
	for fn_arg in method_sig.decl.inputs.iter() {
		match *fn_arg {
			syn::FnArg::Captured(_, ref ty) => {
				params.extend(ty_to_param_type(ty));
			},
			syn::FnArg::SelfValue(_) => { panic!("cannot use self by value"); },
			_ => {},
		}
	}

	abi::legacy::Signature::new_void(params)
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

	let trait_items = match item.node {
		syn::ItemKind::Trait(_, _, _, ref items) => items,
		_ => { panic!("Dispatch trait can work with trait declarations only!"); }
	};

	let signatures: Vec<abi::legacy::NamedSignature> = 
		trait_items.iter().filter_map(trait_item_to_signature).collect();

	let hashed_signatures: Vec<abi::legacy::HashSignature> = 
		signatures.clone().into_iter()
			.map(From::from)
			.collect();

	let table_signatures = hashed_signatures.clone().into_iter().map(|hs| {
		let hash_literal = syn::Lit::Int(hs.hash() as u64, syn::IntTy::U32);
		quote! {
			::pwasm_abi::legacy::HashSignature::new(
				#hash_literal, 
				::pwasm_abi::legacy::Signature::new_void(Vec::new())
			)
		}
	});

	let branches = signatures.into_iter().map(|signature| {
		quote! {
			_ => { self.inner.baz() }
		}
	});

	let dispatch_table = quote! {
		{
			let mut table = ::pwasm_abi::legacy::Table::new(
				[
					#(#table_signatures)*
				].to_vec()
			);
			table
		}
	};

	quote! {
		#item

		struct Endpoint<T: #name> {
			inner: T,
			table: ::pwasm_abi::legacy::Table,
		}

		impl<T: #name> Endpoint<T> {
			pub fn new(inner: T) -> Self {
				Endpoint { 
					inner: inner,
					table: #dispatch_table,
				}
			}

			pub fn dispatch(&mut self, payload: Vec<u8>) -> Vec<u8> {
				// match &payload[0] {
				// 	#(#branches)*
				// };

				Vec::new()
			}
		}
	}
}