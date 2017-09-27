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

use alloc::vec::Vec;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn legacy_dispatch(args: TokenStream, input: TokenStream) -> TokenStream {
	let args_str = args.to_string();
	let endpoint_name = args_str.trim_matches(&['(', ')', '"', ' '][..]);

	let source = input.to_string();
	let ast = syn::parse_item(&source).expect("Failed to parse derive input");

	let generated = impl_legacy_dispatch(&ast, &endpoint_name);

	generated.parse().expect("Failed to parse generated input")
}

fn ty_to_param_type(ty: &syn::Ty) -> abi::legacy::ParamType {
	match *ty {
		syn::Ty::Path(None, ref path) => {
			let last_path = path.segments.last().unwrap();
			match last_path.ident.to_string().as_ref() {
				"u32" => abi::legacy::ParamType::U32,
				"i32" => abi::legacy::ParamType::I32,
				"u64" => abi::legacy::ParamType::U64,
				"i64" => abi::legacy::ParamType::I64,
				"U256" => abi::legacy::ParamType::U256,
				"H256" => abi::legacy::ParamType::H256,
				"Vec" => {
					match last_path.parameters {
						syn::PathParameters::AngleBracketed(ref param_data) => {
							let vec_arg = param_data.types.last().unwrap();
							if let syn::Ty::Path(None, ref nested_path) = *vec_arg {
								if "u8" == nested_path.segments.last().unwrap().ident.to_string() {
									return abi::legacy::ParamType::Bytes;
								}
							}
							abi::legacy::ParamType::Array(Box::new(ty_to_param_type(vec_arg)))
						},
						_ => panic!("Unsupported vec arguments"),
					}
				},
				"String" => abi::legacy::ParamType::String,
				"bool" => abi::legacy::ParamType::Bool,
				ref val @ _ => panic!("Unable to handle param of type {}: not supported by abi", val)
			}
		},
		ref val @ _ => panic!("Unable to handle param of type {:?}: not supported by abi", val),
	}
}

fn parse_rust_signature(method_sig: &syn::MethodSig) -> abi::legacy::Signature {
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

fn param_type_to_ident(param_type: &abi::legacy::ParamType) -> quote::Tokens {
	use abi::legacy::ParamType;
	match *param_type {
		ParamType::U32 => quote! { ::pwasm_abi::legacy::ParamType::U32 },
		ParamType::I32 => quote! { ::pwasm_abi::legacy::ParamType::U32 },
		ParamType::U64 => quote! { ::pwasm_abi::legacy::ParamType::U32 },
		ParamType::I64 => quote! { ::pwasm_abi::legacy::ParamType::U32 },
		ParamType::Bool => quote! { ::pwasm_abi::legacy::ParamType::Bool },
		ParamType::U256 => quote! { ::pwasm_abi::legacy::ParamType::U256 },
		ParamType::H256 => quote! { ::pwasm_abi::legacy::ParamType::H256 },
		ParamType::Address => quote! { ::pwasm_abi::legacy::ParamType::Address },
		ParamType::Bytes => quote! { ::pwasm_abi::legacy::ParamType::Bytes },
		ParamType::Array(ref t) => {
			let nested = param_type_to_ident(t);
			quote! {
				::pwasm_abi::legacy::ParamType::Array(Box::new(#nested))
			}
		},
		ParamType::String => quote! { ::pwasm_abi::legacy::ParamType::String },
	}
}

fn impl_legacy_dispatch(item: &syn::Item, endpoint_name: &str) -> quote::Tokens {
	let name = &item.ident;

	let trait_items = match item.node {
		syn::ItemKind::Trait(_, _, _, ref items) => items,
		_ => { panic!("Dispatch trait can work with trait declarations only!"); }
	};

	let signatures: Vec<abi::legacy::NamedSignature> =
		trait_items.iter().filter_map(trait_item_to_signature).collect();

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

	let hashed_signatures: Vec<abi::legacy::HashSignature> =
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
				::pwasm_abi::legacy::HashSignature::new(
					#hash_literal,
					::pwasm_abi::legacy::Signature::new(
						[#(#param_types),*].to_vec(),
						Some(#return_type),
					)
				)
			}
		} else {
			quote! {
				::pwasm_abi::legacy::HashSignature::new(
					#hash_literal,
					::pwasm_abi::legacy::Signature::new_void(
						[#(#param_types),*].to_vec()
					)
				)
			}
		}
	});

	let branches = hashed_signatures.into_iter()
		.zip(signatures.into_iter())
		.map(|(hs, ns)| {
			let hash_literal = syn::Lit::Int(hs.hash() as u64, syn::IntTy::U32);
			let ident: syn::Ident = ns.name().into();

			let args_line = std::iter::repeat(
				quote! { args.next().expect("Failed to fetch next argument").into() }
			).take(hs.signature().params().len());

			if let Some(_) = hs.signature().result() {
				quote! {
					#hash_literal => {
						Some(
							inner.#ident(
								#(#args_line),*
							).into()
						)
					}
				}
			} else {
				quote! {
					#hash_literal => {
						inner.#ident(
							#(#args_line),*
						);
						None
					}
				}
			}
		}
	);

	let dispatch_table = quote! {
		{
			let table = ::pwasm_abi::legacy::Table::new(
				[
					#(#table_signatures),*
				].to_vec()
			);
			table
		}
	};

	let endpoint_ident: syn::Ident = endpoint_name.into();

	quote! {
		#item

		pub struct #endpoint_ident<T: #name> {
			inner: T,
			table: ::pwasm_abi::legacy::Table,
		}

		impl<T: #name> #endpoint_ident<T> {
			pub fn new(inner: T) -> Self {
				Endpoint {
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
					#ctor_branch
				}).expect("Failed fallback abi dispatch");
			}
		}
	}
}