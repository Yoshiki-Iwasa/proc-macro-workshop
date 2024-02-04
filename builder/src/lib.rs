use original::OriginalMethodsFactory;
use proc_macro2::{Ident, Span};
use quote::quote;
use struct_builder::BuilderFactory;
use syn::{parse_macro_input, DeriveInput, GenericArgument, PathArguments, Type};
mod original;
mod struct_builder;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let mut builder_factory = BuilderFactory::new(input.clone());
    let original_method_factory = OriginalMethodsFactory::new(input.clone());

    let builder_and_methods = builder_factory.build(&input);
    let original_methods = original_method_factory.build();

    let expand = quote! {

        #original_methods

        #builder_and_methods

    };

    proc_macro::TokenStream::from(expand)
}

// literally Option< > only
fn is_option(ty: &Type) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    // get_identは : Stringとかの時しかつかえない
    let is_qself_none = type_path.qself.is_none();

    let is_leading_colon_none = type_path.path.leading_colon.is_none();

    let is_option = type_path.path.segments.first().is_some_and(|segment| {
        let is_option = segment.ident == "Option";
        if let PathArguments::AngleBracketed(args) = &segment.arguments {
            args.args.len() == 1 && is_option
        } else {
            false
        }
    });

    is_qself_none && is_leading_colon_none && is_option
}

fn is_vector(ty: &Type) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    // get_identは : Stringとかの時しかつかえない
    let is_qself_none = type_path.qself.is_none();

    let is_leading_colon_none = type_path.path.leading_colon.is_none();

    let is_vector = type_path.path.segments.first().is_some_and(|segment| {
        let is_vector = segment.ident == "Vec";
        if let PathArguments::AngleBracketed(args) = &segment.arguments {
            args.args.len() == 1 && is_vector
        } else {
            false
        }
    });

    is_qself_none && is_leading_colon_none && is_vector
}

// literally Option< > only
// if not it's option, return original type
fn extract_type_from_option(ty: &Type) -> Type {
    let syn::Type::Path(type_path) = ty else {
        panic!("expect Option<T>")
    };

    let segment = type_path.path.segments.first().expect("Option<T>");

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        panic!("expect Option<T>")
    };

    assert!(args.args.len() == 1, "expect Option<T>");

    let GenericArgument::Type(ty) = args.args.first().unwrap() else {
        panic!("expect Option<T>")
    };

    ty.clone()
}

fn extract_type_from_vector(ty: &Type) -> Type {
    let syn::Type::Path(type_path) = ty else {
        panic!("expect Vec<T>")
    };

    let segment = type_path.path.segments.first().expect("Vec<T>");

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        panic!("expect Vec<T>")
    };

    assert!(args.args.len() == 1, "expect Vec<T>");

    let GenericArgument::Type(ty) = args.args.first().unwrap() else {
        panic!("expect Vec<T>")
    };

    ty.clone()
}

fn builder_name(original_input: &DeriveInput) -> Ident {
    Ident::new(
        &format!("{}Builder", &original_input.ident),
        Span::call_site(),
    )
}
