// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned};
use syn::{
    parse::Parse, parse_macro_input, punctuated::Punctuated, spanned::Spanned, Expr, Lit,
    MetaNameValue, ReturnType, Stmt, Token,
};

/// Transform the annotated function into one used to configure the Pinnacle compositor.
///
/// This will cause the function to connect to Pinnacle's gRPC server, run your configuration code,
/// then block until Pinnacle exits.
///
/// This function will not return unless an error occurs.
///
/// # Usage
/// The function must be marked `async`, as this macro will insert the `#[tokio::main]` macro below
/// it.
///
/// ```
/// #[pinnacle_api::config]
/// async fn main() {}
/// ```
///
/// `pinnacle_api` annotates the function with a bare `#[tokio::main]` attribute.
/// If you would like to configure Tokio's options, pass in
/// `internal_tokio = false` to this macro and annotate the function
/// with your own `tokio::main` attribute.
///
/// `pinnacle_api` provides a re-export of `tokio` that may prove useful. If you need other Tokio
/// features, you may need to bring them in with your own Cargo.toml.
///
/// Note: the `tokio::main` attribute must be inserted *below* the `pinnacle_api::config`
/// attribute, as attributes are expanded from top to bottom.
///
/// ```
/// #[pinnacle_api::config(internal_tokio = false)]
/// #[pinnacle_api::tokio::main(worker_threads = 8)]
/// async fn main() {}
/// ```
#[proc_macro_attribute]
pub fn config(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as syn::ItemFn);
    let macro_input = parse_macro_input!(args as MacroOptions);

    let vis = item.vis;
    let sig = item.sig;

    if sig.asyncness.is_none() {
        return quote_spanned! {sig.fn_token.span()=>
            compile_error!("this function must be marked `async` to run a Pinnacle config");
        }
        .into();
    }

    if let ReturnType::Type(_, ty) = sig.output {
        return quote_spanned! {ty.span()=>
            compile_error!("this function must not have a return type");
        }
        .into();
    }

    let attrs = item.attrs;

    let stmts = item.block.stmts;

    if let Some(ret @ Stmt::Expr(Expr::Return(_), _)) = stmts.last() {
        return quote_spanned! {ret.span()=>
            compile_error!("this function must not return, as it awaits futures after the end of this statement");
        }.into();
    }

    let options = macro_input.options;

    let mut has_internal_tokio = false;
    let mut has_internal_tracing = false;

    let mut internal_tokio = true;
    let mut internal_tracing = true;

    for name_value in options.iter() {
        if name_value.path.get_ident() == Some(&Ident::new("internal_tokio", Span::call_site())) {
            if has_internal_tokio {
                return quote_spanned! {name_value.path.span()=>
                    compile_error!("`internal_tokio` defined twice, remove this one");
                }
                .into();
            }

            has_internal_tokio = true;
            if let Expr::Lit(lit) = &name_value.value {
                if let Lit::Bool(bool) = &lit.lit {
                    internal_tokio = bool.value;
                    continue;
                }
            }

            return quote_spanned! {name_value.value.span()=>
                compile_error!("expected `true` or `false`");
            }
            .into();
        } else if name_value.path.get_ident()
            == Some(&Ident::new("internal_tracing", Span::call_site()))
        {
            if has_internal_tracing {
                return quote_spanned! {name_value.path.span()=>
                    compile_error!("`internal_tracing` defined twice, remove this one");
                }
                .into();
            }

            has_internal_tracing = true;
            if let Expr::Lit(lit) = &name_value.value {
                if let Lit::Bool(bool) = &lit.lit {
                    internal_tracing = bool.value;
                    continue;
                }
            }
        } else {
            return quote_spanned! {name_value.path.span()=>
                compile_error!("expected valid option (`internal_tokio` or `internal_tracing`)");
            }
            .into();
        }
    }

    let tokio_attr = internal_tokio.then(|| {
        quote! {
            #[::pinnacle_api::tokio::main(crate = "::pinnacle_api::tokio")]
        }
    });

    let tracing_fn = internal_tracing.then(|| {
        quote! {
            ::pinnacle_api::set_default_tracing_subscriber();
        }
    });

    quote! {
        #(#attrs)*
        #tokio_attr
        #vis #sig {
            #tracing_fn

            ::pinnacle_api::connect().await.unwrap();

            #(#stmts)*

            ::pinnacle_api::listen().await;
        }
    }
    .into()
}

struct MacroOptions {
    options: Punctuated<MetaNameValue, Token![,]>,
}

impl Parse for MacroOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let options = input.parse_terminated(MetaNameValue::parse, Token![,])?;

        Ok(MacroOptions { options })
    }
}
