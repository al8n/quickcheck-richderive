//! Parsing of `#[quickcheck(...)]` attributes for the container, fields, and
//! enum variants.
//!
//! ## Attribute surface (serde-style)
//!
//! The three `arbitrary` / `shrink` / `with` knobs mirror serde's
//! `serialize_with` / `deserialize_with` / `with` triad:
//!
//! | Attribute | Value shape | Effect |
//! |---|---|---|
//! | `arbitrary = "fn"` | `fn(g: &mut Gen) -> Self` (or `FieldT` at field level) | overrides the gen half |
//! | `shrink = "fn"`    | `fn(v: &Self) -> Box<dyn Iterator<Item = Self>>` | overrides the shrink half |
//! | `with = "mod"`     | a module containing both `mod::arbitrary` and `mod::shrink` | overrides both halves at once |
//!
//! `with` is mutually exclusive with `arbitrary` and `shrink` — the compiler
//! reports a focused error if both forms appear on the same item.

use syn::{Attribute, Error, Path, Token, WherePredicate, parse_str, punctuated::Punctuated};

/// Parse a string-literal value into a `syn::Path`, keeping the literal's span
/// for error reporting.
fn parse_path(lit: &syn::LitStr) -> syn::Result<Path> {
  parse_str::<Path>(&lit.value()).map_err(|e| Error::new(lit.span(), e))
}

/// Parse a string-literal value into a comma-separated list of where-predicates.
fn parse_predicates(lit: &syn::LitStr) -> syn::Result<Vec<WherePredicate>> {
  let parser = Punctuated::<WherePredicate, Token![,]>::parse_terminated;
  syn::parse::Parser::parse_str(parser, &lit.value())
    .map(|p| p.into_iter().collect())
    .map_err(|e| {
      Error::new(
        lit.span(),
        format!("failed to parse `bound` as where-predicates: {e}"),
      )
    })
}

/// Attributes accepted on the struct/enum itself.
#[derive(Default)]
pub(crate) struct ContainerAttrs {
  /// Base path for the emitted `Arbitrary`/`Gen` (default `::quickcheck`).
  pub(crate) krate: Option<Path>,
  /// Accumulated `bound = "..."` predicates (repeatable). If non-empty these
  /// replace the inferred bounds.
  pub(crate) bounds: Vec<WherePredicate>,
  /// `with = "mod"` — a module exporting both `arbitrary(g) -> Self` **and**
  /// `shrink(v: &Self) -> Box<dyn Iterator<Item = Self>>`. Mutually exclusive
  /// with `arbitrary` and `shrink`.
  pub(crate) with: Option<Path>,
  /// `arbitrary = "fn"` — generate the whole value via this function.
  pub(crate) arbitrary: Option<Path>,
  /// `shrink = "fn"` — shrink the whole value via this function.
  pub(crate) shrink: Option<Path>,
  /// `box = "path::to::Box"` — override the `Box` type used in the generated
  /// `shrink` return. Defaults to `::std::boxed::Box` (with the `std` feature) or
  /// `::alloc::boxed::Box` (no-std).
  pub(crate) box_path: Option<Path>,
}

impl ContainerAttrs {
  pub(crate) fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
    let mut out = ContainerAttrs::default();
    for attr in attrs {
      if !attr.path().is_ident("quickcheck") {
        continue;
      }
      attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("crate") {
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.krate = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("bound") {
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.bounds.extend(parse_predicates(&lit)?);
        } else if meta.path.is_ident("with") {
          // Mutex check at the *offending* keyword's span — narrower and more
          // localized than a post-parse pass anchored on the whole attribute,
          // which would render differently across rustc versions.
          if out.arbitrary.is_some() {
            return Err(meta.error(
              "`with` and `arbitrary` are mutually exclusive on a container — \
               `with = \"mod\"` already provides `arbitrary` via `mod::arbitrary`",
            ));
          }
          if out.shrink.is_some() {
            return Err(meta.error(
              "`with` and `shrink` are mutually exclusive on a container — \
               `with = \"mod\"` already provides `shrink` via `mod::shrink`",
            ));
          }
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.with = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("arbitrary") {
          if out.with.is_some() {
            return Err(meta.error(
              "`arbitrary` and `with` are mutually exclusive on a container — \
               `with = \"mod\"` already provides `arbitrary` via `mod::arbitrary`",
            ));
          }
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.arbitrary = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("shrink") {
          if out.with.is_some() {
            return Err(meta.error(
              "`shrink` and `with` are mutually exclusive on a container — \
               `with = \"mod\"` already provides `shrink` via `mod::shrink`",
            ));
          }
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.shrink = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("box") {
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.box_path = Some(parse_path(&lit)?);
        } else {
          return Err(meta.error(
            "unknown container attribute; expected `crate`, `bound`, `with`, \
             `arbitrary`, `shrink`, or `box`",
          ));
        }
        Ok(())
      })?;
    }
    Ok(out)
  }

  /// The quickcheck base path (default `::quickcheck`).
  pub(crate) fn crate_path(&self) -> Path {
    self
      .krate
      .clone()
      .unwrap_or_else(|| syn::parse_quote!(::quickcheck))
  }
}

/// Attributes accepted on a struct field or a variant field.
#[derive(Default)]
pub(crate) struct FieldAttrs {
  /// `with = "mod"` — a module exporting `arbitrary(g) -> FieldT` and
  /// `shrink(v: &FieldT) -> Box<dyn Iterator<Item = FieldT>>`. Mutex with
  /// `arbitrary`/`shrink`/`default`.
  pub(crate) with: Option<Path>,
  /// `arbitrary = "fn"` — generate this field via this function.
  pub(crate) arbitrary: Option<Path>,
  /// `shrink = "fn"` — shrink this field via this function.
  pub(crate) shrink: Option<Path>,
  /// `default` — generate via `Default::default()` and never shrink.
  pub(crate) default: bool,
}

impl FieldAttrs {
  pub(crate) fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
    let mut out = FieldAttrs::default();
    for attr in attrs {
      if !attr.path().is_ident("quickcheck") {
        continue;
      }
      attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("with") {
          if out.arbitrary.is_some() {
            return Err(meta.error(
              "`with` and `arbitrary` are mutually exclusive on a field — \
               `with = \"mod\"` already provides `arbitrary` via `mod::arbitrary`",
            ));
          }
          if out.shrink.is_some() {
            return Err(meta.error(
              "`with` and `shrink` are mutually exclusive on a field — \
               `with = \"mod\"` already provides `shrink` via `mod::shrink`",
            ));
          }
          if out.default {
            return Err(meta.error("`with` and `default` are mutually exclusive on a field"));
          }
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.with = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("arbitrary") {
          if out.with.is_some() {
            return Err(meta.error(
              "`arbitrary` and `with` are mutually exclusive on a field — \
               `with = \"mod\"` already provides `arbitrary` via `mod::arbitrary`",
            ));
          }
          if out.default {
            return Err(meta.error("`arbitrary` and `default` are mutually exclusive on a field"));
          }
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.arbitrary = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("shrink") {
          if out.with.is_some() {
            return Err(meta.error(
              "`shrink` and `with` are mutually exclusive on a field — \
               `with = \"mod\"` already provides `shrink` via `mod::shrink`",
            ));
          }
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.shrink = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("default") {
          if out.with.is_some() || out.arbitrary.is_some() {
            return Err(
              meta.error("`default` is mutually exclusive with `with` and `arbitrary` on a field"),
            );
          }
          out.default = true;
        } else {
          return Err(meta.error(
            "unknown field attribute; expected `with`, `arbitrary`, `shrink`, or `default`",
          ));
        }
        Ok(())
      })?;
    }
    Ok(out)
  }
}

/// Attributes accepted on an enum variant.
#[derive(Default)]
pub(crate) struct VariantAttrs {
  /// `skip` — exclude this variant from `arbitrary` selection.
  pub(crate) skip: bool,
  /// `with = "mod"` — a module exporting `arbitrary(g) -> Self` (yielding this
  /// variant) and `shrink(v: &Self) -> Box<dyn Iterator<Item = Self>>`.
  pub(crate) with: Option<Path>,
  /// `arbitrary = "fn"` — generate the whole `Self` value as this variant.
  pub(crate) arbitrary: Option<Path>,
  /// `shrink = "fn"` — shrink a value of this variant.
  pub(crate) shrink: Option<Path>,
}

impl VariantAttrs {
  pub(crate) fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
    let mut out = VariantAttrs::default();
    for attr in attrs {
      if !attr.path().is_ident("quickcheck") {
        continue;
      }
      attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("skip") {
          out.skip = true;
        } else if meta.path.is_ident("with") {
          if out.arbitrary.is_some() {
            return Err(meta.error("`with` and `arbitrary` are mutually exclusive on a variant"));
          }
          if out.shrink.is_some() {
            return Err(meta.error("`with` and `shrink` are mutually exclusive on a variant"));
          }
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.with = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("arbitrary") {
          if out.with.is_some() {
            return Err(meta.error("`arbitrary` and `with` are mutually exclusive on a variant"));
          }
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.arbitrary = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("shrink") {
          if out.with.is_some() {
            return Err(meta.error("`shrink` and `with` are mutually exclusive on a variant"));
          }
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.shrink = Some(parse_path(&lit)?);
        } else {
          return Err(meta.error(
            "unknown variant attribute; expected `skip`, `with`, `arbitrary`, or `shrink`",
          ));
        }
        Ok(())
      })?;
    }
    Ok(out)
  }

  // Note: combining `skip` with `with`/`arbitrary`/`shrink` is a no-op (the
  // variant is never generated) rather than an error — preserves the
  // previous semantics.
}
