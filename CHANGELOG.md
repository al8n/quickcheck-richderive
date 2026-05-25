# Changelog

## 0.3.0 — `#[test]` attribute macro

### Added

- **`#[quickcheck_richderive::test]`** — a proptest-style proc-macro-attribute
  for property tests, drop-in for `#[quickcheck_macros::quickcheck]`:
  - **Per-arg generator overrides** — `<arg_ident> = "path::to::fn"` swaps an
    individual fn argument's `Arbitrary` impl for a user-supplied
    `fn(&mut Gen) -> ArgT`. Mirrors the derive's existing
    `#[quickcheck(arbitrary = "path")]` field attribute.
  - **Per-test runner config** — `cases`, `max_tests`, `gen_size`, and
    `min_tests_passed` at the call site (no need to hand-roll a
    `QuickCheck::new()…` chain).
  - Bare form (`#[quickcheck_richderive::test]` with no args) is
    behaviour-identical to vanilla `#[quickcheck]`.
  - Return type acceptance set matches `quickcheck::Testable`: `bool`, `()`,
    `TestResult`, and `Result<T: Testable, E: Debug>` all pass through
    unchanged.
  - Per-arg overrides preserve shrinking by wrapping each overridden arg in a
    private newtype that delegates `shrink` to the underlying type's
    `Arbitrary::shrink` — no `Shrink` knob is exposed on the attribute
    surface.
  - **No `seed` key.** `quickcheck::Gen` has no public seed API; deterministic
    sequences require a custom generator backed by an RNG you control. See
    the README's `#[test]` attribute section for details.

## 0.2.0 — serde-style attribute surface

### Breaking

- **`with = "..."` semantics changed: it now expects a MODULE, not a single
  function.** The module must export both `arbitrary(g: &mut Gen) -> T` and
  `shrink(v: &T) -> Box<dyn Iterator<Item = T>>` — mirroring serde's
  `#[serde(with = "module")]`. The pair-mod form applies at all three positions
  (container, field, variant).
- The previous "single function for gen" form lives on under the new
  **`arbitrary = "fn"`** attribute (see below).
- New mutual-exclusion rules:
  - `with` + `arbitrary` on the same item → compile error.
  - `with` + `shrink` on the same item → compile error.
  - `default` + `arbitrary` on a field → compile error (in addition to the
    existing `default` + `with` mutex).

### Added

- **`arbitrary = "fn"`** — single-function override for the gen half only,
  available at container / field / variant levels. Signatures:
  - container/variant: `fn(g: &mut Gen) -> Self`
  - field: `fn(g: &mut Gen) -> FieldT`
- The three knobs `with` / `arbitrary` / `shrink` mirror serde's
  `with` / `deserialize_with` / `serialize_with` triad: `with` bundles both
  halves through a module, `arbitrary` and `shrink` are the per-direction
  overrides.

### Migration

Each previous `#[quickcheck(with = "fn")]` (single-fn) becomes
`#[quickcheck(arbitrary = "fn")]`. Reach for the new module form only when you
also want a custom `shrink`:

```rust
// Before (0.1.x):
#[quickcheck(with = "gen_geo")]

// After (0.2.x) — equivalent gen-half only:
#[quickcheck(arbitrary = "gen_geo")]

// After (0.2.x) — gen + shrink together via a module:
#[quickcheck(with = "geo_helpers")]
// where geo_helpers exports:
//   pub fn arbitrary(g: &mut Gen) -> GeoLocation { ... }
//   pub fn shrink(v: &GeoLocation) -> Box<dyn Iterator<Item = GeoLocation>> { ... }
```

## 0.1.0

- Initial release.
- `#[derive(quickcheck_richderive::Arbitrary)]` emitting a native
  `quickcheck::Arbitrary` impl (`arbitrary` + `shrink`) for structs and enums.
- Container attributes: `crate`, `bound` (repeatable), `with`, `shrink`, `box`.
- Field attributes: `with`, `shrink`, `default`.
- Variant attributes: `skip`, `with`, `shrink`.
- `std` (default) / `alloc` features select the generated `shrink` `Box` type
  (`::std::boxed::Box` / `::alloc::boxed::Box`); `box = "..."` overrides it.
  Generated code otherwise uses only `core` paths (`core::iter::Iterator`,
  `core::clone::Clone`, …), so the output is no-std-ready.
- Bound inference: per generated field type (`<FieldTy>: Arbitrary`, sound for
  projections / nested generics) plus `T: Clone + 'static` per type param for the
  `Arbitrary` supertrait; const-generic field types included. Internal idents are
  collision-free against user `const` params.
