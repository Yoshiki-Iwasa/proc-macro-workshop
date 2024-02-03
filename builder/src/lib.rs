use original::OriginalMethodsFactory;
use proc_macro2::{Ident, Span};
use quote::quote;
use struct_builder::BuilderFactory;
use syn::{parse_macro_input, DeriveInput, GenericArgument, PathArguments, Type};
mod original;
mod struct_builder;

#[proc_macro_derive(Builder)]
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
fn extract_type_from_option(ty: &Type) -> Option<Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let None = type_path.qself else {
        return None;
    };
    let None = type_path.path.leading_colon else {
        return None;
    };

    let segment = (type_path.path.segments.len() == 1).then(|| &type_path.path.segments[0])?;

    if segment.ident != *"Option" {
        return None;
    };

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };

    let GenericArgument::Type(warped_ty) = args.args.first()? else {
        return None;
    };

    Some(warped_ty.clone())
}

fn builder_name(original_input: &DeriveInput) -> Ident {
    Ident::new(
        &format!("{}Builder", &original_input.ident),
        Span::call_site(),
    )
}
