use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn config(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut item = parse_macro_input!(item as syn::ItemFn);
    let module_ident = parse_macro_input!(args as syn::Ident);

    let vis = item.vis;

    item.sig.inputs = syn::punctuated::Punctuated::new();
    let sig = item.sig;

    let attrs = item.attrs;
    let stmts = item.block.stmts;

    quote! {
        #(#attrs)*
        #vis #sig {
            let (#module_ident, __fut_receiver) = ::pinnacle_api::create_modules().unwrap();

            #(#stmts)*

            ::pinnacle_api::listen(__fut_receiver);
        }
    }
    .into()
}
