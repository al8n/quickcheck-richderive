//! `with = "mod"` — module-based pair, providing both `arbitrary` and `shrink`.
//!
//! This is the serde-style form: `#[quickcheck(with = "mod")]` expects the
//! module to export `mod::arbitrary(g: &mut Gen) -> Self` and
//! `mod::shrink(v: &Self) -> Box<dyn Iterator<Item = Self>>`. The two are
//! emitted together (you cannot mix-and-match against `arbitrary = "fn"` or
//! `shrink = "fn"` — the parser rejects it).

use quickcheck::{Arbitrary, Gen};
use quickcheck_richderive::Arbitrary as DeriveArbitrary;

fn gen_() -> Gen {
  Gen::new(16)
}

// ─── container-level `with = "mod"` on a struct ──────────────────────────────

mod whole_struct {
  use super::*;

  #[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
  #[quickcheck(with = "whole_helpers")]
  pub(super) struct Whole {
    pub(super) a: u32,
    pub(super) b: u32,
  }

  pub(super) mod whole_helpers {
    use super::Whole;
    use quickcheck::Gen;

    pub fn arbitrary(_g: &mut Gen) -> Whole {
      Whole { a: 42, b: 99 }
    }

    pub fn shrink(value: &Whole) -> Box<dyn Iterator<Item = Whole>> {
      if value.a > 0 {
        Box::new(std::iter::once(Whole { a: 0, b: value.b }))
      } else {
        Box::new(std::iter::empty())
      }
    }
  }
}

#[test]
fn container_with_mod_uses_both_arbitrary_and_shrink() {
  let mut g = gen_();
  let v = whole_struct::Whole::arbitrary(&mut g);
  assert_eq!(v, whole_struct::Whole { a: 42, b: 99 });

  let probe = whole_struct::Whole { a: 5, b: 7 };
  let shrinks: Vec<_> = probe.shrink().collect();
  assert_eq!(shrinks, vec![whole_struct::Whole { a: 0, b: 7 }]);
}

// ─── container-level `with = "mod"` on an enum ───────────────────────────────

mod whole_enum {
  use super::*;

  #[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
  #[quickcheck(with = "e_helpers")]
  pub(super) enum E {
    A(u32),
    #[allow(dead_code)] // helper only emits `A`; `B` exists to keep the type a real enum.
    B,
  }

  pub(super) mod e_helpers {
    use super::E;
    use quickcheck::Gen;

    pub fn arbitrary(_g: &mut Gen) -> E {
      E::A(7)
    }

    pub fn shrink(value: &E) -> Box<dyn Iterator<Item = E>> {
      match *value {
        E::A(n) if n > 0 => Box::new(std::iter::once(E::A(0))),
        _ => Box::new(std::iter::empty()),
      }
    }
  }
}

#[test]
fn enum_with_mod_uses_both_arbitrary_and_shrink() {
  let mut g = gen_();
  assert_eq!(whole_enum::E::arbitrary(&mut g), whole_enum::E::A(7));
  let shrinks: Vec<_> = whole_enum::E::A(3).shrink().collect();
  assert_eq!(shrinks, vec![whole_enum::E::A(0)]);
}

// ─── field-level `with = "mod"` ──────────────────────────────────────────────

mod field_with_mod {
  use super::*;

  // Verifies the field-type form: `mod::arbitrary(g) -> FieldT` +
  // `mod::shrink(v: &FieldT) -> Box<dyn Iterator<Item = FieldT>>`.
  #[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
  pub(super) struct S {
    #[quickcheck(with = "u8_helpers")]
    pub(super) x: u8,
    pub(super) y: u8,
  }

  pub(super) mod u8_helpers {
    use quickcheck::Gen;

    pub fn arbitrary(_g: &mut Gen) -> u8 {
      7
    }

    /// Yields a sentinel value (`u8::MAX`) that `u8`'s default `Arbitrary::shrink`
    /// can never produce — it only walks toward zero — so observing it in the
    /// shrunken set unambiguously proves `mod::shrink` was used.
    pub fn shrink(_v: &u8) -> Box<dyn Iterator<Item = u8>> {
      Box::new(std::iter::once(u8::MAX))
    }
  }
}

#[test]
fn field_with_mod_uses_both() {
  let mut g = gen_();
  let v = field_with_mod::S::arbitrary(&mut g);
  assert_eq!(v.x, 7);
  // `y` is field-derived → unconstrained; what we care about is that `x` came
  // from the helper.
  let probe = field_with_mod::S { x: 9, y: 1 };
  let shrinks: Vec<_> = probe.shrink().collect();
  // The default `u8` shrink for `x = 9` only walks toward zero, so a shrunken
  // `x == u8::MAX` can ONLY come from our helper — proves the `with`-module
  // shrink dispatch fired. (A weaker `x == 0` assertion would also pass if the
  // dispatch were broken, since u8's default shrink yields 0 anyway.)
  assert!(
    shrinks.iter().any(|s| s.x == u8::MAX),
    "expected at least one shrink to have x == u8::MAX from the helper; \
     got: {:?}",
    shrinks
  );
}

// ─── variant-level `with = "mod"` ────────────────────────────────────────────

mod variant_with_mod {
  use super::*;

  #[derive(Clone, Debug, PartialEq, DeriveArbitrary)]
  pub(super) enum E {
    #[quickcheck(with = "v_helpers")]
    Custom(u8, u8),
    Plain(u8),
  }

  pub(super) mod v_helpers {
    use super::E;
    use quickcheck::Gen;

    pub fn arbitrary(_g: &mut Gen) -> E {
      E::Custom(11, 13)
    }

    pub fn shrink(value: &E) -> Box<dyn Iterator<Item = E>> {
      match *value {
        E::Custom(_, _) => Box::new(std::iter::once(E::Custom(0, 0))),
        _ => Box::new(std::iter::empty()),
      }
    }
  }
}

#[test]
fn variant_with_mod_uses_both() {
  // Probe a `Custom` value and confirm its shrink comes from the helper.
  let probe = variant_with_mod::E::Custom(5, 6);
  let shrinks: Vec<_> = probe.shrink().collect();
  assert_eq!(shrinks, vec![variant_with_mod::E::Custom(0, 0)]);
}
