use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    parse, parse_macro_input, parse_quote, Data, DeriveInput, Error, Expr, Field, GenericParam,
    Generics, LitStr,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(std::fmt::Debug));
        }
    }
    generics
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = input.ident.clone();

    let struct_name_lit_str = LitStr::new(struct_name.to_string().as_str(), Span::call_site());
    let generics = add_trait_bounds(input.generics.clone());

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = extract_fields(&input)?;
    let field_name_chain_methods = fields
        .iter()
        .map(|field| {
            let ident = field.ident.clone().map_or_else(
                || Err(syn_error("Field name is expected", field)),
                syn::Result::Ok,
            )?;
            let format = extract_debug_attr(field);
            let field_name_literal = LitStr::new(ident.to_string().as_str(), Span::call_site());

            let value = if let Some(fmt) = format {
                quote!(&format_args!(#fmt, &self.#ident))
            } else {
                quote!(&self.#ident)
            };
            Ok(quote! {
                .field(#field_name_literal, #value)
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote! {

        // impl std::fmt::Debug for #impl_generics {
        //     fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //         fmt.write_fmt(format_args!("Foo {}", self.0))
        //     }
        // }

        impl #impl_generics std::fmt::Debug for #struct_name #ty_generics #where_clause {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                fmt.debug_struct(#struct_name_lit_str)
                #(#field_name_chain_methods)*
                .finish()
            }
        }
    })
}

fn extract_debug_attr(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        let meta_name_value = attr.meta.require_name_value().unwrap();

        if !meta_name_value.path.is_ident("debug") {
            continue;
        }

        if let Expr::Lit(lit) = &meta_name_value.value {
            match &lit.lit {
                syn::Lit::Str(s) => return Some(s.value()),
                _ => continue,
            }
        } else {
            continue;
        }
    }
    None
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
