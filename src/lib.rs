#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
  parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Error, Ident, Path, Token, Type,
  WherePredicate,
};

mod attrs;
mod codegen;

use attrs::{ContainerAttrs, FieldAttrs, VariantAttrs};

/// Derive a native [`quickcheck::Arbitrary`] implementation.
///
/// See the crate-level documentation and `README.md` for the supported
/// `#[quickcheck(...)]` attributes.
#[proc_macro_derive(Arbitrary, attributes(quickcheck))]
pub fn derive_arbitrary(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  expand(input)
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
  let container = ContainerAttrs::parse(&input.attrs)?;
  let qc = container.crate_path();

  // Validate *every* field's and variant's attributes upfront, regardless of
  // which codegen path is later taken, so unknown/conflicting attrs are rejected
  // even in positions a particular path would otherwise skip.
  validate_all_attrs(&input)?;

  // Allocates all macro-internal identifiers so none collides with a user
  // `const` parameter of the same spelling (which bypasses macro hygiene).
  let hyg = codegen::Hygiene::new(&input);

  // The `&mut Gen` parameter ident, shared by the impl signature and every
  // generated body (collision-free + `mixed_site`).
  let g = hyg.ident("__quickcheck_g");

  // The `Box` type for `shrink`'s return: an explicit `box = "..."` override, or
  // `::std::boxed::Box` (`std` feature) / `::alloc::boxed::Box` (no-std).
  let box_ty = box_path(&container);

  let name = &input.ident;
  let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
  let where_clause = build_where_clause(&input, &container, &qc);

  let (arbitrary_body, shrink_body) = match &input.data {
    Data::Struct(data) => {
      codegen::struct_bodies(name, &data.fields, &container, &g, &hyg, &box_ty, &qc)?
    }
    Data::Enum(data) => codegen::enum_bodies(name, data, &container, &g, &hyg, &box_ty, &qc)?,
    Data::Union(_) => {
      return Err(Error::new_spanned(
        &input,
        "`#[derive(Arbitrary)]` does not support unions",
      ))
    }
  };

  Ok(quote! {
    const _: () = {
      impl #impl_generics #qc::Arbitrary for #name #ty_generics #where_clause {
        fn arbitrary(#g: &mut #qc::Gen) -> Self {
          #arbitrary_body
        }

        fn shrink(&self) -> #box_ty<dyn ::core::iter::Iterator<Item = Self>> {
          #shrink_body
        }
      }
    };
  })
}

/// The `Box` path for `shrink`'s return type: an explicit container
/// `box = "..."`, else `::std::boxed::Box` (with the `std` feature) or
/// `::alloc::boxed::Box` (no-std).
fn box_path(container: &ContainerAttrs) -> Path {
  if let Some(p) = &container.box_path {
    p.clone()
  } else if cfg!(feature = "std") {
    syn::parse_quote!(::std::boxed::Box)
  } else {
    syn::parse_quote!(::alloc::boxed::Box)
  }
}

/// Build the `where` clause for the impl.
///
/// * If any container `bound` attr is present, the clause is the type's own
///   predicates plus exactly the parsed bound predicates (wholesale replacement
///   of inference).
/// * Otherwise infer two kinds of predicate:
///   - `T: Clone + 'static` for every generic **type** param, to satisfy the
///     `Arbitrary: Clone + 'static` supertrait on `Self` (the type's own
///     `Clone`/`'static` need it; a `<FieldTy>: Arbitrary` bound does *not* imply
///     `T: Clone`); and
///   - `<FieldTy>: #qc::Arbitrary` for each generated field type that mentions a
///     generic param (sound for projections / nested generics — see
///     [`inferred_bound_types`]).
fn build_where_clause(input: &DeriveInput, container: &ContainerAttrs, qc: &Path) -> TokenStream2 {
  let mut predicates: Punctuated<WherePredicate, Token![,]> = input
    .generics
    .where_clause
    .as_ref()
    .map(|w| w.predicates.clone())
    .unwrap_or_default();

  if container.bounds.is_empty() {
    for type_param in input.generics.type_params() {
      let ident = &type_param.ident;
      predicates.push(syn::parse_quote!(#ident: ::core::clone::Clone + 'static));
    }
    for ty in inferred_bound_types(input, container) {
      predicates.push(syn::parse_quote!(#ty: #qc::Arbitrary));
    }
  } else {
    for predicate in &container.bounds {
      predicates.push(predicate.clone());
    }
  }

  if predicates.is_empty() {
    quote!()
  } else {
    quote!(where #predicates)
  }
}

/// The set of **field types** that should receive an inferred `: Arbitrary`
/// bound: the types of fields generated via `<qc>::Arbitrary::arbitrary` that
/// mention at least one generic type param.
///
/// Bounding the field type itself (e.g. `<T as Trait>::Item: Arbitrary`), rather
/// than the type params found inside it (`T: Arbitrary`), is sound for projected
/// / associated types — where `T: Arbitrary` would *not* imply the projection is
/// `Arbitrary` — and for nested generics (`Vec<T>: Arbitrary`). Concrete field
/// types (mentioning no param) are left to the call site, so we don't emit
/// redundant `u32: Arbitrary`-style bounds.
///
/// A field type is considered to "mention a generic param" if it references any
/// generic **type or const** param — so const-generic-bearing field types
/// (`Only<N>`, `[u8; N]`) are bounded too, not just type-param ones.
///
/// Deduplicated, in first-seen order. Parse errors are ignored here — they are
/// surfaced by [`validate_all_attrs`] / codegen, so this best-effort inference
/// simply contributes nothing for an unparsable field.
fn inferred_bound_types(input: &DeriveInput, container: &ContainerAttrs) -> Vec<Type> {
  // Generic type *and* const params (lifetimes never need an `Arbitrary` bound).
  let param_idents: Vec<Ident> = input
    .generics
    .type_params()
    .map(|p| p.ident.clone())
    .chain(input.generics.const_params().map(|p| p.ident.clone()))
    .collect();
  if param_idents.is_empty() {
    return Vec::new();
  }

  // Collect the `syn::Type`s of every field that is generated via `arbitrary`.
  let mut generated_tys: Vec<Type> = Vec::new();

  // A container `with` builds the whole value itself ⇒ no field uses `arbitrary`.
  if container.with.is_none() {
    match &input.data {
      Data::Struct(data) => collect_arbitrary_field_types(&data.fields, &mut generated_tys),
      Data::Enum(data) => {
        for variant in &data.variants {
          let vattrs = match VariantAttrs::parse(&variant.attrs) {
            Ok(v) => v,
            Err(_) => continue,
          };
          // `skip` variants are never generated; variant `with` builds the
          // whole value ⇒ neither contributes field bounds.
          if vattrs.skip || vattrs.with.is_some() {
            continue;
          }
          collect_arbitrary_field_types(&variant.fields, &mut generated_tys);
        }
      }
      Data::Union(_) => {}
    }
  }

  let mut seen = std::collections::HashSet::new();
  generated_tys
    .into_iter()
    .filter(|ty| param_idents.iter().any(|p| type_uses_param(ty, p)))
    .filter(|ty| seen.insert(quote!(#ty).to_string()))
    .collect()
}

/// Push the `syn::Type` of each field in `fields` that is generated via
/// `Arbitrary::arbitrary` (i.e. no field `with` and not `default`).
fn collect_arbitrary_field_types(fields: &syn::Fields, out: &mut Vec<Type>) {
  for field in fields.iter() {
    let attrs = match FieldAttrs::parse(&field.attrs) {
      Ok(a) => a,
      Err(_) => continue,
    };
    if attrs.with.is_none() && !attrs.default {
      out.push(field.ty.clone());
    }
  }
}

/// Whether the type-param `ident` appears anywhere within `ty`.
///
/// Walks the type's token stream (recursing into delimited groups) and matches
/// any identifier token equal to `ident`. This is conservative — a same-named
/// associated path segment would also match — but only ever *adds* a sound
/// `: Arbitrary` bound, never drops a required one.
fn type_uses_param(ty: &Type, ident: &Ident) -> bool {
  fn tokens_contain_ident(tokens: TokenStream2, ident: &Ident) -> bool {
    use proc_macro2::TokenTree;
    tokens.into_iter().any(|tt| match tt {
      TokenTree::Ident(i) => &i == ident,
      TokenTree::Group(g) => tokens_contain_ident(g.stream(), ident),
      _ => false,
    })
  }
  tokens_contain_ident(quote!(#ty), ident)
}

/// Validate *every* field's and variant's `#[quickcheck(...)]` attributes so
/// unknown/conflicting attrs are rejected regardless of the codegen path later
/// taken (container `with`, variant `with`, `skip`, etc. would otherwise skip
/// the relevant `parse_*` call). Validation only — results are discarded.
fn validate_all_attrs(input: &DeriveInput) -> syn::Result<()> {
  match &input.data {
    Data::Struct(data) => {
      for field in data.fields.iter() {
        FieldAttrs::parse(&field.attrs)?;
      }
    }
    Data::Enum(data) => {
      for variant in &data.variants {
        VariantAttrs::parse(&variant.attrs)?;
        for field in variant.fields.iter() {
          FieldAttrs::parse(&field.attrs)?;
        }
      }
    }
    Data::Union(_) => {}
  }
  Ok(())
}
