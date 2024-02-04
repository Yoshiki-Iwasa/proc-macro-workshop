use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Error, Field, LitStr};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = input.ident.clone();

    let struct_name_lit_str = LitStr::new(struct_name.to_string().as_str(), Span::call_site());

    let fields = extract_fields(&input)?;
    let field_name_chain_methods = fields
        .iter()
        .map(|field| {
            let ident = field.ident.clone().map_or_else(
                || Err(syn_error("Field name is expected", field)),
                syn::Result::Ok,
            )?;
            let field_name_literal = LitStr::new(ident.to_string().as_str(), Span::call_site());
            Ok(quote! {
                .field(#field_name_literal, &self.#ident)
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote! {
        impl std::fmt::Debug for #struct_name {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                fmt.debug_struct(#struct_name_lit_str)
                #(#field_name_chain_methods)*
                .finish()
            }
        }
    })
}

fn extract_fields(input: &DeriveInput) -> syn::Result<Vec<Field>> {
    let Data::Struct(data_struct) = &input.data else {
        return Err(syn_error("expect struct", input.clone()));
    };
    let syn::Fields::Named(fields_named) = &data_struct.fields else {
        return Err(syn_error("expect FieldsNamed", data_struct.fields.clone()));
    };

    Ok(fields_named.named.iter().cloned().collect::<Vec<_>>())
}

fn syn_error(message: &str, token: impl ToTokens) -> syn::Error {
    Error::new_spanned(token, message)
}
