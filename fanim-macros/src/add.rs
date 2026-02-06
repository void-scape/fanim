use proc_macro::TokenStream;
use quote::quote;

pub fn derive_add_inner(input: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let (name, fields) = crate::parse_input(input)?;
    Ok(match fields {
        crate::Fields::Named(fields) => {
            quote! {
                impl core::ops::Add for #name {
                    type Output = Self;
                    fn add(self, rhs: Self) -> Self {
                        Self {
                            #(#fields: self.#fields + rhs.#fields),*
                        }
                    }
                }
            }
        }
        crate::Fields::Unnamed(fields) => {
            quote! {
                impl core::ops::Add for #name {
                    type Output = Self;
                    fn add(self, rhs: Self) -> Self {
                        Self(#(self.#fields + rhs.#fields),*)
                    }
                }
            }
        }
    })
}
