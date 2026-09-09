//! RustaSea procedural macros — route, middleware, authorize and validate
//! attributes plus the M5 declarative attribute bundle.
//!
//! Middleware/authorize wiring to `tower::Layer` chains happens at router
//! build time from the collected attributes (FS-M3-06), so those expansions
//! stay zero-cost and source-preserving. `#[validate]` is the enforcing
//! exception: it collects per-field rule declarations and emits a
//! `Validatable::validate` body that runs them against the JSON form of the
//! struct, returning an `ErrorBag` on any failure (FS-M3-05, FR-308).
//!
//! The M5 (DX) attribute surface — `#[tries]`, `#[backoff]`, `#[timeout]`,
//! `#[failOnTimeout]`, `#[withoutBroadcasting]`, `#[repairToolCalls]`,
//! `#[usage]`, `#[help]`, `#[hidden]`, `#[queue]`, `#[connection]` — is
//! defined here at the crate root (required for proc-macro attributes) and
//! delegates parsing/emission to the private [`attrs`] helpers. Every M5
//! attribute is source-preserving and emits doc-hidden helper consts the
//! runtime reads (FS-M5-03, FR-506).

mod attrs;
mod model;
pub(crate) mod model_helpers;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemFn};

/// Attribute macro for route handlers.
///
/// Grammar: `#[route(method = "GET", path = "/users")]` on an `async fn`.
/// The function is re-emitted unchanged and a doc-hidden const
/// `__RUSTASEA_ROUTE_<Fn>` records the method + path metadata the router
/// reads when building the route table:
///
/// ```rust,ignore
/// #[route(method = "GET", path = "/users")]
/// async fn index() -> &'static str { "users" }
/// ```
#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let ident = input.sig.ident.clone();
    match attrs::parse_route(attr.into()) {
        Ok((method, path)) => {
            let const_name = syn::Ident::new(
                &format!("__RUSTASEA_ROUTE_{ident}"),
                proc_macro2::Span::call_site(),
            );
            let expanded = quote! {
                #input

                #[doc(hidden)]
                #[allow(non_upper_case_globals)]
                const #const_name: (&str, &str) = (#method, #path);
            };
            expanded.into()
        }
        Err(err) => err.to_compile_error().into(),
    }
}

/// Attribute macro for middleware handlers.
///
/// Grammar: `#[middleware("auth:jwt", "throttle:60,1")]` — a comma-separated
/// list of string middleware specs. The function is re-emitted unchanged and
/// a doc-hidden const `__RUSTASEA_MIDDLEWARE_<Fn>` records the middleware
/// name list the router reads when wiring `tower::Layer` chains at build time
/// (FS-M3-06):
///
/// ```rust,ignore
/// #[middleware("auth:jwt")]
/// async fn profile() -> &'static str { "profile" }
/// ```
#[proc_macro_attribute]
pub fn middleware(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let ident = input.sig.ident.clone();
    match attrs::parse_string_list(attr.into(), "middleware") {
        Ok(specs) => {
            let const_name = syn::Ident::new(
                &format!("__RUSTASEA_MIDDLEWARE_{ident}"),
                proc_macro2::Span::call_site(),
            );
            let expanded = quote! {
                #input

                #[doc(hidden)]
                #[allow(non_upper_case_globals)]
                const #const_name: &[&str] = &[#(#specs),*];
            };
            expanded.into()
        }
        Err(err) => err.to_compile_error().into(),
    }
}

/// Attribute macro for authorization gates.
///
/// Grammar: `#[authorize("update", User)]` — an ability string plus an
/// optional target type, comma-separated. The function is re-emitted
/// unchanged and a doc-hidden const `__RUSTASEA_AUTHORIZE_<Fn>` records the
/// `(ability, target)` pair; the runtime resolves the enforcing guard from
/// the handler's `#[middleware("auth:<guard>")]` spec and rejects with
/// `GuardMismatch` before the body when the guard is unknown (FS-M3-06,
/// TC-M3-02):
///
/// ```rust,ignore
/// #[authorize("update", User)]
/// async fn update_user() -> &'static str { "updated" }
/// ```
#[proc_macro_attribute]
pub fn authorize(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let ident = input.sig.ident.clone();
    match attrs::parse_authorize(attr.into()) {
        Ok((ability, target)) => {
            let const_name = syn::Ident::new(
                &format!("__RUSTASEA_AUTHORIZE_{ident}"),
                proc_macro2::Span::call_site(),
            );
            let tuple = if target.is_empty() {
                quote! { (#ability, "") }
            } else {
                quote! { (#ability, #target) }
            };
            let expanded = quote! {
                #input

                #[doc(hidden)]
                #[allow(non_upper_case_globals)]
                const #const_name: (&str, &str) = #tuple;
            };
            expanded.into()
        }
        Err(err) => err.to_compile_error().into(),
    }
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
            pub fn __rustasea_validate_payload() {}
        }
    };
    expanded.into()
}

/// Derive macro implementing the ORM `Model` contract (M2).
///
/// Derives `table_name` from the type (`snake_plural` convention) and infers
/// soft deletes / timestamps / the `id` primary key from the struct fields:
///
/// ```rust,ignore
/// #[derive(Model)]
/// #[model(table = "people", soft_deletes = "none")]
/// struct User { id: Uuid, created_at: DateTime<Utc>, ... }
/// ```
///
/// Optional `#[model(...)]` container attributes: `table = "..."` overrides
/// the derived name; `soft_deletes = "none"` / `timestamps = "none"` opt out
/// of inferred columns. Structs must carry a `uuid::Uuid` field named `id`.
#[proc_macro_derive(Model, attributes(model))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match model::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
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

        impl #impl_generics rustasea_validation::Validatable for #name #ty_generics
        where
            #name #ty_generics: serde::de::DeserializeOwned + serde::Serialize,
            #where_clause
        {
            fn validate(&self) -> std::result::Result<(), rustasea_validation::ErrorBag> {
                let mut rules = rustasea_validation::Rules::new();
                #(
                    rules.add(#rule_fields, #rule_lists);
                )*
                let data = serde_json::to_value(self)
                    .map_err(|e| rustasea_validation::ErrorBag::from_message(e.to_string()))?;
                rules.validate(&data)
            }
        }
    };
    expanded.into()
}

/// Attribute: job retry budget.
///
/// Grammar: `#[tries(3)]` on a job type. The value is a usize; the helper
/// const `__RUSTASEA_TRIES_<Type>` records the declarative budget that queue
/// workers honour (an explicit `#[tries]` shadows `ShouldRetry`).
#[proc_macro_attribute]
pub fn tries(attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::usize_attr(attr, item, "tries", "TRIES")
}

/// Attribute: base retry backoff.
///
/// Grammar: `#[backoff(10)]` — base delay in **seconds** before the first
/// retry (later attempts double). Emits `__RUSTASEA_BACKOFF_SECS_<Type>`.
#[proc_macro_attribute]
pub fn backoff(attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::usize_attr(attr, item, "backoff", "BACKOFF_SECS")
}

/// Attribute: per-attempt timeout.
///
/// Grammar: `#[timeout(30)]` — per-attempt timeout in **seconds**; `0`
/// disables the timeout. Emits `__RUSTASEA_TIMEOUT_SECS_<Type>`.
#[proc_macro_attribute]
pub fn timeout(attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::usize_attr(attr, item, "timeout", "TIMEOUT_SECS")
}

/// Attribute: fail (do not release) when a timed-out attempt is observed.
///
/// Grammar: `#[failOnTimeout]` — a bare marker const that the worker reads.
#[proc_macro_attribute]
pub fn fail_on_timeout(_attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::marker_attr(item, "FAIL_ON_TIMEOUT")
}

/// Attribute: suppress event broadcasting for this type.
///
/// Grammar: `#[withoutBroadcasting]` — marker; the dispatcher skips the
/// broadcast fan-out when the type carries this attribute.
#[proc_macro_attribute]
pub fn without_broadcasting(_attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::marker_attr(item, "WITHOUT_BROADCASTING")
}

/// Attribute: enable AI tool-call repair on this agent/tool type.
///
/// Grammar: `#[repairToolCalls]` — marker consumed by the M6 AI SDK; the
/// attribute surface ships in M5 so `make:agent`/`make:tool` generators can
/// emit it early.
#[proc_macro_attribute]
pub fn repair_tool_calls(_attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::marker_attr(item, "REPAIR_TOOL_CALLS")
}

/// Attribute: console usage signature.
///
/// Grammar: `#[usage("app:send {user}")]` — the Laravel-style signature
/// `cargo artisan list` renders in its table and JSON output.
#[proc_macro_attribute]
pub fn usage(attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::string_attr(attr, item, "usage", "USAGE")
}

/// Attribute: console help/description text.
///
/// Grammar: `#[help("Run pending migrations")]` — shown under the command
/// name in `cargo artisan list`.
#[proc_macro_attribute]
pub fn help(attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::string_attr(attr, item, "help", "HELP")
}

/// Attribute: hide the command from `list` unless `--all` is passed.
///
/// Grammar: `#[hidden]` — bare marker const on a command type.
#[proc_macro_attribute]
pub fn hidden(_attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::marker_attr(item, "HIDDEN")
}

/// Attribute: default queue for a job.
///
/// Grammar: `#[queue("emails")]` — overrides the routed queue for the type.
/// Emits `__RUSTASEA_QUEUE_<Type>`.
#[proc_macro_attribute]
pub fn queue(attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::string_attr(attr, item, "queue", "QUEUE")
}

/// Attribute: default connection for a job.
///
/// Grammar: `#[connection("redis")]` — overrides the routed connection.
/// Emits `__RUSTASEA_CONNECTION_<Type>`.
#[proc_macro_attribute]
pub fn connection(attr: TokenStream, item: TokenStream) -> TokenStream {
    attrs::string_attr(attr, item, "connection", "CONNECTION")
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
