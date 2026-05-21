//! Hygiene coverage (Finding #3): generated internal identifiers must not
//! collide with plausible user names. Compilation is the assertion; the bodies
//! also exercise `arbitrary` + `shrink` at runtime.

// The const-generic params are intentionally named `g` (lower-case) to collide
// with the macro's old `arbitrary` parameter; silence the naming lint.
#![allow(non_upper_case_globals)]

use quickcheck::{Arbitrary, Gen};
use quickcheck_derive::Arbitrary as DeriveArbitrary;

fn gen() -> Gen {
  Gen::new(16)
}

// A user const-generic parameter literally named `g` must not clash with the
// generated `arbitrary` parameter.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct ConstG<const g: usize> {
  a: u32,
}

#[test]
fn const_generic_named_g_compiles_and_runs() {
  let mut gen = gen();
  let value: ConstG<3> = ConstG::arbitrary(&mut gen);
  let _shrinks: Vec<ConstG<3>> = value.shrink().collect();
}

// A tuple struct const-generic named `g` (exercises the unnamed codegen path).
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct ConstGTuple<const g: usize>(u16, bool);

#[test]
fn const_generic_tuple_named_g() {
  let mut gen = gen();
  let _value: ConstGTuple<5> = ConstGTuple::arbitrary(&mut gen);
  let probe = ConstGTuple::<5>(7, true);
  let _shrinks: Vec<ConstGTuple<5>> = probe.shrink().collect();
}

// Fields literally named after the macro's old internal locals (`chain`, `g`,
// `out`) must compile and shrink correctly.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
struct CollidingFields {
  chain: u8,
  g: u16,
  out: bool,
}

#[test]
fn fields_named_like_internals_struct() {
  let value = CollidingFields {
    chain: 5,
    g: 9,
    out: true,
  };
  let shrinks: Vec<CollidingFields> = value.shrink().collect();
  // Each shrink changes exactly one field; nothing else is corrupted.
  for s in &shrinks {
    let diffs =
      (s.chain != value.chain) as u32 + (s.g != value.g) as u32 + (s.out != value.out) as u32;
    assert_eq!(diffs, 1, "exactly one field shrinks at a time: {s:?}");
  }
  assert!(!shrinks.is_empty());

  let mut gen = gen();
  let _generated = CollidingFields::arbitrary(&mut gen);
}

// Same colliding field names inside an enum struct-variant (exercises the
// variant_shrink named-binding path).
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
enum CollidingEnum {
  V { chain: u8, g: u16, out: bool },
  Other(u32),
}

#[test]
fn fields_named_like_internals_enum() {
  let value = CollidingEnum::V {
    chain: 4,
    g: 8,
    out: true,
  };
  for s in value.shrink() {
    match s {
      CollidingEnum::V { chain, g, out } => {
        let diffs = (chain != 4) as u32 + (g != 8) as u32 + (!out) as u32;
        assert_eq!(diffs, 1);
      }
      other => panic!("shrink changed variant: {other:?}"),
    }
  }

  let mut gen = gen();
  let _generated = CollidingEnum::arbitrary(&mut gen);
}

// Enum named-variant fields literally named after the macro's OWN internal
// idents (round-2 finding B): the named-variant shrink must bind fields to fresh
// positional idents, so these are never shadowed by a generated local.
#[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
enum InternalNames {
  V {
    __quickcheck_chain: u8,
    __quickcheck_field0: u16,
    __quickcheck_base0: bool,
    __quickcheck_shrunk: u32,
  },
  Other(u8),
}

#[test]
fn enum_fields_named_like_macro_internals() {
  let value = InternalNames::V {
    __quickcheck_chain: 3,
    __quickcheck_field0: 7,
    __quickcheck_base0: true,
    __quickcheck_shrunk: 11,
  };
  for s in value.shrink() {
    match s {
      InternalNames::V {
        __quickcheck_chain,
        __quickcheck_field0,
        __quickcheck_base0,
        __quickcheck_shrunk,
      } => {
        let diffs = (__quickcheck_chain != 3) as u32
          + (__quickcheck_field0 != 7) as u32
          + (!__quickcheck_base0) as u32
          + (__quickcheck_shrunk != 11) as u32;
        assert_eq!(diffs, 1, "exactly one field shrinks at a time");
      }
      other => panic!("shrink changed variant: {other:?}"),
    }
  }

  let mut gen = gen();
  let _generated = InternalNames::arbitrary(&mut gen);
}

// NOTE on the residual limitation: a user `const` parameter named *exactly* like
// one of the macro's generated locals (e.g. `const __quickcheck_chain`) still
// cannot be supported — rustc rejects it with E0530 ("let bindings cannot shadow
// const parameters") and resolves identically-named pattern bindings to the const
// param, *both pre-hygiene*, so even `Span::mixed_site()` idents collide. No
// derive can generate a binding guaranteed distinct from such a const param; the
// `__quickcheck_` prefix makes this an effectively impossible name in practice.
