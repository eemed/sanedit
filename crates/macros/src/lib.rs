extern crate proc_macro;
use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{AttributeArgs, Ident};

/// Converts the function into an Action. The original function name will be
/// used as the action name and the function will be suffixed with
/// `_action_impl`.
#[proc_macro_attribute]
pub fn action(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as AttributeArgs);
    let desc = {
        let desc = args.first();
        if desc.is_none() {
            return quote! {
                compile_error!("Action needs a description #[action(<description>)]");
            }
            .into();
        }
        desc.unwrap()
    };

    let (fun, name) = {
        // Add _action_impl suffix to function name, and create an action
        let mut f = syn::parse_macro_input!(item as syn::ItemFn);
        let name = f.sig.ident;
        let func_name = {
            let ident = format!("{name}_action_impl");
            Ident::new(&ident, Span::call_site().into())
        };
        f.sig.ident = func_name;

        (f, name)
    };

    let fname = &fun.sig.ident;

    let result = quote! {
        #fun

        #[allow(non_upper_case_globals)]
        pub(crate) const #name: crate::actions::Action = crate::actions::Action::Static {
            name: stringify!(#name),
            fun: #fname,
            desc: #desc,
        };
    };
    result.into()
}
