use {quote, syn, utils, proc_macro2};

pub struct Interface {
	name: String,
	constructor: Option<Signature>,
	items: Vec<Item>,
}

pub struct Event {
	pub name: syn::Ident,
	pub canonical: String,
	pub method_sig: syn::MethodSig,
	pub indexed: Vec<(syn::Pat, syn::Type)>,
	pub data: Vec<(syn::Pat, syn::Type)>,
}

#[derive(Clone)]
pub struct Signature {
	pub name: syn::Ident,
	pub canonical: String,
	pub method_sig: syn::MethodSig,
	pub hash: u32,
	pub arguments: Vec<(syn::Pat, syn::Type)>,
	pub return_type: Option<syn::Type>,
	pub is_constant: bool,
	pub is_payable: bool,
}

pub enum Item {
	Signature(Signature),
	Event(Event),
	Other(syn::ItemTrait),
}

impl Item {
	fn name(&self) -> Option<&syn::Ident> {
		use Item::*;
		match *self {
			Signature(ref sig) => Some(&sig.name),
			Event(ref event) => Some(&event.name),
			Other(_) => None,
		}
	}
}

impl Interface {
	pub fn from_item(source: syn::ItemTrait) -> Self {

		let (constructor_items, other_items) = source.items
			.into_iter()
			.map(Item::from_trait_item)
			.partition::<Vec<Item>, _>(|item| item.name().map_or(false, |ident| ident.as_ref() == "constructor"));

		Interface {
			constructor: constructor_items
				.into_iter()
				.next()
				.map(|item| match item { Item::Signature(sig) => sig, _ => panic!("constructor must be function!") }),
			name: source.ident.as_ref().to_string(),
			items: other_items,
		}
	}

	pub fn items(&self) -> &[Item] {
		&self.items
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn constructor(&self) -> Option<&Signature> {
		self.constructor.as_ref()
	}
}

fn into_signature(ident: syn::Ident, method_sig: syn::MethodSig, is_constant: bool, is_payable: bool) -> Signature {
	let arguments: Vec<(syn::Pat, syn::Type)> = utils::iter_signature(&method_sig).collect();
	let canonical = utils::canonical(&ident, &method_sig);
	let return_type: Option<syn::Type> = match method_sig.decl.output {
		syn::ReturnType::Default => None,
		syn::ReturnType::Type(ref ty) => Some(ty.clone()),
	};
	let hash = utils::hash(&canonical);

	Signature {
		name: ident,
		arguments: arguments,
		method_sig: method_sig,
		canonical: canonical,
		hash: hash,
		return_type: return_type,
		is_constant: is_constant,
		is_payable: is_payable,
	}
}

fn has_attribute(attrs: &[syn::Attribute], name: &str) -> bool {
	attrs.iter().any(|a| match a.value {
		syn::Meta::Word(ref ident) => ident.as_ref() == name,
		_ => false
	})
}

impl Item {
	pub fn from_trait_item(source: syn::ItemTrait) -> Self {
		let ident = source.ident;
		let node = source.node;
		let attrs = source.attrs;
		match source {
			syn::TraitItem::Method(method) => {
				if has_attribute(&method.attrs, "event") {
					let (indexed, non_indexed) = utils::iter_signature(&method.sig)
						.partition(|&(ref pat, _)| quote! { #pat }.to_string().starts_with("indexed_"));
					let canonical = utils::canonical(&method.sig.ident, &method.sig);

					let event = Event {
						name: method.sig.ident.clone(),
						canonical: canonical,
						indexed: indexed,
						data: non_indexed,
						method_sig: method.sig,
					};

					Item::Event(event)
				} else {
					Item::Signature(
						into_signature(method.sig.ident.clone(),
							method.sig,
							has_attribute(&attrs, "constant"),
							has_attribute(&attrs, "payable")
						)
					)
				}
			},
			_ => {
				Item::Other(syn::TraitItem { attrs: attrs, node: node, ident: ident })
			}
		}
	}
}

impl quote::ToTokens for Item {
	fn to_tokens(&self, tokens: &mut quote::Tokens) {
		match *self {
			Item::Event(ref event) => {
				let method_sig = &event.method_sig;
				let name = &event.name;
				tokens.append_all(&[
					utils::produce_signature(
						name,
						method_sig,
						{
							let keccak = utils::keccak(&event.canonical);
							let hash_bytes = keccak.as_ref().iter().map(|b| {
								syn::Lit::Int(syn::LitInt::new(*b as u64, syn::IntSuffix::U8, proc_macro2::call_site()))
							});

							let indexed_pats = event.indexed.iter()
								.map(|&(ref pat, _)| pat);

							let data_pats = event.data.iter()
								.map(|&(ref pat, _)| pat);

							let data_pats_count_lit = syn::Lit::Int(syn::LitInt::new(event.data.len() as u64, syn::IntSuffix::Usize, proc_macro2::call_site()));

							quote! {
								let topics = &[
									[#(#hash_bytes),*].into(),
									#(::pwasm_abi::eth::AsLog::as_log(&#indexed_pats)),*
								];

								let mut sink = ::pwasm_abi::eth::Sink::new(#data_pats_count_lit);
								#(sink.push(#data_pats));*;
								let payload = sink.finalize_panicking();

								::pwasm_ethereum::log(topics, &payload);
							}
						}
					)
				]);
			},
			Item::Signature(ref signature) => {
				tokens.append_all(&[syn::TraitItemMethod {
					sig: syn::MethodSig {

					}
					attrs: Vec::new(),
					node: syn::TraitItem::Method(syn::TraitItemMethod{
						attrs: vec![],
						sig: signature.method_sig.clone(),
						default: None,
						semi_token: None,
					}),
				}]);
			},
			Item::Other(ref item) => {
				tokens.append_all(&[item]);
			}
		}
	}
}

impl quote::ToTokens for Interface {
	fn to_tokens(&self, tokens: &mut quote::Tokens) {
		let trait_ident: syn::Ident = self.name.clone().into();

		let items = &self.items;
		let constructor_item = self.constructor().map(|c| Item::Signature(c.clone()));
		tokens.append(
			quote! (
				pub trait #trait_ident {
					#constructor_item
					#(#items)*
				}
			)
		);
	}
}
