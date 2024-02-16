use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse2, parse_macro_input, parse_quote, Data, DeriveInput, Error, Expr, Field, GenericParam,
    Generics,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn add_trait_bounds(mut generics: Generics, fields: &[Field]) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            let type_ident = &type_param.ident;
            if fields.iter().any(|field| {
                if let Ok(phantom) = parse2::<syn::Type>(quote!(PhantomData<#type_ident>)) {
                    field.ty == phantom
                } else {
                    false
                }
            }) {
                continue;
            }
            type_param.bounds.push(parse_quote!(std::fmt::Debug));
        }
    }
    generics
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = extract_fields(&input)?;

    let field_name_chain_methods = fields
        .iter()
        .map(|field| {
            let field_ident = field.ident.clone().map_or_else(
                || Err(syn_error("Field name is expected", field)),
                syn::Result::Ok,
            )?;
            let field_name = field_ident.clone().to_string();
            let format = extract_debug_attr(field);

            let value = if let Some(fmt) = format {
                quote!(&format_args!(#fmt, &self.#field_ident))
            } else {
                quote!(&self.#field_ident)
            };

            Ok(quote! {
                .field(#field_name, #value)
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let struct_name_ident = input.ident.clone();
    let struct_name_str = input.ident.clone().to_string();

    //
    let generics = add_trait_bounds(input.generics.clone(), &fields);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics std::fmt::Debug for #struct_name_ident #ty_generics #where_clause {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                fmt.debug_struct(#struct_name_str)
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
