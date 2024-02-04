use proc_macro2::Ident;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, token::Pub, Attribute, Data, DeriveInput, Expr, Field, Lit, Visibility};

use crate::{
    builder_name, extract_type_from_option, extract_type_from_vector, is_option, is_vector,
};

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
            if !is_option(&original_type)
                && !(is_vector(&original_type)
                    && field.attrs.iter().any(Self::is_attribute_builder))
            {
                field.ty = parse_quote! {
                    Option<#original_type>
                };
            }
            field.vis = Visibility::Public(Pub::default());
            field.attrs = vec![]
        })
    }

    pub fn build(&mut self, original_input: &DeriveInput) -> TokenStream {
        self.set_builder_name(original_input);
        self.convert_fields_into_builder();
        self.set_derive_attributes();

        let accessor = Self::accessor(original_input);
        let build_fn = self.build_fn(original_input);

        // builderを作るときに
        let base = self.base.clone();

        quote! {
          #base

          #accessor

          #build_fn
        }
    }

    // #[builder(each = "arg")]みたいな形式をdetectする
    fn extract_arg_name(attr: &Attribute) -> Ident {
        let builder: Expr = attr.parse_args().expect("#[builder(each = \"arg\")]");
        let Expr::Assign(assign) = builder else {
            panic!("#[builder(each = \"arg\")]")
        };

        assert!(
            assign.attrs.is_empty(),
            "attribute on expression is not allowed"
        );

        let Expr::Path(expr_path) = *assign.left else {
            panic!("")
        };
        assert!(
            expr_path.attrs.is_empty() && expr_path.qself.is_none(),
            "#[builder(each = \"arg\")]"
        );
        assert!(
            expr_path
                .path
                .get_ident()
                .is_some_and(|ident| ident == "each"),
            "#[builder(each = \"arg\")]"
        );

        let Expr::Lit(lit) = *assign.right else {
            panic!("#[builder(each = \"arg\")]")
        };
        let Lit::Str(lit_str) = lit.lit else {
            panic!("#[builder(each = \"arg\")]")
        };
        Ident::new(lit_str.value().as_str(), Span::call_site())
    }

    fn is_attribute_builder(attr: &Attribute) -> bool {
        attr.path().is_ident("builder")
    }

    fn accessor(original_input: &DeriveInput) -> TokenStream {
        let fields = Self::extract_original_fields(original_input);
        let methods = fields
            .iter()
            .map(|field| {
                let ident = &field.ident.clone().unwrap();
                let ty = &field.ty;
                if is_option(ty) {
                    let wraped_type = extract_type_from_option(ty);
                    quote! {
                      pub fn #ident(&mut self, #ident: #wraped_type) -> &mut Self {
                          self.#ident = Some(#ident);
                          self
                      }
                    }
                } else if let Some(attr) = field
                    .attrs
                    .clone()
                    .into_iter()
                    .find(Self::is_attribute_builder)
                {
                    if !is_vector(&field.ty) {
                        panic!("Vec<T> is expected")
                    }
                    let arg_name = Self::extract_arg_name(&attr);
                    let vec_inner_ty = extract_type_from_vector(&field.ty);
                    quote! {
                      pub fn #arg_name(&mut self, #arg_name: #vec_inner_ty) -> &mut Self {
                        self.#ident.push(#arg_name);
                        self
                      }
                    }
                } else {
                    quote! {
                      pub fn #ident(&mut self, #ident: #ty) -> &mut Self {
                          self.#ident = Some(#ident);
                          self
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
                match is_option(&original_field.ty)
                    || (is_vector(&original_field.ty)
                        && original_field.attrs.iter().any(Self::is_attribute_builder))
                {
                    true => {
                        quote! {
                            let #field_name = self.#field_name.clone();
                        }
                    }
                    false => {
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
