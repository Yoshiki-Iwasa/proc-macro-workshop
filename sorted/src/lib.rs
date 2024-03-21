use std::cmp::Ordering;

use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Error, Item, Meta,
};

#[derive(Default)]
struct MatchVisitor {
    error: Option<syn::Error>,
}

impl MatchVisitor {
    pub fn check_arm_sorted(i: &syn::ExprMatch) -> syn::Result<()> {
        let pats = i
            .arms
            .iter()
            .map(|arm| (arm.pat.clone(), arm.span()))
            .collect::<Vec<_>>();

        let paths = pats
            .iter()
            .map(|(pat, span)| {
                let path = match pat {
                    syn::Pat::Path(expr_path) => &expr_path.path,
                    syn::Pat::Struct(pat_struct) => &pat_struct.path,
                    syn::Pat::TupleStruct(tuple_struct) => &tuple_struct.path,
                    _ => unimplemented!("sorted macro is only for fn, struct, tuple"),
                };

                let new_path = path
                    .segments
                    .clone()
                    .into_iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("");
                (new_path, span)
            })
            .collect::<Vec<_>>();
        if let Some(Err(e)) = paths.iter().map(syn::Result::Ok).reduce(|prev, now| {
            let prev = prev?;
            let now = now?;
            let (prev_path, _) = &prev;
            let (now_path, now_span) = &now;

            if prev_path.cmp(now_path) == Ordering::Greater {
                return Err(syn::Error::new(
                    *(*now_span),
                    format!("{} should sort before {}", now_path, prev_path),
                ));
            }
            Ok(prev)
        }) {
            return Err(e);
        };

        Ok(())
    }
}

impl VisitMut for MatchVisitor {
    fn visit_expr_match_mut(&mut self, i: &mut syn::ExprMatch) {
        if let Some((index, _)) = i.attrs.iter().enumerate().find(|(_index, attr)| {
            if let Meta::Path(path) = &attr.meta {
                path.is_ident("sorted")
            } else {
                false
            }
        }) {
            i.attrs.remove(index);
            match Self::check_arm_sorted(i) {
                Ok(()) => visit_mut::visit_expr_match_mut(self, i),
                Err(e) => self.error = Some(e),
            }
        }
    }
}

#[proc_macro_attribute]
pub fn check(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut item_fn: syn::ItemFn = parse_macro_input!(input);

    let mut match_visitor = MatchVisitor::default();

    visit_mut::visit_item_fn_mut(&mut match_visitor, &mut item_fn);

    if let Some(e) = match_visitor.error {
        let error_stream = e.into_compile_error();
        quote! {
            #error_stream

            #item_fn
        }
    } else {
        quote! {
            #item_fn
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn sorted(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = args;
    let input = parse_macro_input!(input as Item);
    match check_sorted(input.clone()) {
        Ok(input_stream) => input_stream,
        Err(e) => {
            let error_stream = e.into_compile_error();
            quote! {
                #error_stream

                #input
            }
        }
    }
    .into()
}

fn check_sorted(input: Item) -> syn::Result<proc_macro2::TokenStream> {
    let Item::Enum(item_enum) = &input else {
        return Err(Error::new(
            Span::call_site(),
            "expected enum or match expression",
        ));
    };

    let variants = &item_enum.variants;

    if let Some(Err(e)) = variants.iter().map(syn::Result::Ok).reduce(|prev, now| {
        let prev = prev?;
        let now = now?;

        if prev.ident.cmp(&now.ident) == Ordering::Greater {
            return Err(syn::Error::new(
                now.span(),
                format!("{} should sort before {}", now.ident, prev.ident),
            ));
        }
        Ok(prev)
    }) {
        return Err(e);
    };

    let token_stream = quote! {
        #input
    };

    Ok(token_stream)
}
