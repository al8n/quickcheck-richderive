<div align="center">
<h1>quickcheck-richderive</h1>
</div>
<div align="center">

A `#[derive(Arbitrary)]` proc-macro that emits a **native**
[`quickcheck::Arbitrary`] implementation — both `arbitrary` **and** a real
`shrink` — for structs and enums.

[<img alt="github" src="https://img.shields.io/badge/github-al8n/quickcheck--richderive-8da0cb?style=for-the-badge&logo=Github" height="22">][Github-url]
<img alt="LoC" src="https://img.shields.io/endpoint?url=https%3A%2F%2Fgist.githubusercontent.com%2Fal8n%2F327b2a8aef9003246e45c6e47fe63937%2Fraw%2Fquickcheck-richderive" height="22">
[<img alt="Build" src="https://img.shields.io/github/actions/workflow/status/al8n/quickcheck-richderive/ci.yml?logo=Github-Actions&style=for-the-badge" height="22">][CI-url]


[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-quickcheck--richderive-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">][doc-url]
[<img alt="crates.io" src="https://img.shields.io/crates/v/quickcheck-richderive?style=for-the-badge&logo=data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iaXNvLTg4NTktMSI/Pg0KPCEtLSBHZW5lcmF0b3I6IEFkb2JlIElsbHVzdHJhdG9yIDE5LjAuMCwgU1ZHIEV4cG9ydCBQbHVnLUluIC4gU1ZHIFZlcnNpb246IDYuMDAgQnVpbGQgMCkgIC0tPg0KPHN2ZyB2ZXJzaW9uPSIxLjEiIGlkPSJMYXllcl8xIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHhtbG5zOnhsaW5rPSJodHRwOi8vd3d3LnczLm9yZy8xOTk5L3hsaW5rIiB4PSIwcHgiIHk9IjBweCINCgkgdmlld0JveD0iMCAwIDUxMiA1MTIiIHhtbDpzcGFjZT0icHJlc2VydmUiPg0KPGc+DQoJPGc+DQoJCTxwYXRoIGQ9Ik0yNTYsMEwzMS41MjgsMTEyLjIzNnYyODcuNTI4TDI1Niw1MTJsMjI0LjQ3Mi0xMTIuMjM2VjExMi4yMzZMMjU2LDB6IE0yMzQuMjc3LDQ1Mi41NjRMNzQuOTc0LDM3Mi45MTNWMTYwLjgxDQoJCQlsMTU5LjMwMyw3OS42NTFWNDUyLjU2NHogTTEwMS44MjYsMTI1LjY2MkwyNTYsNDguNTc2bDE1NC4xNzQsNzcuMDg3TDI1NiwyMDIuNzQ5TDEwMS44MjYsMTI1LjY2MnogTTQzNy4wMjYsMzcyLjkxMw0KCQkJbC0xNTkuMzAzLDc5LjY1MVYyNDAuNDYxbDE1OS4zMDMtNzkuNjUxVjM3Mi45MTN6IiBmaWxsPSIjRkZGIi8+DQoJPC9nPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPC9zdmc+DQo=" height="22">][crates-url]
[<img alt="crates.io" src="https://img.shields.io/crates/d/quickcheck-richderive?color=critical&logo=data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBzdGFuZGFsb25lPSJubyI/PjwhRE9DVFlQRSBzdmcgUFVCTElDICItLy9XM0MvL0RURCBTVkcgMS4xLy9FTiIgImh0dHA6Ly93d3cudzMub3JnL0dyYXBoaWNzL1NWRy8xLjEvRFREL3N2ZzExLmR0ZCI+PHN2ZyB0PSIxNjQ1MTE3MzMyOTU5IiBjbGFzcz0iaWNvbiIgdmlld0JveD0iMCAwIDEwMjQgMTAyNCIgdmVyc2lvbj0iMS4xIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHAtaWQ9IjM0MjEiIGRhdGEtc3BtLWFuY2hvci1pZD0iYTMxM3guNzc4MTA2OS4wLmkzIiB3aWR0aD0iNDgiIGhlaWdodD0iNDgiIHhtbG5zOnhsaW5rPSJodHRwOi8vd3d3LnczLm9yZy8xOTk5L3hsaW5rIj48ZGVmcz48c3R5bGUgdHlwZT0idGV4dC9jc3MiPjwvc3R5bGU+PC9kZWZzPjxwYXRoIGQ9Ik00NjkuMzEyIDU3MC4yNHYtMjU2aDg1LjM3NnYyNTZoMTI4TDUxMiA3NTYuMjg4IDM0MS4zMTIgNTcwLjI0aDEyOHpNMTAyNCA2NDAuMTI4QzEwMjQgNzgyLjkxMiA5MTkuODcyIDg5NiA3ODcuNjQ4IDg5NmgtNTEyQzEyMy45MDQgODk2IDAgNzYxLjYgMCA1OTcuNTA0IDAgNDUxLjk2OCA5NC42NTYgMzMxLjUyIDIyNi40MzIgMzAyLjk3NiAyODQuMTYgMTk1LjQ1NiAzOTEuODA4IDEyOCA1MTIgMTI4YzE1Mi4zMiAwIDI4Mi4xMTIgMTA4LjQxNiAzMjMuMzkyIDI2MS4xMkM5NDEuODg4IDQxMy40NCAxMDI0IDUxOS4wNCAxMDI0IDY0MC4xOTJ6IG0tMjU5LjItMjA1LjMxMmMtMjQuNDQ4LTEyOS4wMjQtMTI4Ljg5Ni0yMjIuNzItMjUyLjgtMjIyLjcyLTk3LjI4IDAtMTgzLjA0IDU3LjM0NC0yMjQuNjQgMTQ3LjQ1NmwtOS4yOCAyMC4yMjQtMjAuOTI4IDIuOTQ0Yy0xMDMuMzYgMTQuNC0xNzguMzY4IDEwNC4zMi0xNzguMzY4IDIxNC43MiAwIDExNy45NTIgODguODMyIDIxNC40IDE5Ni45MjggMjE0LjRoNTEyYzg4LjMyIDAgMTU3LjUwNC03NS4xMzYgMTU3LjUwNC0xNzEuNzEyIDAtODguMDY0LTY1LjkyLTE2NC45MjgtMTQ0Ljk2LTE3MS43NzZsLTI5LjUwNC0yLjU2LTUuODg4LTMwLjk3NnoiIGZpbGw9IiNmZmZmZmYiIHAtaWQ9IjM0MjIiIGRhdGEtc3BtLWFuY2hvci1pZD0iYTMxM3guNzc4MTA2OS4wLmkwIiBjbGFzcz0iIj48L3BhdGg+PC9zdmc+&style=for-the-badge" height="22">][crates-url]
<img alt="license" src="https://img.shields.io/badge/License-Apache%202.0/MIT-blue.svg?style=for-the-badge&fontColor=white&logoColor=f5c076&logo=data:image/svg+xml;base64,PCFET0NUWVBFIHN2ZyBQVUJMSUMgIi0vL1czQy8vRFREIFNWRyAxLjEvL0VOIiAiaHR0cDovL3d3dy53My5vcmcvR3JhcGhpY3MvU1ZHLzEuMS9EVEQvc3ZnMTEuZHRkIj4KDTwhLS0gVXBsb2FkZWQgdG86IFNWRyBSZXBvLCB3d3cuc3ZncmVwby5jb20sIFRyYW5zZm9ybWVkIGJ5OiBTVkcgUmVwbyBNaXhlciBUb29scyAtLT4KPHN2ZyBmaWxsPSIjZmZmZmZmIiBoZWlnaHQ9IjgwMHB4IiB3aWR0aD0iODAwcHgiIHZlcnNpb249IjEuMSIgaWQ9IkNhcGFfMSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB4bWxuczp4bGluaz0iaHR0cDovL3d3dy53My5vcmcvMTk5OS94bGluayIgdmlld0JveD0iMCAwIDI3Ni43MTUgMjc2LjcxNSIgeG1sOnNwYWNlPSJwcmVzZXJ2ZSIgc3Ryb2tlPSIjZmZmZmZmIj4KDTxnIGlkPSJTVkdSZXBvX2JnQ2FycmllciIgc3Ryb2tlLXdpZHRoPSIwIi8+Cg08ZyBpZD0iU1ZHUmVwb190cmFjZXJDYXJyaWVyIiBzdHJva2UtbGluZWNhcD0icm91bmQiIHN0cm9rZS1saW5lam9pbj0icm91bmQiLz4KDTxnIGlkPSJTVkdSZXBvX2ljb25DYXJyaWVyIj4gPGc+IDxwYXRoIGQ9Ik0xMzguMzU3LDBDNjIuMDY2LDAsMCw2Mi4wNjYsMCwxMzguMzU3czYyLjA2NiwxMzguMzU3LDEzOC4zNTcsMTM4LjM1N3MxMzguMzU3LTYyLjA2NiwxMzguMzU3LTEzOC4zNTcgUzIxNC42NDgsMCwxMzguMzU3LDB6IE0xMzguMzU3LDI1OC43MTVDNzEuOTkyLDI1OC43MTUsMTgsMjA0LjcyMywxOCwxMzguMzU3UzcxLjk5MiwxOCwxMzguMzU3LDE4IHMxMjAuMzU3LDUzLjk5MiwxMjAuMzU3LDEyMC4zNTdTMjA0LjcyMywyNTguNzE1LDEzOC4zNTcsMjU4LjcxNXoiLz4gPHBhdGggZD0iTTE5NC43OTgsMTYwLjkwM2MtNC4xODgtMi42NzctOS43NTMtMS40NTQtMTIuNDMyLDIuNzMyYy04LjY5NCwxMy41OTMtMjMuNTAzLDIxLjcwOC0zOS42MTQsMjEuNzA4IGMtMjUuOTA4LDAtNDYuOTg1LTIxLjA3OC00Ni45ODUtNDYuOTg2czIxLjA3Ny00Ni45ODYsNDYuOTg1LTQ2Ljk4NmMxNS42MzMsMCwzMC4yLDcuNzQ3LDM4Ljk2OCwyMC43MjMgYzIuNzgyLDQuMTE3LDguMzc1LDUuMjAxLDEyLjQ5NiwyLjQxOGM0LjExOC0yLjc4Miw1LjIwMS04LjM3NywyLjQxOC0xMi40OTZjLTEyLjExOC0xNy45MzctMzIuMjYyLTI4LjY0NS01My44ODItMjguNjQ1IGMtMzUuODMzLDAtNjQuOTg1LDI5LjE1Mi02NC45ODUsNjQuOTg2czI5LjE1Miw2NC45ODYsNjQuOTg1LDY0Ljk4NmMyMi4yODEsMCw0Mi43NTktMTEuMjE4LDU0Ljc3OC0zMC4wMDkgQzIwMC4yMDgsMTY5LjE0NywxOTguOTg1LDE2My41ODIsMTk0Ljc5OCwxNjAuOTAzeiIvPiA8L2c+IDwvZz4KDTwvc3ZnPg==" height="22">

</div>

## Introduction

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
use quickcheck_richderive::Arbitrary;

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
| [`with = "mod"`](#container-with--arbitrary--shrink) | A **module** exporting both `mod::arbitrary` and `mod::shrink` — overrides both halves at once. Serde-style. |
| [`arbitrary = "fn"`](#container-with--arbitrary--shrink) | Generate the whole value via this function. |
| [`shrink = "fn"`](#container-with--arbitrary--shrink) | Shrink the whole value via this function. |
| [`box = "..."`](#box--pathtobox) | Override the `Box` type used in the `shrink` return. |

#### `crate = "path::to::quickcheck"`

Point the generated code at a re-exported or renamed `quickcheck`. Useful when
`quickcheck` is re-exported through another crate, or vendored under a different
name.

```rust
use quickcheck_richderive::Arbitrary;

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
associated types sound. It additionally adds a single **`Self: Clone + 'static`**
bound — the exact `Arbitrary: Clone + 'static` supertrait obligation on the
implementing type, which a `<FieldTy>: Arbitrary` bound (e.g. `Vec<T>: Arbitrary`)
does **not** imply. Bounding `Self` rather than each `T` avoids over-constraining
manually-`Clone` types and correctly handles lifetime-generic targets. If you
supply one or more `bound` attributes, they **replace** that inference entirely:
the generated `where` clause becomes the type's own predicates **plus exactly**
the predicates you list (multiple `bound = "..."` accumulate).

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

#### Container `with` / `arbitrary` / `shrink`

Override generation and/or shrinking of the **whole** value. Three knobs,
mirroring serde's `serialize_with` / `deserialize_with` / `with` triad:

| Attribute | Value | Signature(s) the consumer must export |
|-----------|-------|---------------------------------------|
| `with = "mod"` | a **module** | `fn arbitrary(g: &mut Gen) -> Self` **and** `fn shrink(v: &Self) -> Box<dyn Iterator<Item = Self>>` |
| `arbitrary = "fn"` | a **function** | `fn(g: &mut Gen) -> Self` |
| `shrink = "fn"` | a **function** | `fn(v: &Self) -> Box<dyn Iterator<Item = Self>>` |

`with` bundles both halves through one module; `arbitrary` and `shrink` are the
per-direction overrides. `with` is **mutually exclusive** with both `arbitrary`
and `shrink` (compile error). Defaults that kick in when an attribute is absent:

- `with = "mod"` alone: both halves come from the module.
- `arbitrary = "fn"` alone: gen uses `fn`; **shrink is empty** (no shrink route).
- `shrink = "fn"` alone: gen is still field/variant-derived; shrink uses `fn`.
- `arbitrary = "fn"` + `shrink = "fn"`: each direction from its own function.

##### `with = "mod"` — module pair (serde-style)

```rust,ignore
#[derive(Clone, Arbitrary)]
#[quickcheck(with = "geo_helpers")]
struct GeoLocation { /* private, range-checked fields */ }

mod geo_helpers {
    use super::GeoLocation;
    use quickcheck::Gen;

    pub fn arbitrary(g: &mut Gen) -> GeoLocation {
        let lat = (i64::arbitrary(g) % 9_001) as f64 / 100.0;   // [-90, 90]
        let lon = (i64::arbitrary(g) % 18_001) as f64 / 100.0;  // [-180, 180]
        GeoLocation::try_new(lat, lon, None).unwrap()
    }

    pub fn shrink(v: &GeoLocation) -> Box<dyn Iterator<Item = GeoLocation>> {
        // …whatever shrink strategy makes sense for the validated invariants
        Box::new(std::iter::empty())
    }
}
```

##### `arbitrary = "fn"` — single-fn gen, no shrink

When the type has no useful shrink and you only need to override generation:

```rust,ignore
#[derive(Clone, Arbitrary)]
#[quickcheck(arbitrary = "gen_geo")]
struct GeoLocation { /* private, range-checked fields */ }

fn gen_geo(g: &mut Gen) -> GeoLocation {
    let lat = (i64::arbitrary(g) % 9_001) as f64 / 100.0;
    let lon = (i64::arbitrary(g) % 18_001) as f64 / 100.0;
    GeoLocation::try_new(lat, lon, None).unwrap()
}
```

##### `shrink = "fn"` — single-fn shrink, field-derived gen

Useful when the field-by-field default gen is fine but you want a smarter
shrink strategy.

#### `box = "path::to::Box"`

Override the `Box` type in the generated `shrink` return
(`shrink(&self) -> Box<dyn Iterator<Item = Self>>`). By default it is
`::std::boxed::Box` with the `std` feature, or an internally-aliased
`alloc::boxed::Box` in no-std (see [Features](#features)); `box` overrides either,
e.g. to point at a re-exported / custom `Box`:

```rust,ignore
#[derive(Clone, Arbitrary)]
#[quickcheck(box = "my_crate::reexport::Box")]
struct S { x: u32 }
```

> The `box` path is emitted **verbatim** — the consuming crate must be able to
> resolve it. In particular `box = "alloc::boxed::Box"` (or `"::alloc::..."`)
> requires the consumer's own `extern crate alloc;`, since the macro cannot add a
> crate-root import. For no-std the **`alloc` feature** is the self-contained
> choice (it aliases `alloc` internally); reach for `box` only for a genuinely
> custom `Box`.

---

### Field attributes (struct fields, and fields of struct/tuple variants)

| Attribute | Value | Effect |
|-----------|-------|--------|
| `with = "mod"` | a module | `mod::arbitrary(g: &mut Gen) -> FieldT` + `mod::shrink(v: &FieldT) -> Box<dyn Iterator<Item = FieldT>>` |
| `arbitrary = "fn"` | a function | `fn(g: &mut Gen) -> FieldT` — gen half only |
| `shrink = "fn"` | a function | `fn(v: &FieldT) -> Box<dyn Iterator<Item = FieldT>>` — shrink half only |
| `default` | — | Generate via `Default::default()`; the field is **held constant** when shrinking. |

`with` is mutually exclusive with `arbitrary` and `shrink`. `default` is mutually
exclusive with `with` and `arbitrary`. The per-field shrink rule:

- `with = "mod"` → use `mod::shrink`;
- `shrink = "fn"` → use `fn`;
- plain field → `quickcheck::Arbitrary::shrink`;
- `arbitrary = "fn"`-without-`shrink`, or `default` → **held constant** (never shrunk).

```rust,ignore
#[derive(Clone, Arbitrary)]
struct Packet {
    // serde-style pair: one module provides both halves
    #[quickcheck(with = "foreign_id_helpers")]
    id: ForeignId,

    // single-fn forms — useful when you only need one direction
    #[quickcheck(arbitrary = "gen_other", shrink = "shrink_other")]
    other: ForeignOther,

    // never generated from `g`; always `Default::default()`, never shrunk
    #[quickcheck(default)]
    cached: Cache,

    // plain: Arbitrary::arbitrary / Arbitrary::shrink
    payload: Vec<u8>,
}

mod foreign_id_helpers {
    use quickcheck::Gen;
    use super::ForeignId;

    pub fn arbitrary(g: &mut Gen) -> ForeignId { ForeignId::new(u32::arbitrary(g)) }
    pub fn shrink(v: &ForeignId) -> Box<dyn Iterator<Item = ForeignId>> {
        Box::new(v.as_u32().shrink().map(ForeignId::new))
    }
}
```

---

### Variant attributes (enum variants)

| Attribute | Value | Effect |
|-----------|-------|--------|
| `skip` | — | Exclude from `arbitrary` selection. A value that *is* this variant shrinks to empty. If **every** variant is `skip`, a `compile_error!` is produced. |
| `with = "mod"` | a module | `mod::arbitrary(g: &mut Gen) -> Self` (yielding this variant) + `mod::shrink(v: &Self) -> Box<dyn Iterator<Item = Self>>` |
| `arbitrary = "fn"` | a function | Generate the whole `Self` value as this variant: `fn(g: &mut Gen) -> Self`. Takes precedence over the variant's field attributes. |
| `shrink = "fn"` | a function | Shrink a value of this variant: `fn(v: &Self) -> Box<dyn Iterator<Item = Self>>`. `arbitrary`-without-`shrink` ⇒ empty for that variant. |

`arbitrary` picks **uniformly** among the non-skipped variants via `g.choose`.
Variants without a variant-level `with` or `arbitrary` are generated
field-by-field; their fields accept the field attributes above. The same
mutual-exclusion rules as the container apply (`with` vs `arbitrary`/`shrink`).

```rust,ignore
#[derive(Clone, Arbitrary)]
enum Event {
    #[quickcheck(skip)]
    Internal,

    Tick,
    Resize { width: u32, height: u32 },

    // serde-style pair on a variant
    #[quickcheck(with = "custom_helpers")]
    Custom(Payload),

    // single-fn forms
    #[quickcheck(arbitrary = "gen_special")]
    Special(u32),

    // per-field attributes inside a variant
    Frame {
        #[quickcheck(arbitrary = "gen_pixels")]
        pixels: Pixels,
        index: u64,
    },
}

mod custom_helpers {
    use super::*;
    pub fn arbitrary(g: &mut Gen) -> Event { Event::Custom(Payload::arbitrary(g)) }
    pub fn shrink(_v: &Event) -> Box<dyn Iterator<Item = Event>> { Box::new(std::iter::empty()) }
}
fn gen_special(g: &mut Gen) -> Event { Event::Special(u32::arbitrary(g)) }
fn gen_pixels(g: &mut Gen) -> Pixels { Pixels::arbitrary(g) }
```

## Codegen summary

- **Output** — wrapped in `const _: () = { impl … { fn arbitrary; fn shrink } };`;
  `Arbitrary` / `Gen` are referenced through the `crate` path.
- **`where` clause** — `split_for_impl` + either explicit `bound`s or inferred
  predicates: `Self: Clone + 'static` (the `Arbitrary` supertrait) plus a
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
- `with` together with `arbitrary` on the same item (`with = "mod"` already
  provides `arbitrary` via `mod::arbitrary`);
- `with` together with `shrink` on the same item (same reasoning);
- `default` together with `with` or `arbitrary` on the same field;
- an `enum` whose every variant is `#[quickcheck(skip)]`.

## Features

This is a proc-macro crate; its features select what the generated `shrink`
returns:

- **`std`** (default) — `shrink` returns `::std::boxed::Box<dyn Iterator<…>>`.
- **`alloc`** — for no-std consumers: `shrink` returns an
  `alloc::boxed::Box<dyn Iterator<…>>`. Enable with
  `default-features = false, features = ["alloc"]`. **Self-contained**: the
  generated `const` block aliases `alloc` internally
  (`extern crate alloc as <alias>;`), so the consumer needs **no**
  `extern crate alloc;` of its own.

Because Cargo **unifies** features and a proc-macro is compiled **once**, the
`std`/`alloc` choice is workspace-global, not per-consumer: `std` wins if both end
up enabled, and with **neither** the derive emits a `compile_error!` rather than
guessing. If a no-std consumer is in a workspace where `std` gets forced on, give
that type an explicit `#[quickcheck(box = "...")]` pointing at a `Box` it *can*
resolve (a re-exported one, or `alloc::boxed::Box` **with** the consumer's own
`extern crate alloc;` — the `box` path is emitted verbatim and the macro cannot
add a crate-root import). Generation otherwise uses only `core` paths
(`core::iter::Iterator`, `core::clone::Clone`, `core::default::Default`), so the
output is no-std-ready.

## Limitations

- **`#[repr(packed)]` structs are not supported.** The field-derived `shrink`
  borrows fields (`&self.field`), which is invalid for a packed layout (rustc
  `error[E0793]: reference to packed field is unaligned`). Use a non-packed type,
  or skip the field-derived path entirely — either with `#[quickcheck(with =
  "module")]` where the module exports both `arbitrary` and `shrink`, or with
  paired `#[quickcheck(arbitrary = "fn", shrink = "fn")]` overrides.
- **Edition 2018 or later** is required by consumers. The generated code uses
  absolute `::core` paths, which edition 2015 does not have in the crate root
  (it would need `extern crate core;`). Editions 2018 and 2021 are unaffected.

## License

`quickcheck-richderive` is under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT) for details.

Copyright (c) 2026 Al Liu.

[`quickcheck::Arbitrary`]: https://docs.rs/quickcheck/latest/quickcheck/trait.Arbitrary.html

[Github-url]: https://github.com/al8n/quickcheck-richderive/
[CI-url]: https://github.com/al8n/quickcheck-richderive/actions/workflows/ci.yml
[doc-url]: https://docs.rs/quickcheck-richderive
[crates-url]: https://crates.io/crates/quickcheck-richderive
[codecov-url]: https://app.codecov.io/gh/al8n/quickcheck-richderive/
