//! Integration coverage for `#[quickcheck_richderive::test]`.
//!
//! These tests run as `cargo test`'s own `#[test]` harness: each `#[quickcheck_richderive::test]`
//! item expands into a `#[test] fn` that drives `quickcheck::QuickCheck` against
//! the property body. A passing case = the runner ran `cases` iterations
//! without panicking; a `#[should_panic]` case = the runner *did* panic (i.e.
//! a property failure was correctly surfaced).

use quickcheck::{Arbitrary, Gen, TestResult};

// ---------------------------------------------------------------------------
// Bare form: vanilla `#[test]`-style use, no attribute args.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::test]
fn bare_form_bool(xs: Vec<u32>) -> bool {
  let mut a = xs.clone();
  a.sort();
  a.sort();
  let mut b = xs;
  b.sort();
  a == b
}

// ---------------------------------------------------------------------------
// `cases = N` shrinks the iteration count down; we just check it compiles and
// runs. Pair with `max_tests` to confirm both knobs are consumed.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::test(cases = 5, max_tests = 50)]
fn cases_and_max_tests(x: u8) -> bool {
  let _ = x;
  true
}

// ---------------------------------------------------------------------------
// `gen_size` and `min_tests_passed`.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::test(gen_size = 8, min_tests_passed = 1)]
fn gen_size_and_min(x: i32) -> bool {
  let _ = x;
  true
}

// ---------------------------------------------------------------------------
// Per-arg override + shrinking. The custom generator yields i32 in [1, 100].
// Property is always true, so we're really testing that the override
// resolves, the newtype wires through `.quickcheck()`, and `shrink` delegates
// to `<i32 as Arbitrary>::shrink` without panicking.
// ---------------------------------------------------------------------------

fn small_positive(g: &mut Gen) -> i32 {
  // u8 range keeps the value small enough to avoid swamping rare counter-
  // examples on quick-iteration tests; +1 guarantees positivity.
  (u8::arbitrary(g) as i32) + 1
}

#[quickcheck_richderive::test(cases = 30, a = "small_positive")]
fn per_arg_override(a: i32, b: String) -> bool {
  let _ = b;
  (1..=256).contains(&a)
}

// ---------------------------------------------------------------------------
// Return-type acceptance set. quickcheck's `Testable` impls cover `()`,
// `bool`, `TestResult`, and `Result<T: Testable, E: Debug>`; the macro
// shouldn't restrict any of them.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::test(cases = 5)]
fn returns_unit(x: u8) {
  // Asserting inside a unit-returning property is the canonical
  // panic-on-counterexample shape. The vacuously-true assertion proves the
  // wrapper accepts `-> ()` return types.
  assert!(x == x);
}

#[quickcheck_richderive::test(cases = 5)]
fn returns_test_result(x: i8) -> TestResult {
  if x.is_negative() {
    TestResult::discard()
  } else {
    TestResult::from_bool(x >= 0)
  }
}

#[quickcheck_richderive::test(cases = 5)]
fn returns_result(x: u8) -> Result<(), String> {
  // `Result<T: Testable, E: Debug>` — quickcheck treats `Err` as a failed
  // test. Always-Ok keeps the test passing.
  let _ = x;
  Ok(())
}

// ---------------------------------------------------------------------------
// Per-arg override *with* a `TestResult` return type. Cross-product coverage
// for the most likely real-world combination.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::test(cases = 10, n = "small_positive")]
fn override_with_test_result(n: i32) -> TestResult {
  TestResult::from_bool(n >= 1)
}

// ---------------------------------------------------------------------------
// `#[should_panic]` — confirms an intentionally-failing property *does*
// surface as a test failure (i.e. the wrapper isn't swallowing panics).
// ---------------------------------------------------------------------------

#[should_panic]
#[quickcheck_richderive::test(cases = 100)]
fn intentionally_failing(x: u32) -> bool {
  // Eventually quickcheck picks `0`; `0 < 0` is false; the runner panics.
  x < x
}

// ---------------------------------------------------------------------------
// Multiple overrides on a multi-arg fn. Confirms iteration order and that
// non-overridden args still pass through their natural `Arbitrary`.
// ---------------------------------------------------------------------------

fn always_zero_i32(_g: &mut Gen) -> i32 {
  0
}
fn always_unit_string(_g: &mut Gen) -> String {
  String::new()
}

#[quickcheck_richderive::test(cases = 5, a = "always_zero_i32", c = "always_unit_string")]
fn multiple_overrides(a: i32, b: u8, c: String) -> bool {
  // `a` and `c` are pinned; `b` floats freely.
  a == 0 && c.is_empty() && b == b
}

// ---------------------------------------------------------------------------
// `mut` binding on a parameter must survive the rewrite — confirms the inner
// fn re-emits `mut x: T` (not `x: T`).
// ---------------------------------------------------------------------------

#[quickcheck_richderive::test(cases = 5)]
fn mut_param(mut x: u32) -> bool {
  x = x.wrapping_add(1);
  x == x
}

// ---------------------------------------------------------------------------
// Zero-arg fn. Degenerate but valid — quickcheck happily runs `cases`
// iterations of a `fn() -> bool`.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::test(cases = 3)]
fn zero_args() -> bool {
  true
}
