use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let expand = quote! {
        impl #name {
            pub fn builder() {}
        }
    };

    proc_macro::TokenStream::from(expand)
}
