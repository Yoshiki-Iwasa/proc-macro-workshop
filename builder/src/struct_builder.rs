use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, token::Pub, Data, DeriveInput, Field, Visibility};

use crate::{builder_name, extract_type_from_option};

pub struct BuilderFactory {
    base: DeriveInput,
}

impl BuilderFactory {
    pub fn new(original: DeriveInput) -> Self {
        Self { base: original }
    }

    fn set_builder_name(&mut self, original_input: &DeriveInput) {
        self.base.ident = builder_name(original_input);
    }

    fn convert_fields_into_builder(&mut self) {
        let Data::Struct(data_struct) = &mut self.base.data else {
            panic!("Should be Struct")
        };
        let syn::Fields::Named(fields_named) = &mut data_struct.fields else {
            panic!("field name is necessary")
        };

        fields_named.named.iter_mut().for_each(|field| {
            let original_type = field.ty.clone();
            if extract_type_from_option(&original_type).is_none() {
                field.ty = parse_quote! {
                    Option<#original_type>
                };
            }
            field.vis = Visibility::Public(Pub::default());
        })
    }

    pub fn build(&mut self, original_input: &DeriveInput) -> TokenStream {
        self.set_builder_name(original_input);
        self.convert_fields_into_builder();
        self.set_derive_attributes();

        let accessor = Self::accessor(original_input);
        let build_fn = self.build_fn(original_input);

        let base = self.base.clone();

        quote! {
          #base

          #accessor

          #build_fn
        }
    }

    fn accessor(original_input: &DeriveInput) -> TokenStream {
        let fields = Self::extract_original_fields(original_input);
        let methods = fields
            .iter()
            .map(|field| {
                let ident = &field.ident.clone().unwrap();
                let ty = &field.ty;
                match extract_type_from_option(&field.ty) {
                    Some(ori_type) => {
                        quote! {
                          pub fn #ident(&mut self, #ident: #ori_type) -> &mut Self {
                              self.#ident = Some(#ident);
                              self
                          }
                        }
                    }
                    None => {
                        quote! {
                          pub fn #ident(&mut self, #ident: #ty) -> &mut Self {
                              self.#ident = Some(#ident);
                              self
                          }
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        let builder_name = builder_name(original_input);
        quote! {
            impl #builder_name {
                #(#methods)*
            }
        }
    }

    fn set_derive_attributes(&mut self) {
        let attr = parse_quote! {
            #[derive(Default, Debug, Clone)]
        };
        self.base.attrs = vec![attr]
    }

    fn extract_original_fields(original_input: &DeriveInput) -> Vec<Field> {
        let Data::Struct(data_struct) = &original_input.data else {
            panic!("Should be Struct")
        };
        let syn::Fields::Named(fields_named) = &data_struct.fields else {
            panic!("field name is necessary")
        };

        fields_named.named.iter().cloned().collect::<Vec<_>>()
    }

    fn extract_fields(&self) -> Vec<Field> {
        let Data::Struct(data_struct) = &self.base.data else {
            panic!("Should be Struct")
        };
        let syn::Fields::Named(fields_named) = &data_struct.fields else {
            panic!("field name is necessary")
        };

        fields_named.named.iter().cloned().collect::<Vec<_>>()
    }

    fn build_fn(&self, original_input: &DeriveInput) -> TokenStream {
        // originalとタイプが同じものは弾くか？
        let fields = self.extract_fields();
        let field_names = fields
            .into_iter()
            .map(|field| field.ident.unwrap())
            .collect::<Vec<_>>();

        let original_fields = Self::extract_original_fields(original_input);

        // ここで、もともとoptionだったものは無視していい
        let field_checks = original_fields
            .iter()
            .map(|original_field| {
                let field_name = original_field.ident.clone().unwrap();
                match extract_type_from_option(&original_field.ty) {
                    Some(_) => {
                        quote! {
                            let #field_name = self.#field_name.clone();
                        }
                    }
                    None => {
                        quote! {
                            let #field_name = self.#field_name.clone().map_or_else(|| {
                                Err(format!("{} is not set", stringify!(#field_name)))
                            }, Ok)?;
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        let builder_name = builder_name(original_input);
        let original_name = original_input.ident.clone();
        quote! {
            impl #builder_name {
                pub fn build(&mut self) -> Result<#original_name, Box<dyn std::error::Error>> {
                        #(#field_checks)*

                    Ok(#original_name {
                        #(#field_names),*
                    })
                }
            }
        }
    }
}
