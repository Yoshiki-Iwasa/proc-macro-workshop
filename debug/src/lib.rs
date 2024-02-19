use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::Parse, parse2, parse_macro_input, parse_quote, Attribute, Data, DeriveInput, Error,
    Expr, Field, GenericArgument, GenericParam, Generics, Lit, MacroDelimiter, MetaNameValue,
    PathArguments, WherePredicate,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn add_where_clause_from_struct_attr(
    mut generics: Generics,
    attrs: &[Attribute],
) -> Option<Generics> {
    let where_predicates = attrs
        .iter()
        .filter_map(|attr| attr.meta.require_list().ok())
        .filter_map(|meta_list| {
            if matches!(meta_list.delimiter, MacroDelimiter::Paren(_))
                && meta_list.path.is_ident("debug")
            {
                let token_stream = &meta_list.tokens;
                let name_value: Option<MetaNameValue> = parse2(token_stream.clone()).ok();
                name_value
            } else {
                None
            }
        })
        .filter_map(|name_value| {
            if name_value.path.is_ident("bound") {
                Some(name_value.value)
            } else {
                None
            }
        })
        .filter_map(|expr| {
            let Expr::Lit(lit_expr) = expr else {
                return None;
            };
            let Lit::Str(lit_str) = &lit_expr.lit else {
                return None;
            };

            lit_str.parse_with(WherePredicate::parse).ok()
        })
        .collect::<Vec<_>>();
    if where_predicates.is_empty() {
        None
    } else {
        let where_clause = generics.make_where_clause();
        where_predicates.iter().for_each(|where_predicate| {
            where_clause.predicates.push(parse_quote!(#where_predicate));
        });
        Some(generics)
    }
}

fn add_where_clause_from_fields(mut generics: Generics, fields: &[Field]) -> Generics {
    for field in fields {
        let syn::Type::Path(type_path) = &field.ty else {
            continue;
        };

        let PathArguments::AngleBracketed(angle_generics_args) =
            &type_path.path.segments.last().unwrap().arguments
        else {
            continue;
        };

        for generics_arg in &angle_generics_args.args {
            let GenericArgument::Type(ty) = generics_arg else {
                continue;
            };

            let syn::Type::Path(inner_type_path) = ty else {
                continue;
            };

            let inner_first_segment = inner_type_path.path.segments.first().unwrap();

            let first_ident = &inner_first_segment.ident;

            let type_params = generics.type_params_mut();

            let mut where_predicates: Vec<WherePredicate> = vec![];
            for type_param in type_params {
                let inner_type_used =
                    &type_param.ident == first_ident && inner_type_path.path.segments.len() > 1;

                if inner_type_used {
                    where_predicates.push(parse_quote!(#inner_type_path: std::fmt::Debug));
                    continue;
                }
            }
            let where_clause = generics.make_where_clause();
            where_clause.predicates.extend(where_predicates);
        }
    }

    generics
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
            if fields.iter().any(|field| {
                let syn::Type::Path(type_path) = &field.ty else {
                    return false;
                };
                let PathArguments::AngleBracketed(args) =
                    &type_path.path.segments.last().unwrap().arguments
                else {
                    return false;
                };
                args.args.iter().any(|arg| {
                    let GenericArgument::Type(ty) = arg else {
                        return false;
                    };

                    let syn::Type::Path(inner_type_path) = ty else {
                        return false;
                    };
                    inner_type_path.path.segments.len() > 1
                        && &inner_type_path.path.segments.first().unwrap().ident == type_ident
                })
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

    let generics = if let Some(generics) =
        add_where_clause_from_struct_attr(input.generics.clone(), &input.attrs)
    {
        generics
    } else {
        add_trait_bounds(input.generics.clone(), &fields)
    };
    let generics = add_where_clause_from_fields(generics, &fields);

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
