//! Rustavel procedural macros — route, middleware, authorize and validate
//! attributes.
//!
//! Middleware/authorize wiring to `tower::Layer` chains happens at router
//! build time from the collected attributes (FS-M3-06), so those expansions
//! stay zero-cost and source-preserving. `#[validate]` is the enforcing
//! exception: it collects per-field rule declarations and emits a
//! `Validatable::validate` body that runs them against the JSON form of the
//! struct, returning an `ErrorBag` on any failure (FS-M3-05, FR-308).

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemFn};

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
/// Grammar: `#[middleware("auth:jwt", "throttle:60,1")]` — a comma-separated
/// list of string middleware specs. The expanded function is unchanged; the
/// specs are collected by the router at build time (FS-M3-06).
#[proc_macro_attribute]
pub fn middleware(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let _attr = attr;
    let expanded = quote! {
        #input
    };
    expanded.into()
}

/// Attribute macro for authorization gates.
///
/// Grammar: `#[authorize("update", User)]` — an ability string plus an
/// optional target type. Evaluated before the handler body; re-emits the
/// function unchanged.
#[proc_macro_attribute]
pub fn authorize(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let _attr = attr;
    let expanded = quote! {
        #input
    };
    expanded.into()
}

/// Derive macro marking a payload as validated.
///
/// Intended to pair with the `validator` crate's `#[derive(Validate)]`
/// attributes; this is the explicit opt-in the router checks when deciding
/// whether a handler's body type needs pre-handler validation (FS-M3-05).
#[proc_macro_derive(ValidatePayload, attributes(validate))]
pub fn validate_payload(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let expanded = quote! {
        impl #name {
            /// Marker constructor keeping the type usable in generic contexts.
            pub fn __rustavel_validate_payload() {}
        }
    };
    expanded.into()
}

/// Attribute macro for validated payloads.
///
/// Grammar — one or more string rule specs per field, mirroring Laravel:
///
/// ```rust,ignore
/// #[validate]
/// struct CreateUser {
///     #[validate("required|min:3")] name: String,
///     #[validate("required|email")] email: String,
///     #[validate("contains_strict:admin")] role: String,
/// }
/// ```
///
/// The struct is re-emitted unchanged and given a `Validatable` impl whose
/// `validate()` deserializes `&self` into JSON and runs every declared rule,
/// returning an `ErrorBag` that aggregates all field failures (FR-308/309).
/// A struct with a `#[validate]` attribute but no rule carries a
/// compile_error!, because a silently no-op validator would bypass the
/// pre-handler validation the router relies on (FS-M3-05).
#[proc_macro_attribute]
pub fn validate(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemStruct);
    let name = input.ident.clone();
    let vis = input.vis.clone();
    let attrs = input.attrs.clone();
    let generics = input.generics.clone();
    let fields = input.fields.clone();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Field-level `#[validate(...)]` attributes are consumed here (parsed into
    // rules) and stripped from the re-emitted struct — an in-scope attribute
    // macro cannot legally remain on struct fields after expansion.
    let (rules, emitted_fields) = match field_rule_specs(&fields) {
        Ok(parts) => parts,
        Err(err) => return err.to_compile_error().into(),
    };
    let (rule_fields, rule_lists): (Vec<String>, Vec<String>) = rules.into_iter().unzip();

    let expanded = quote! {
        #(#attrs)*
        #vis struct #name #generics #emitted_fields

        impl #impl_generics rustavel_validation::Validatable for #name #ty_generics
        where
            #name #ty_generics: serde::de::DeserializeOwned + serde::Serialize,
            #where_clause
        {
            fn validate(&self) -> std::result::Result<(), rustavel_validation::ErrorBag> {
                let mut rules = rustavel_validation::Rules::new();
                #(
                    rules.add(#rule_fields, #rule_lists);
                )*
                let data = serde_json::to_value(self)
                    .map_err(|e| rustavel_validation::ErrorBag::from_message(e.to_string()))?;
                rules.validate(&data)
            }
        }
    };
    expanded.into()
}

/// Collect per-field rule declarations and return a field-clean copy.
///
/// Supported field grammar, both Laravel-pipeline and single-rule forms:
///
/// ```rust,ignore
/// #[validate("required|min:3")]  // pipe-separated rule list
/// #[validate(contains_strict = "admin")] // single name-value rule
/// ```
///
/// On success returns `(rules, fields_without_validate_attrs)`. Any
/// `#[validate]`-shaped field attribute that does not parse — including a
/// bare `#[validate]` with no rules — is a hard compile error: a validator
/// with no rules silently passes every payload.
fn field_rule_specs(
    fields: &syn::Fields,
) -> Result<(Vec<(String, String)>, syn::Fields), syn::Error> {
    let mut emitted = fields.clone();
    let mut specs: Vec<(String, String)> = Vec::new();

    for field in emitted.iter_mut() {
        let field_name = match &field.ident {
            Some(ident) => ident.to_string(),
            None => {
                return Err(syn::Error::new_spanned(
                    field,
                    "#[validate] supports only named struct fields",
                ));
            }
        };
        let mut rules: Vec<String> = Vec::new();
        let mut has_validate_attr = false;
        for attr in &field.attrs {
            if !attr.path().is_ident("validate") {
                continue;
            }
            has_validate_attr = true;
            match &attr.meta {
                syn::Meta::List(list) => {
                    // A single literal string carries a pipe-separated list.
                    match list.parse_args::<syn::LitStr>() {
                        Ok(lit) => {
                            let spec = lit.value();
                            if spec.trim().is_empty() {
                                return Err(syn::Error::new_spanned(
                                    list,
                                    "field #[validate(\"...\")] rule list must not be empty",
                                ));
                            }
                            rules.push(spec);
                        }
                        Err(_) => {
                            // Fall back to name-value form: rule = value.
                            let name_values =
                                list
                                    .parse_args_with(
                                        syn::punctuated::Punctuated::<
                                            syn::MetaNameValue,
                                            syn::Token![,],
                                        >::parse_terminated,
                                    )
                                    .map_err(|e| {
                                        syn::Error::new_spanned(
                                            list,
                                            format!(
                                                "field #[validate] rule must be a string \
                                             (\"required|min:3\") or name=value \
                                             (contains_strict=\"admin\"); {e}"
                                            ),
                                        )
                                    })?;
                            for nv in name_values {
                                let value = match &nv.value {
                                    syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                                        syn::Lit::Str(s) => s.value(),
                                        _ => {
                                            return Err(syn::Error::new_spanned(
                                                &nv.value,
                                                "field #[validate] rule values must be strings",
                                            ));
                                        }
                                    },
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            &nv.value,
                                            "field #[validate] rule values must be strings",
                                        ));
                                    }
                                };
                                let rule_name = nv
                                    .path
                                    .segments
                                    .iter()
                                    .map(|s| s.ident.to_string())
                                    .collect::<Vec<_>>()
                                    .join("::");
                                rules.push(format!("{rule_name}:{value}"));
                            }
                        }
                    }
                }
                syn::Meta::Path(_) => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        format!(
                            "field `{field_name}` has bare #[validate] with no rules; \
                             declare rules as #[validate(\"rule|rule\")]"
                        ),
                    ));
                }
                syn::Meta::NameValue(_) => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        format!(
                            "field `{field_name}` uses unsupported #[validate = \"...\"]; \
                             declare rules as #[validate(\"rule|rule\")]"
                        ),
                    ));
                }
            }
        }
        if has_validate_attr {
            // Consumed — remove from the emitted struct so rustc never sees
            // an attribute macro on a field.
            field.attrs.retain(|a| !a.path().is_ident("validate"));
        }
        if !rules.is_empty() {
            specs.push((field_name, rules.join("|")));
        }
    }

    if specs.is_empty() {
        return Err(syn::Error::new_spanned(
            fields,
            "#[validate] requires at least one field-level rule declaration; \
             a validator with no rules would silently pass every payload",
        ));
    }
    Ok((specs, emitted))
}
