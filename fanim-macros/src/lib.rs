use proc_macro::TokenStream;
use quote::quote;

mod add;
mod lerp;
mod param;

#[proc_macro_derive(Lerp)]
pub fn derive_lerp(input: TokenStream) -> TokenStream {
    lerp::derive_lerp_inner(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Add)]
pub fn derive_add(input: TokenStream) -> TokenStream {
    add::derive_add_inner(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Param)]
pub fn derive_param(input: TokenStream) -> TokenStream {
    param::derive_param_inner(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn the_crate() -> proc_macro2::TokenStream {
    let found_crate =
        proc_macro_crate::crate_name("fanim").expect("fanim is present in `Cargo.toml`");
    match found_crate {
        proc_macro_crate::FoundCrate::Itself => quote! { crate },
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote! { #ident }
        }
    }
}

enum Fields {
    Named(Vec<syn::Ident>),
    Unnamed(Vec<syn::Member>),
}

fn parse_input(input: TokenStream) -> syn::Result<(syn::Ident, Fields)> {
    let input: syn::DeriveInput = syn::parse(input)?;
    let name = input.ident;
    Ok(match input.data {
        syn::Data::Struct(d) => match d.fields {
            syn::Fields::Named(f) => (
                name,
                Fields::Named(
                    f.named
                        .iter()
                        .map(|f| f.ident.clone().unwrap())
                        .collect::<Vec<_>>(),
                ),
            ),
            syn::Fields::Unnamed(f) => (
                name,
                Fields::Unnamed(
                    (0..f.unnamed.len())
                        .map(|i| {
                            syn::Member::Unnamed(syn::Index {
                                index: i as u32,
                                span: proc_macro2::Span::call_site(),
                            })
                        })
                        .collect(),
                ),
            ),
            _ => panic!("struct must contain data"),
        },
        _ => panic!("only structs are supported"),
    })
}
