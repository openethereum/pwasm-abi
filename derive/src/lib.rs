extern crate proc_macro;
extern crate pwasm_abi as abi;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

#[proc_macro_derive(LegacyDispatch, attributes(dispatch))]
pub fn derive_legacy_dispatch(input: TokenStream) -> TokenStream {
	input
}