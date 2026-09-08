//! `#[derive(Model)]` — ORM model contract expansion (M2).
//!
//! Emits a `rustavel_orm::model::Model` impl for a plain data struct holding
//! row fields. Column metadata (soft deletes, timestamps, the `id` primary
//! key, update tracking) is inferred from the struct fields, matching the
//! existing hand-written implementations in `rustavel_orm::factory::User`.
//!
//! Supported annotations (all optional):
//!
//! ```rust,ignore
//! #[derive(Model)]
//! #[model(table = "people")]        // default: snake_plural(TypeName)
//! #[model(soft_deletes = "none")]   // default: auto (field `deleted_at`)
//! #[model(timestamps = "none")]     // default: auto (fields created_at/updated_at)
//! struct User { id: Uuid, ... }
//! ```

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Fields};

use crate::model_helpers::{column_name, is_created_at, is_deleted_at, is_updated_at};

/// Parse the `#[model(...)]` container metadata.
fn parse_container(input: &DeriveInput) -> (Option<String>, bool, bool) {
    let mut table: Option<String> = None;
    let mut soft_deletes = true;
    let mut timestamps = true;
    for attr in &input.attrs {
        if !attr.path().is_ident("model") {
            continue;
        }
        if let syn::Meta::List(list) = &attr.meta {
            let pairs = match list.parse_args_with(
                syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated,
            ) {
                Ok(pairs) => pairs,
                Err(_) => continue,
            };
            for pair in pairs {
                let key = pair
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                let value = match &pair.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => s.value(),
                    _ => continue,
                };
                match key.as_str() {
                    "table" => table = Some(value),
                    "soft_deletes" => soft_deletes = value != "none",
                    "timestamps" => timestamps = value != "none",
                    _ => {}
                }
            }
        }
    }
    (table, soft_deletes, timestamps)
}

/// Build the `Model` impl for `#[derive(Model)]`.
pub fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (table, soft_deletes, timestamps) = parse_container(input);

    // Reject non-struct targets early with a spanned error.
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "#[derive(Model)] requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(Model)] can only be applied to structs",
            ));
        }
    };

    let mut has_id = false;
    let mut has_created = false;
    let mut has_updated = false;
    let mut has_deleted = false;
    let mut id_field = syn::Ident::new("id", proc_macro2::Span::call_site());
    let mut touch_fields: Vec<TokenStream> = Vec::new();

    for field in &fields.named {
        let ident = match &field.ident {
            Some(ident) => ident.clone(),
            None => continue,
        };
        let col = column_name(&ident.to_string());
        match col.as_str() {
            "id" if field.ty.to_token_stream().to_string().contains("Uuid") => {
                has_id = true;
                id_field = ident.clone();
            }
            _ => {}
        }
        if is_created_at(&col) {
            has_created = true;
        }
        if is_updated_at(&col) {
            has_updated = true;
            touch_fields.push(quote! { self.#ident = chrono::Utc::now(); });
        }
        if is_deleted_at(&col) {
            has_deleted = true;
        }
    }

    if !has_id {
        return Err(syn::Error::new_spanned(
            input,
            "#[derive(Model)] requires an `id: uuid::Uuid` primary key field",
        ));
    }

    let soft = soft_deletes && has_deleted;
    let ts = timestamps && has_created && has_updated;
    let type_name = name.to_string();

    // An explicit `#[model(table = "...")]` wins; otherwise snake_plural.
    let table_expr = match table {
        Some(table) => quote! { #table.to_string() },
        None => {
            let table = crate::model_helpers::table_name(&type_name);
            quote! { #table.to_string() }
        }
    };

    let uses_soft = if soft {
        quote! { true }
    } else {
        quote! { false }
    };
    let uses_ts = if ts {
        quote! { true }
    } else {
        quote! { false }
    };
    let touch_body = if ts && !touch_fields.is_empty() {
        quote! { #(#touch_fields)* }
    } else {
        quote! {}
    };

    Ok(quote! {
        #[automatically_derived]
        impl rustavel_orm::model::Model for #name {
            /// Type name driving default table derivation.
            fn type_name() -> &'static str {
                #type_name
            }

            /// Table name (`snake_plural` convention, overridable via `#[model(table)]`).
            fn table_name() -> String {
                #table_expr
            }

            /// Whether this model soft-deletes via `deleted_at`.
            fn uses_soft_deletes() -> bool {
                #uses_soft
            }

            /// Whether this model maintains `created_at`/`updated_at`.
            fn uses_timestamps() -> bool {
                #uses_ts
            }

            /// Primary key value.
            fn primary_key(&self) -> uuid::Uuid {
                self.#id_field
            }

            /// Assign a fresh client-generated UUID (v7) before persistence.
            fn assign_id(&mut self) -> uuid::Uuid {
                self.#id_field = uuid::Uuid::now_v7();
                self.#id_field
            }

            /// Bump `updated_at` on the instance (in-memory, pre-save).
            fn touch(&mut self) {
                #touch_body
            }
        }
    })
}
