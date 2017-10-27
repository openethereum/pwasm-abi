use {quote, syn, utils};
use abi::eth::NamedSignature;

pub struct Interface {
	name: String,
	endpoint_name: String,
	client_name: String,
	items: Vec<Item>,
}

pub struct Event {
	pub name: syn::Ident,
	pub method_sig: syn::MethodSig,
	indexed: Vec<(syn::Pat, syn::Ty)>,
	data: Vec<(syn::Pat, syn::Ty)>,
	signature: NamedSignature,
}

pub enum Item {
	Signature(syn::Ident, syn::MethodSig),
	Event(Event),
	Other(syn::TraitItem),
}

impl Interface {
	pub fn from_item(source: syn::Item) -> Self {
		let trait_items = match source.node {
			syn::ItemKind::Trait(_, _, _, items) => items,
			_ => { panic!("Dispatch trait can work with trait declarations only!"); }
		};

		Interface {
			name: source.ident.as_ref().to_string(),
			endpoint_name: String::new(),
			client_name: String::new(),
			items: trait_items.into_iter().map(Item::from_trait_item).collect(),
		}
	}

	pub fn endpoint(mut self, endpoint_name: String) -> Self {
		self.endpoint_name = endpoint_name;
		self
	}

	pub fn client(mut self, client_name: String) -> Self {
		self.client_name = client_name;
		self
	}

	pub fn items(&self) -> &[Item] {
		&self.items
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn endpoint_name(&self) -> &str {
		&self.endpoint_name
	}

	pub fn client_name(&self) -> &str {
		&self.client_name
	}
}

impl Item {
	pub fn from_trait_item(source: syn::TraitItem) -> Self {
		let ident = source.ident;
		let node = source.node;
		let attrs = source.attrs;
		match node {
			syn::TraitItemKind::Method(method_sig, None) => {
				if attrs.iter().any(|a| match a.value {
					syn::MetaItem::Word(ref ident) => ident.as_ref() == "event" ,
					_ => false
				}) {
					let (indexed, non_indexed) = utils::iter_signature(&method_sig)
						.partition(|&(ref pat, _)| quote! { #pat }.to_string().starts_with("indexed_"));

					let named_signature =  NamedSignature::new(
						ident.to_string(),
						utils::parse_rust_signature(&method_sig)
					);
					let event = Event {
						name: ident,
						indexed: indexed,
						data: non_indexed,
						signature: named_signature,
						method_sig: method_sig,
					};

					Item::Event(event)
				} else {

					Item::Signature(ident, method_sig)
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
							let hash = event.signature.hash();

							let hash_bytes = hash.as_ref().iter().map(|b| {
								syn::Lit::Int(*b as u64, syn::IntTy::U8)
							});

							let indexed_pats = event.indexed.iter()
								.map(|&(ref pat, _)| pat);

							let data_pats = event.data.iter()
								.map(|&(ref pat, _)| pat);

							quote! {
								let topics = &[
									[#(#hash_bytes),*].into(),
									#(::pwasm_abi::eth::AsLog::as_log(&#indexed_pats)),*
								];
								let values: &[::pwasm_abi::eth::ValueType] = &[
									#(#data_pats.into()),*
								];
								let payload = ::pwasm_abi::eth::encode_values(values);
							}
						}
					)
				]);
			},
			Item::Signature(ref name, ref method_sig) => {
				tokens.append_all(&[syn::TraitItem {
					ident: name.clone(),
					attrs: Vec::new(),
					node: syn::TraitItemKind::Method(
						method_sig.clone(),
						None,
					),
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
		tokens.append(
			quote! (
				pub trait #trait_ident {
					#(#items)*
				}
			)
		);
	}
}