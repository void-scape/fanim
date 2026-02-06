use convert_case::Casing;
use proc_macro::TokenStream;
use quote::quote;

pub fn derive_param_inner(input: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let input: syn::DeriveInput = syn::parse(input)?;
    let name = &input.ident;
    let snake_case_name = syn::Ident::new(
        &input.ident.to_string().to_case(convert_case::Case::Snake),
        proc_macro2::Span::call_site(),
    );
    let the_crate = crate::the_crate();
    Ok(quote! {
        impl #name {
            pub fn system(
                mut params: bevy_ecs::prelude::Query<(&#name, &mut #the_crate::params::Params),
                bevy_ecs::prelude::Changed<#name>>,
            ) {
                for (component, mut params) in params.iter_mut() {
                    params.#snake_case_name = component.clone();
                }
            }
        }
    })
}
