use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, parse_quote, token::Pub, Attribute, Data, DeriveInput, Visibility};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let builder_struct = builder_struct(input.clone());

    let name = input.ident;

    let builder_name = Ident::new(&format!("{}Builder", name.clone()), Span::call_site());

    // let builder_struct_def = builder_struct(&builder_name, &data);

    //

    let expand = quote! {

        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name::default()
            }
        }

        #builder_struct
    };

    proc_macro::TokenStream::from(expand)
}

fn builder_struct(mut input: DeriveInput) -> TokenStream {
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

    quote! {
        #input
    }
}
