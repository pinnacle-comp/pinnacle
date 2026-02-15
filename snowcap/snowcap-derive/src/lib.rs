use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

mod signal;

/// Signal derive macro.
#[proc_macro_derive(Signal)]
pub fn derive_signal(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let generics = signal::add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics Signal for #name #ty_generics #where_clause {}
    };

    expanded.into()
}

/// [`Universal`] derive macro.
#[proc_macro_derive(Universal)]
pub fn derive_universal(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics Universal for #name #ty_generics #where_clause {}
    };

    expanded.into()
}
