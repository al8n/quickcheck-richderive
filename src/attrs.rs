//! Parsing of `#[quickcheck(...)]` attributes for the container, fields, and
//! enum variants.

use syn::{
  parse_str, punctuated::Punctuated, spanned::Spanned, Attribute, Error, Path, Token,
  WherePredicate,
};

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
  /// `with = "fn"` — generate the entire value via this function.
  pub(crate) with: Option<Path>,
  /// `shrink = "fn"` — shrink the entire value via this function.
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
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.with = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("shrink") {
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.shrink = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("box") {
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.box_path = Some(parse_path(&lit)?);
        } else {
          return Err(meta.error(
            "unknown container attribute; expected `crate`, `bound`, `with`, `shrink`, or `box`",
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
  /// `with = "fn"` — generate this field via this function.
  pub(crate) with: Option<Path>,
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
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.with = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("shrink") {
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.shrink = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("default") {
          out.default = true;
        } else {
          return Err(
            meta.error("unknown field attribute; expected `with`, `shrink`, or `default`"),
          );
        }
        Ok(())
      })?;
    }
    if out.default && out.with.is_some() {
      return Err(Error::new(
        attrs
          .iter()
          .find(|a| a.path().is_ident("quickcheck"))
          .map(|a| a.span())
          .unwrap_or_else(proc_macro2::Span::call_site),
        "`default` and `with` are mutually exclusive on a field",
      ));
    }
    Ok(out)
  }
}

/// Attributes accepted on an enum variant.
#[derive(Default)]
pub(crate) struct VariantAttrs {
  /// `skip` — exclude this variant from `arbitrary` selection.
  pub(crate) skip: bool,
  /// `with = "fn"` — generate the whole `Self` value as this variant.
  pub(crate) with: Option<Path>,
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
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.with = Some(parse_path(&lit)?);
        } else if meta.path.is_ident("shrink") {
          let lit: syn::LitStr = meta.value()?.parse()?;
          out.shrink = Some(parse_path(&lit)?);
        } else {
          return Err(
            meta.error("unknown variant attribute; expected `skip`, `with`, or `shrink`"),
          );
        }
        Ok(())
      })?;
    }
    Ok(out)
  }
}
