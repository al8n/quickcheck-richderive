# Changelog

## Unreleased

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
