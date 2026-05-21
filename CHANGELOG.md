# Changelog

## Unreleased

## 0.1.0

- Initial release.
- `#[derive(quickcheck_derive::Arbitrary)]` emitting a native
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
