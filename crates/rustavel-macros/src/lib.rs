//! Rustavel procedural macros — route and middleware attributes.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Attribute macro for route handlers.
///
/// Re-emits the annotated function unchanged with a marker comment.
#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let _attr = attr;
    let expanded = quote! {
        #input
    };
    expanded.into()
}

/// Attribute macro for middleware handlers.
///
/// Re-emits the annotated function unchanged with a marker comment.
#[proc_macro_attribute]
pub fn middleware(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let _attr = attr;
    let expanded = quote! {
        #input
    };
    expanded.into()
}
