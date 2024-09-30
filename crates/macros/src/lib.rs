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

#[proc_macro_derive(DocComment)]
pub fn describe(input: TokenStream) -> TokenStream {
    let syn::DeriveInput { ident, data, .. } = syn::parse_macro_input!(input);

    let (idents, docs) = match data {
        syn::Data::Struct(syn::DataStruct { fields, .. }) => {
            let mut idents = Vec::new();
            let mut docs = Vec::new();
            for var in &fields {
                let comment = parse_doc_comment(&var.attrs);
                docs.push(comment);
                idents.push(var.ident.as_ref().unwrap().clone());
            }

            (idents, docs)
        }
        _ => {
            panic!("only structs are supported")
        }
    };
    let idents: Vec<String> = idents.into_iter().map(|ident| ident.to_string()).collect();
    // Procmacro2 stream, so typeless, no idea why it works though
    let docs: Vec<_> = docs
        .into_iter()
        .map(|opt| match opt {
            Some(c) => quote! { Some(#c) },
            None => quote! { None },
        })
        .collect();

    let output = quote! {
        impl #ident {
            pub fn doc_comment(field: &str) -> Option<&'static str> {
                match field {
                    #( #idents => #docs ),*,
                    _ => None,
                }
            }
        }
    };

    output.into()
}

fn parse_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let mut result = String::new();
    for attr in attrs {
        let meta = attr.parse_meta().unwrap();
        if let syn::Meta::NameValue(name_value) = meta {
            if name_value.path.is_ident("doc") {
                if let syn::Lit::Str(doc) = name_value.lit {
                    result.push_str(&doc.value().trim());
                    result.push('\n');
                }
            }
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}
