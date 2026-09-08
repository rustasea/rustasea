//! Shared parsing/emission helpers for the M5 declarative attribute bundle
//! (FS-M5-03, FR-506).
//!
//! Procedural attribute macros must be defined in the crate root, so every
//! `#[proc_macro_attribute]` entry point lives in `lib.rs`; this private
//! module holds the parsers and the source-preserving emitters they delegate
//! to. Each expansion re-emits the annotated item unchanged and appends a
//! doc-hidden helper const (`__RUSTAVEL_*`) that the runtime discovers
//! reflectively — the consts are inert until a driver reads them.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item};

/// Extract the ident from an annotated struct/enum/union.
///
/// Returns `None` when the attribute is applied to an item without a type
/// name (fn, const, impl, mod), which all of these attributes reject.
pub(crate) fn item_ident(input: &Item) -> Option<syn::Ident> {
    match input {
        Item::Struct(s) => Some(s.ident.clone()),
        Item::Enum(e) => Some(e.ident.clone()),
        Item::Union(u) => Some(u.ident.clone()),
        _ => None,
    }
}

/// Build the fallback ident used when the target item type is unsupported.
///
/// All emitters still re-emit the item unchanged (source-preserving), so the
/// worst case is a misleading helper const name — never a code change.
pub(crate) fn fallback_ident() -> syn::Ident {
    syn::Ident::new("Type", proc_macro2::Span::call_site())
}

/// Parse a positive integer literal from the attribute token stream.
///
/// Accepts a bare literal (`#[tries(3)]`). Trailing comma tokens are
/// tolerated so `#[tries(3,)]` does not break the macro.
pub(crate) fn parse_usize_literal(
    tokens: proc_macro2::TokenStream,
    name: &str,
) -> Result<usize, syn::Error> {
    let mut value: Option<usize> = None;
    for token in tokens {
        match token {
            proc_macro2::TokenTree::Literal(lit) => {
                let text = lit.to_string();
                let parsed: usize = text.parse().map_err(|_| {
                    syn::Error::new_spanned(
                        &lit,
                        format!("#[{name}] expects a positive integer literal"),
                    )
                })?;
                if value.replace(parsed).is_some() {
                    return Err(syn::Error::new_spanned(
                        &lit,
                        format!("#[{name}] expects a single integer literal"),
                    ));
                }
            }
            proc_macro2::TokenTree::Punct(p) if p.as_char() == ',' => {
                // tolerate trailing commas
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    format!("#[{name}] expects a single integer literal"),
                ));
            }
        }
    }
    value.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("#[{name}] requires an integer literal, e.g. #[{name}(3)]"),
        )
    })
}

/// Parse a single string literal argument.
///
/// `#[usage("app:send {user}")]` — a lone literal, optionally wrapped in an
/// extra parenthesis group; surrounding quotes are stripped.
pub(crate) fn parse_string_literal(
    tokens: proc_macro2::TokenStream,
    attr: &str,
) -> Result<String, syn::Error> {
    let mut collected: Vec<proc_macro2::TokenTree> = tokens.into_iter().collect();
    if matches!(collected.first(), Some(proc_macro2::TokenTree::Group(_))) {
        if let proc_macro2::TokenTree::Group(group) = collected.remove(0) {
            collected = group.stream().into_iter().collect();
        }
    }
    let text = collected
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join("");
    let text = text.trim();
    let text = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(text);
    if text.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("#[{attr}] requires a non-empty string literal"),
        ));
    }
    Ok(text.to_string())
}

/// Build a `__RUSTAVEL_*` helper const identifier.
fn const_ident(key: &str, suffix: Option<&syn::Ident>) -> syn::Ident {
    let suffix = suffix.map(|i| format!("_{i}")).unwrap_or_default();
    syn::Ident::new(
        &format!("__RUSTAVEL_{key}{suffix}"),
        proc_macro2::Span::call_site(),
    )
}

/// Emit a usize-typed helper const for the annotated item.
///
/// Grammar: `#[tries(3)]`, `#[backoff(10)]`, `#[timeout(30)]` — appends
/// `const __RUSTAVEL_{KEY}_{Type}: usize` after the unchanged item.
pub(crate) fn usize_attr(
    attr: TokenStream,
    item: TokenStream,
    attribute: &str,
    key: &str,
) -> TokenStream {
    let input = parse_macro_input!(item as Item);
    let ident = item_ident(&input).unwrap_or_else(fallback_ident);
    match parse_usize_literal(attr.into(), attribute) {
        Ok(value) => {
            let const_name = const_ident(key, Some(&ident));
            quote! {
                #input
                #[doc(hidden)]
                #[allow(non_upper_case_globals)]
                const #const_name: usize = #value;
            }
            .into()
        }
        Err(err) => err.to_compile_error().into(),
    }
}

/// Emit a bare boolean marker const for the annotated item.
///
/// Grammar: `#[hidden]`, `#[failOnTimeout]`, `#[withoutBroadcasting]`,
/// `#[repairToolCalls]` — appends `const __RUSTAVEL_{KEY}: bool = true`.
pub(crate) fn marker_attr(item: TokenStream, key: &str) -> TokenStream {
    let input = parse_macro_input!(item as Item);
    let const_name = const_ident(key, None);
    quote! {
        #input
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        const #const_name: bool = true;
    }
    .into()
}

/// Emit a `&'static str`-typed helper const for the annotated item.
///
/// Grammar: `#[usage("...")]`, `#[help("...")]`, `#[queue("...")]`,
/// `#[connection("...")]` — appends `const __RUSTAVEL_{KEY}_{Type}: &str`.
pub(crate) fn string_attr(
    attr: TokenStream,
    item: TokenStream,
    attribute: &str,
    key: &str,
) -> TokenStream {
    let input = parse_macro_input!(item as Item);
    let ident = item_ident(&input).unwrap_or_else(fallback_ident);
    match parse_string_literal(attr.into(), attribute) {
        Ok(text) => {
            let const_name = const_ident(key, Some(&ident));
            quote! {
                #input
                #[doc(hidden)]
                #[allow(non_upper_case_globals)]
                const #const_name: &str = #text;
            }
            .into()
        }
        Err(err) => err.to_compile_error().into(),
    }
}
