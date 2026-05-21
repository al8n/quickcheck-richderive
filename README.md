<div align="center">
<h1>quickcheck-derive</h1>
</div>
<div align="center">

A `#[derive(Arbitrary)]` proc-macro that emits a **native**
[`quickcheck::Arbitrary`] implementation — both `arbitrary` **and** a real
`shrink` — for structs and enums.

</div>

Unlike bridges that route generation through `arbitrary::Unstructured` byte
buffers, this derive produces a genuine quickcheck impl that calls
`Arbitrary::arbitrary` / `Arbitrary::shrink` directly on the fields, so
`quickcheck`'s size control and shrinking work as intended.

This crate does **not** depend on `quickcheck` itself (only as a dev-dependency
for its own tests). The generated code refers to *your* quickcheck via the
[`crate`](#crate--pathtoquickcheck) attribute, defaulting to `::quickcheck`, so
consumers bring their own.

## Usage

```rust
use quickcheck_derive::Arbitrary;

#[derive(Clone, Debug, Arbitrary)]
struct Point {
    x: i32,
    y: i32,
}
```

The generated impl is wrapped in an anonymous `const _: () = { … };` for hygiene
and provides:

```rust,ignore
fn arbitrary(g: &mut Gen) -> Self;
fn shrink(&self) -> Box<dyn Iterator<Item = Self>>;
```

`arbitrary` builds each field with `quickcheck::Arbitrary::arbitrary`; `shrink`
shrinks **one field at a time**, holding the others at their current value, and
chains the resulting iterators. The derived type must be `Clone` (a
`quickcheck::Arbitrary` supertrait) — `shrink` clones `self` to hold the
unchanged fields.

## Attribute surface

All attributes live under the `quickcheck` path: `#[quickcheck(...)]`. They apply
at three positions — the **container** (the `struct`/`enum`), each **field**, and
each enum **variant**.

---

### Container attributes (on the `struct` / `enum`)

| Attribute | Meaning |
|-----------|---------|
| [`crate = "..."`](#crate--pathtoquickcheck) | Base path for the emitted `Arbitrary` / `Gen`. Default `::quickcheck`. |
| [`bound = "..."`](#bound--p-bound-q-other-repeatable) | **Repeatable.** Replaces the inferred generic bounds. |
| [`with = "..."`](#container-with--shrink) | Generate the whole value via a function. |
| [`shrink = "..."`](#container-with--shrink) | Shrink the whole value via a function. |
| [`box = "..."`](#box--pathtobox) | Override the `Box` type used in the `shrink` return. |

#### `crate = "path::to::quickcheck"`

Point the generated code at a re-exported or renamed `quickcheck`. Useful when
`quickcheck` is re-exported through another crate, or vendored under a different
name.

```rust
use quickcheck_derive::Arbitrary;

// `quickcheck` re-exported under a different path:
mod reexport {
    pub use quickcheck::*;
}

#[derive(Clone, Arbitrary)]
#[quickcheck(crate = "reexport")]
struct S {
    x: u8,
}
```

#### `bound = "P: Bound, Q: Other"` (repeatable)

By default the derive infers a **`<FieldTy>: quickcheck::Arbitrary` bound for each
generated field whose type mentions a generic parameter** (type *or* const) — e.g.
`T: Arbitrary` for a `T` field, `Vec<T>: Arbitrary`, `<T as Trait>::Item:
Arbitrary` for a projection, or `Only<N>: Arbitrary` for a const-generic field
type. Fields produced via `with` / `default`, and `skip` / variant-`with`
variants, contribute no bound (they are never generated via `Arbitrary`).
Bounding the field types — rather than the params inside them — keeps projected /
associated types sound. It additionally adds **`T: Clone + 'static` for every
generic type parameter**, which the `Arbitrary: Clone + 'static` supertrait on
`Self` requires (a `Vec<T>: Arbitrary` bound does **not** imply `T: Clone`, yet
the derived `Clone for Foo<T>` needs it). If you supply one or more `bound`
attributes, they **replace** that inference entirely: the generated `where`
clause becomes the type's own predicates **plus exactly** the predicates you list
(multiple `bound = "..."` accumulate).

```rust,ignore
// Default inference: `where T: quickcheck::Arbitrary`.
#[derive(Clone, Arbitrary)]
struct Wrapper<T>(T);

// Custom bounds replace the inference entirely.
#[derive(Clone, Arbitrary)]
#[quickcheck(bound = "T: Clone + Default + 'static")]
struct Defaulted<T> {
    #[quickcheck(default)]
    inner: T,
}
```

> **`'static` / `Clone` caveat.** Because `quickcheck::Arbitrary: Clone +
> 'static`, the impl always needs those for the type itself. When you override
> with `bound`, you are responsible for any bounds the body relies on — a generic
> param that is still generated via `Arbitrary::arbitrary` must keep
> `: quickcheck::Arbitrary`, and a param used in a `shrink`/`Clone` context must
> keep `: Clone + 'static`. The inference is dropped wholesale, so an incomplete
> custom bound will fail to compile.

#### Container `with` / `shrink`

Override generation and/or shrinking of the **whole** value with free functions:

| Attribute | Signature |
|-----------|-----------|
| `with = "f"` | `fn(g: &mut Gen) -> Self` |
| `shrink = "s"` | `fn(v: &Self) -> Box<dyn Iterator<Item = Self>>` |

The two are independent: `with` without `shrink` ⇒ `shrink` is empty; `shrink`
without `with` ⇒ generation is still field/variant-derived.

This is the idiomatic way to support **types with invariants** — generate only
valid values by routing through a checked constructor, instead of field-by-field:

```rust,ignore
use quickcheck::Gen;

#[derive(Clone, Arbitrary)]
#[quickcheck(with = "gen_geo")]
struct GeoLocation { /* private, range-checked fields */ }

fn gen_geo(g: &mut Gen) -> GeoLocation {
    // produce in-range inputs, then go through the validating constructor
    let lat = (i64::arbitrary(g) % 9_001) as f64 / 100.0; // [-90, 90]
    let lon = (i64::arbitrary(g) % 18_001) as f64 / 100.0; // [-180, 180]
    GeoLocation::try_new(lat, lon, None).unwrap()
}
```

#### `box = "path::to::Box"`

Override the `Box` type in the generated `shrink` return
(`shrink(&self) -> Box<dyn Iterator<Item = Self>>`). By default it is
`::std::boxed::Box` with the `std` feature, or `::alloc::boxed::Box` in no-std
(see [Features](#features)); `box` overrides either, e.g. to point at a
re-exported / custom `Box`:

```rust,ignore
#[derive(Clone, Arbitrary)]
#[quickcheck(box = "my_crate::reexport::Box")]
struct S { x: u32 }
```

---

### Field attributes (struct fields, and fields of struct/tuple variants)

| Attribute | Meaning |
|-----------|---------|
| `with = "f"` | Generate this field via `fn(g: &mut Gen) -> FieldT`. |
| `shrink = "s"` | Shrink this field via `fn(v: &FieldT) -> Box<dyn Iterator<Item = FieldT>>`. |
| `default` | Generate via `Default::default()`; the field is **held constant** when shrinking. |

`with` and `default` are **mutually exclusive** on the same field (compile
error). The per-field shrink rule:

- `shrink = "s"` → use `s`;
- plain field → `quickcheck::Arbitrary::shrink`;
- `with`-without-`shrink`, or `default` → **held constant** (never shrunk).

```rust,ignore
use quickcheck::Gen;

#[derive(Clone, Arbitrary)]
struct Packet {
    // custom generator + custom shrinker for a foreign / unsupported type
    #[quickcheck(with = "gen_id", shrink = "shrink_id")]
    id: ForeignId,

    // never generated from `g`; always `Default::default()`, never shrunk
    #[quickcheck(default)]
    cached: Cache,

    // plain: Arbitrary::arbitrary / Arbitrary::shrink
    payload: Vec<u8>,
}

fn gen_id(g: &mut Gen) -> ForeignId { ForeignId::new(u32::arbitrary(g)) }
fn shrink_id(v: &ForeignId) -> Box<dyn Iterator<Item = ForeignId>> {
    Box::new(v.as_u32().shrink().map(ForeignId::new))
}
```

---

### Variant attributes (enum variants)

| Attribute | Meaning |
|-----------|---------|
| `skip` | Exclude from `arbitrary` selection. A value that *is* this variant shrinks to empty. If **every** variant is `skip`, a `compile_error!` is produced. |
| `with = "f"` | Generate the whole `Self` value as this variant: `fn(g: &mut Gen) -> Self`. **Takes precedence** over the variant's field attributes. |
| `shrink = "s"` | Shrink a value of this variant: `fn(v: &Self) -> Box<dyn Iterator<Item = Self>>`. `with`-without-`shrink` ⇒ empty for that variant. |

`arbitrary` picks **uniformly** among the non-skipped variants via `g.choose`.
Variants without a variant-level `with` are generated field-by-field, and their
fields accept the field attributes above (named and tuple variants alike).

```rust,ignore
use quickcheck::Gen;

#[derive(Clone, Arbitrary)]
enum Event {
    // never produced by `arbitrary`
    #[quickcheck(skip)]
    Internal,

    // unit + field-derived variants
    Tick,
    Resize { width: u32, height: u32 },

    // whole-variant override
    #[quickcheck(with = "gen_custom", shrink = "shrink_custom")]
    Custom(Payload),

    // per-field attributes inside a variant
    Frame {
        #[quickcheck(with = "gen_pixels")]
        pixels: Pixels,
        index: u64,
    },
}

fn gen_custom(g: &mut Gen) -> Event { Event::Custom(Payload::arbitrary(g)) }
fn shrink_custom(v: &Event) -> Box<dyn Iterator<Item = Event>> { Box::new(std::iter::empty()) }
fn gen_pixels(g: &mut Gen) -> Pixels { Pixels::arbitrary(g) }
```

## Codegen summary

- **Output** — wrapped in `const _: () = { impl … { fn arbitrary; fn shrink } };`;
  `Arbitrary` / `Gen` are referenced through the `crate` path.
- **`where` clause** — `split_for_impl` + either explicit `bound`s or an inferred
  `<FieldTy>: <crate>::Arbitrary` per generated field type that mentions a generic
  (type or const) param.
- **Struct** — `arbitrary` builds the struct literal (each field per its rule);
  `shrink` clones `self`, assigns one shrunk field at a time, and chains.
- **Enum** — `arbitrary` does `match *g.choose(&[non-skipped indices]).unwrap()`;
  `shrink` matches the current variant and **rebuilds it explicitly** with one
  field shrunk and the rest cloned. Unit variants, `skip`ped variants, and
  fully held-constant variants shrink to empty.

## Compile-time errors

The derive reports these as `compile_error!` with a focused span (covered by the
`tests/ui` suite):

- a `union` target (`derive(Arbitrary)` supports only structs and enums);
- an unknown key in a container / field / variant `#[quickcheck(...)]`;
- `default` together with `with` on the same field;
- an `enum` whose every variant is `#[quickcheck(skip)]`.

## Features

This is a proc-macro crate; its features select what the generated `shrink`
returns:

- **`std`** (default) — `shrink` returns `::std::boxed::Box<dyn Iterator<…>>`.
- **`alloc`** — for no-std consumers: `shrink` returns
  `::alloc::boxed::Box<dyn Iterator<…>>` instead. Enable with
  `default-features = false, features = ["alloc"]`.

A container `#[quickcheck(box = "...")]` overrides the `Box` path regardless of
feature. (Generation otherwise uses only `core` paths — `core::iter`,
`core::clone::Clone`, `core::default::Default` — so the output is no-std-ready.)

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

[`quickcheck::Arbitrary`]: https://docs.rs/quickcheck/latest/quickcheck/trait.Arbitrary.html
