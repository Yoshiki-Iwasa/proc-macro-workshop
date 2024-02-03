use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, token::Pub, Attribute, Data, DeriveInput, Type, Visibility,
};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();

    let builder_struct = builder_struct(input.clone());

    let field_name_type = field_name_type_set(&input);

    let builder_name = builder_struct.ident.clone();
    let accessor = accessor(builder_name.clone(), field_name_type);

    let builer_fn = build_fn(builder_name.clone(), name.clone(), field_names(&input));

    let expand = quote! {
        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name::default()
            }
        }

        #builder_struct


        #accessor

        #builer_fn
    };

    proc_macro::TokenStream::from(expand)
}

fn accessor(builder_name: Ident, original_field_name_type: Vec<(Ident, Type)>) -> TokenStream {
    let tokens = original_field_name_type
        .into_iter()
        .map(|(ident, ty)| {
            quote! {
                pub fn #ident(&mut self, #ident: #ty) {
                    self.#ident = Some(#ident)
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl #builder_name {
            #(#tokens)*
        }
    }
}

fn build_fn(
    builder_name: Ident,
    target_struct_name: Ident,
    field_names: Vec<Ident>,
) -> TokenStream {
    let tokens = field_names
        .iter()
        .map(|field_name| {
            quote! {
                let #field_name = self.#field_name.map_or_else(|| {
                    Err(format!("{} is empty", stringify!(#field_name)))
                }, Ok)?;
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl #builder_name {
            fn build(self) -> Result<#target_struct_name, Box<dyn std::error::Error>> {
                #(#tokens)*

                Ok(#target_struct_name {
                    #(#field_names),*
                })

            }
        }
    }
}

fn field_name_type_set(input: &DeriveInput) -> Vec<(Ident, Type)> {
    match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(field_named) => field_named
                .named
                .iter()
                .map(|ddd| (ddd.ident.clone().unwrap(), ddd.ty.clone()))
                .collect::<Vec<_>>(),
            syn::Fields::Unnamed(_) => panic!("field name is necessary"),
            syn::Fields::Unit => panic!("field name is necessary"),
        },
        Data::Enum(_) => panic!("Should be struct"),
        Data::Union(_) => panic!("Should be struct"),
    }
}

fn field_names(input: &DeriveInput) -> Vec<Ident> {
    match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(field_named) => field_named
                .named
                .iter()
                .map(|ddd| ddd.ident.clone().unwrap())
                .collect::<Vec<_>>(),
            syn::Fields::Unnamed(_) => panic!("field name is necessary"),
            syn::Fields::Unit => panic!("field name is necessary"),
        },
        Data::Enum(_) => panic!("Should be struct"),
        Data::Union(_) => panic!("Should be struct"),
    }
}

fn builder_struct(mut input: DeriveInput) -> DeriveInput {
    let name = input.ident.clone();
    let builder_name = Ident::new(&format!("{}Builder", name.clone()), Span::call_site());

    input.ident = builder_name;

    match &mut input.data {
        Data::Struct(data_struct) => match &mut data_struct.fields {
            syn::Fields::Named(field_named) => &mut field_named.named.iter_mut().for_each(|ddd| {
                let ty = ddd.ty.clone();
                ddd.ty = parse_quote! {
                    Option<#ty>
                };
                ddd.vis = Visibility::Public(Pub::default());
            }),
            syn::Fields::Unnamed(_) => panic!("field name is necessary"),
            syn::Fields::Unit => panic!("field name is necessary"),
        },
        Data::Enum(_) => panic!("Should be struct"),
        Data::Union(_) => panic!("Should be struct"),
    };

    let derive_attribute: Attribute = parse_quote! {
        #[derive(Default, Debug)]
    };

    input.attrs = vec![derive_attribute];

    input
}
