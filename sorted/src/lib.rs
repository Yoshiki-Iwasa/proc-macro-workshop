use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Error, Item};

#[proc_macro_attribute]
pub fn sorted(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = args;
    let input = parse_macro_input!(input as Item);
    check_sorted(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn check_sorted(input: Item) -> syn::Result<proc_macro2::TokenStream> {
    let Item::Enum(_) = &input else {
        return Err(Error::new(
            Span::call_site(),
            "expected enum or match expression",
        ));
    };

    let token_stream = quote! {
        #input
    };

    Ok(token_stream)
}
