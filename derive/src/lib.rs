#![feature(alloc)]
#![feature(proc_macro)]
#![recursion_limit="128"]

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

fn param_type_to_ident(param_type: &abi::legacy::ParamType) -> syn::Path {
	use abi::legacy::ParamType;
	match *param_type {
		ParamType::U32 => syn::parse_path("::pwasm_abi::legacy::ParamType::U32").expect("failed to parse paramtype u32"),
		ParamType::Bool => syn::parse_path("::pwasm_abi::legacy::ParamType::Bool").expect("failed to parse paramtype bool"),
		_ => panic!("unsupported signature param type"),
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

			if let Some(return_type) = hs.signature().result() {
				quote! {
					#hash_literal => {
						Some(
							inner.#ident(
								args.next().expect("Failed to fetch next argument").into(), 
								args.next().expect("Failed to fetch next argument").into(), 
							).into()
						)
					}
				}
			} else {
				quote! {
					#hash_literal => { 
						inner.#ident(
							args.next().expect("Failed to fetch next argument").into(), 
							args.next().expect("Failed to fetch next argument").into(), 
						); 
						None 
					}
				}
			}
		}
	);

	let dispatch_table = quote! {
		{
			let mut table = ::pwasm_abi::legacy::Table::new(
				[
					#(#table_signatures),*
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
				let inner = &mut self.inner;
				self.table.dispatch(payload, |method_id, args| {
					let mut args = args.into_iter();
					match method_id {
				 		#(#branches),*,
						_ => panic!("Invalid method signature"),
					}
				}).expect("Failed abi dispatch")
			}
		}
	}
}