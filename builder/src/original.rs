use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::builder_name;

pub struct OriginalMethodsFactory {
    base: DeriveInput,
}

impl OriginalMethodsFactory {
    pub fn new(input: DeriveInput) -> Self {
        Self { base: input }
    }

    pub fn impl_methods(&self, methods: Vec<TokenStream>) -> TokenStream {
        let ident = self.base.ident.clone();
        quote! {
          impl #ident {
            #(#methods)*
          }
        }
    }

    pub fn build(&self) -> TokenStream {
        let builder_method = self.builder_method();

        let methods = vec![builder_method];

        self.impl_methods(methods)
    }

    fn builder_method(&self) -> TokenStream {
        let builder_name = builder_name(&self.base);

        quote! {
          pub fn builder() -> #builder_name {
            #builder_name::default()
          }
        }
    }
}
