use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned};
use syn::{
    parse::Parse, parse_macro_input, punctuated::Punctuated, spanned::Spanned, Expr, Lit,
    MetaNameValue, Token,
};

/// Transform the annotated function into one used to configure the Pinnacle compositor.
///
/// This will cause the function to connect to Pinnacle's gRPC server, run your configuration code,
/// then await all necessary futures needed to call callbacks.
///
/// This function will not return unless an error occurs.
///
/// # Usage
/// The function must be marked `async`, as this macro will insert the `#[tokio::main]` macro below
/// it.
///
/// It takes in an ident, with which Pinnacle's `ApiModules` struct will be bound to.
///
/// ```
/// #[pinnacle_api::config(modules)]
/// async fn main() {
///     // `modules` is now accessible in the function body
///     let ApiModules { .. } = modules;
/// }
/// ```
///
/// `pinnacle_api` annotates the function with a bare `#[tokio::main]` attribute.
/// If you would like to configure Tokio's options, additionally pass in
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
/// #[pinnacle_api::config(modules, internal_tokio = false)]
/// #[pinnacle_api::tokio::main(worker_threads = 8)]
/// async fn main() {}
/// ```
#[proc_macro_attribute]
pub fn config(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as syn::ItemFn);
    let macro_input = parse_macro_input!(args as MacroInput);

    let vis = item.vis;
    let sig = item.sig;

    if sig.asyncness.is_none() {
        return quote_spanned! {sig.fn_token.span()=>
            compile_error!("This function must be marked `async` to run a Pinnacle config");
        }
        .into();
    }

    let attrs = item.attrs;

    let stmts = item.block.stmts;

    let module_ident = macro_input.ident;

    let options = macro_input.options;

    let mut has_internal_tokio = false;

    let mut internal_tokio = true;

    if let Some(options) = options {
        for name_value in options.iter() {
            if name_value.path.get_ident() == Some(&Ident::new("internal_tokio", Span::call_site()))
            {
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
            } else {
                return quote_spanned! {name_value.path.span()=>
                    compile_error!("expected valid option (currently only `internal_tokio`)");
                }
                .into();
            }
        }
    }

    let tokio_attr = internal_tokio.then(|| {
        quote! {
            #[::pinnacle_api::tokio::main]
        }
    });

    quote! {
        #(#attrs)*
        #tokio_attr
        #vis #sig {
            let (#module_ident, __fut_receiver) = ::pinnacle_api::connect().await.unwrap();

            #(#stmts)*

            ::pinnacle_api::listen(__fut_receiver).await;
        }
    }
    .into()
}

struct MacroInput {
    ident: syn::Ident,
    options: Option<Punctuated<MetaNameValue, Token![,]>>,
}

impl Parse for MacroInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;

        let comma = input.parse::<Token![,]>();

        let mut options = None;

        if comma.is_ok() {
            options = Some(input.parse_terminated(MetaNameValue::parse, Token![,])?);
        }

        if !input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "expected `,` followed by options",
            ));
        }

        Ok(MacroInput { ident, options })
    }
}
