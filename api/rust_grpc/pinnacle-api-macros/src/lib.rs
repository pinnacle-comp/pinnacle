use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn config(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as syn::ItemFn);
    let module_ident = parse_macro_input!(args as syn::Ident);

    let vis = item.vis;
    let sig = item.sig;
    let attrs = item.attrs;
    let stmts = item.block.stmts;

    quote! {
        #(#attrs)*
        #vis #sig {
            let (#module_ident, __fut_receiver) = ::pinnacle_api::connect().await.unwrap();

            #(#stmts)*

            ::pinnacle_api::listen(__fut_receiver).await;
        }
    }
    .into()
}
