use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;

    let input: Item = parse_macro_input!(input);

    let token_steram = quote! {
        #input
    };

    token_steram.into()
}
