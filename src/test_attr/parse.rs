//! Parser for `#[quickcheck_richderive::test(...)]` attribute arguments.
//!
//! The attribute accepts a comma-separated list of `key = value` pairs. Four
//! keys are *reserved* for runner configuration; every other identifier is
//! interpreted as a **per-argument generator override** whose name must match
//! a parameter in the annotated function's signature.
//!
//! | Key | Type | Notes |
//! |---|---|---|
//! | `cases = N`              | `u64` literal   | `.tests(N)` on the runner |
//! | `max_tests = N`          | `u64` literal   | `.max_tests(N)` (discard cap) |
//! | `gen_size = N`           | `usize` literal | `Gen::new(N)` |
//! | `min_tests_passed = N`   | `u64` literal   | omitted if unset |
//! | `<arg_ident> = "path"`   | path string     | `fn(&mut Gen) -> ArgType` |
//!
//! Validation against the user's fn signature (i.e. checking each override
//! identifier actually names a parameter) happens later in the expansion stage
//! — `parse` only enforces shape.

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::{
  Error, Ident, LitInt, LitStr, Path, Token, parse::Parse, parse::ParseStream, parse_str,
  punctuated::Punctuated,
};

/// A parsed per-argument generator override: `<arg_ident> = "path::to::fn"`.
pub(crate) struct ArgOverride {
  /// The argument identifier (must match a parameter name on the user fn).
  pub(crate) arg: Ident,
  /// The path to the generator function. Signature: `fn(&mut Gen) -> ArgType`.
  pub(crate) path: Path,
}

/// Fully-parsed attribute arguments, post-shape-check.
///
/// Field-validation responsibilities split between this struct and the
/// expansion stage:
///
/// * **Here:** literal kinds and integer parsing (`cases = "100"` is a shape
///   error caught here); duplicate keys (caught here); unknown reserved-shaped
///   keys are NOT rejected here — any non-reserved ident is admitted as a
///   per-arg override.
/// * **Expansion:** matching each `ArgOverride.arg` to a real fn parameter,
///   rejecting overrides naming nonexistent params.
pub(crate) struct TestAttrArgs {
  /// `cases = N` — `.tests(N)`. Default `100`.
  pub(crate) cases: u64,
  /// `max_tests = N` — `.max_tests(N)`. Default `10_000`.
  pub(crate) max_tests: u64,
  /// `gen_size = N` — `Gen::new(N)`. Default `100`.
  pub(crate) gen_size: usize,
  /// `min_tests_passed = N` — `.min_tests_passed(N)`. `None` ⇒ omit the call.
  pub(crate) min_tests_passed: Option<u64>,
  /// Per-arg generator overrides. Order preserved as written.
  pub(crate) arg_overrides: Vec<ArgOverride>,
}

impl Default for TestAttrArgs {
  fn default() -> Self {
    // Defaults match `quickcheck::QuickCheck::new()` itself, so the bare-form
    // expansion behaves like `#[quickcheck_macros::quickcheck]`.
    Self {
      cases: 100,
      max_tests: 10_000,
      gen_size: 100,
      min_tests_passed: None,
      arg_overrides: Vec::new(),
    }
  }
}

impl TestAttrArgs {
  /// Parse the attribute's argument tokens. An empty token-stream returns
  /// defaults — the bare `#[quickcheck_richderive::test]` form.
  pub(crate) fn parse(tokens: TokenStream2) -> syn::Result<Self> {
    if tokens.is_empty() {
      return Ok(Self::default());
    }
    syn::parse2::<Self>(tokens)
  }
}

impl Parse for TestAttrArgs {
  fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
    let mut out = TestAttrArgs::default();
    // Track which keys we've already seen, by ident name. A duplicate of any
    // key — reserved or per-arg — is an error at the *second* occurrence's
    // span (the first is the authoritative value, conceptually).
    let mut seen: HashSet<String> = HashSet::new();

    // Parse `key = value, key = value, ...` until exhaustion. A trailing
    // comma is allowed.
    let entries: Punctuated<KvEntry, Token![,]> = Punctuated::parse_terminated(input)?;

    for entry in entries {
      let key_str = entry.key.to_string();
      if !seen.insert(key_str.clone()) {
        return Err(Error::new(
          entry.key.span(),
          format!("duplicate key `{key_str}` in #[quickcheck_richderive::test(...)]"),
        ));
      }

      match key_str.as_str() {
        "cases" => out.cases = parse_u64(&entry, "cases")?,
        "max_tests" => out.max_tests = parse_u64(&entry, "max_tests")?,
        "gen_size" => out.gen_size = parse_usize(&entry, "gen_size")?,
        "min_tests_passed" => {
          out.min_tests_passed = Some(parse_u64(&entry, "min_tests_passed")?);
        }
        // Common typo for a reserved key — fail loudly rather than silently
        // accepting it as a per-arg override and confusing the user.
        "case" | "test" | "tests" | "size" | "seed" => {
          return Err(Error::new(
            entry.key.span(),
            format!(
              "unknown key `{key_str}` in #[quickcheck_richderive::test(...)]; \
               did you mean one of `cases`, `max_tests`, `gen_size`, `min_tests_passed`? \
               (note: deterministic seeding via `seed` is not supported by upstream \
               `quickcheck`; see the README's reference table)"
            ),
          ));
        }
        // Otherwise: per-arg generator override. Value must be a string
        // literal carrying a syntactically valid `syn::Path`.
        _ => {
          let path = expect_path_string(&entry, &key_str)?;
          out.arg_overrides.push(ArgOverride {
            arg: entry.key,
            path,
          });
        }
      }
    }

    Ok(out)
  }
}

/// One `key = value` pair, value held as raw tokens for the dispatcher to
/// re-interpret based on the key's shape. The `=` token isn't retained; the
/// value's own span is enough for shape-mismatch diagnostics.
struct KvEntry {
  key: Ident,
  value: KvValue,
}

impl Parse for KvEntry {
  fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
    let key: Ident = input.parse()?;
    let _eq: Token![=] = input.parse()?;
    let value: KvValue = input.parse()?;
    Ok(Self { key, value })
  }
}

/// The right-hand side of a `key = value` entry. We accept exactly two value
/// shapes: integer literals (for runner-config keys) and string literals (for
/// per-arg override paths). Mismatches are reported at the value's span by
/// `parse_u64` / `parse_usize` / `expect_path_string`.
enum KvValue {
  Int(LitInt),
  Str(LitStr),
}

impl KvValue {
  fn span(&self) -> Span {
    match self {
      Self::Int(lit) => lit.span(),
      Self::Str(lit) => lit.span(),
    }
  }
}

impl Parse for KvValue {
  fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
    let lookahead = input.lookahead1();
    if lookahead.peek(LitInt) {
      Ok(Self::Int(input.parse()?))
    } else if lookahead.peek(LitStr) {
      Ok(Self::Str(input.parse()?))
    } else {
      Err(lookahead.error())
    }
  }
}

fn parse_u64(entry: &KvEntry, key: &str) -> syn::Result<u64> {
  match &entry.value {
    KvValue::Int(lit) => lit.base10_parse::<u64>().map_err(|e| {
      Error::new(
        lit.span(),
        format!("`{key}` must fit in `u64`: {e}"),
      )
    }),
    KvValue::Str(_) => Err(Error::new(
      entry.value.span(),
      format!("`{key}` expects an integer literal, not a string"),
    )),
  }
}

fn parse_usize(entry: &KvEntry, key: &str) -> syn::Result<usize> {
  match &entry.value {
    KvValue::Int(lit) => lit.base10_parse::<usize>().map_err(|e| {
      Error::new(
        lit.span(),
        format!("`{key}` must fit in `usize`: {e}"),
      )
    }),
    KvValue::Str(_) => Err(Error::new(
      entry.value.span(),
      format!("`{key}` expects an integer literal, not a string"),
    )),
  }
}

/// Per-arg overrides expect a *string literal* carrying a `syn::Path` —
/// matching the existing `#[quickcheck(arbitrary = "path")]` convention on the
/// derive. An integer literal here is a shape mismatch.
fn expect_path_string(entry: &KvEntry, key: &str) -> syn::Result<Path> {
  let lit = match &entry.value {
    KvValue::Str(lit) => lit,
    KvValue::Int(_) => {
      // Most likely cause: the user mistyped a reserved-config key (e.g.
      // `cass` instead of `cases`), so admit-as-override then this branch
      // catches the integer literal at the value-shape level. Hint
      // accordingly.
      return Err(Error::new(
        entry.value.span(),
        format!(
          "`{key}` is interpreted as a per-arg generator override, which expects a \
           string literal path (e.g. `{key} = \"my::gen\"`); \
           if you meant a runner-config key, the supported ones are \
           `cases`, `max_tests`, `gen_size`, and `min_tests_passed`"
        ),
      ));
    }
  };
  parse_str::<Path>(&lit.value()).map_err(|e| Error::new(lit.span(), e))
}

