use {syn, quote};

pub fn produce_signature<T: quote::ToTokens>(
	ident: &syn::Ident,
	method_sig: &syn::MethodSig,
	t: T,
) -> quote::Tokens
{
	let args = method_sig.decl.inputs.iter().filter_map(|arg| {
		match *arg {
			syn::FnArg::Captured(ref pat, ref ty) => Some(quote!{#pat: #ty}),
			_ => None,
		}
	});
	match method_sig.decl.output {
		syn::FunctionRetTy::Ty(ref output) => {
			quote!{
				fn #ident(&mut self, #(#args),*) -> #output {
					#t
				}
			}
		},
		syn::FunctionRetTy::Default => {
			quote!{
				fn #ident(&mut self, #(#args),*) {
					#t
				}
			}
		}
	}
}