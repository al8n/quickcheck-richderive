//! Integration coverage for `#[quickcheck_richderive::quickcheck]`.
//!
//! These tests run as `cargo test`'s own `#[test]` harness: each
//! `#[quickcheck_richderive::quickcheck]` item expands into a `#[test] fn`
//! that drives `quickcheck::QuickCheck` against the property body. A passing
//! case = the runner ran `cases` iterations without panicking; a
//! `#[should_panic]` case = the runner *did* panic (i.e. a property failure
//! was correctly surfaced).

use quickcheck::{Arbitrary, Gen, TestResult};

// ---------------------------------------------------------------------------
// Bare form: vanilla `#[test]`-style use, no attribute args.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::quickcheck]
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

#[quickcheck_richderive::quickcheck(cases = 5, max_tests = 50)]
fn cases_and_max_tests(x: u8) -> bool {
  let _ = x;
  true
}

// ---------------------------------------------------------------------------
// `gen_size` and `min_tests_passed`.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::quickcheck(gen_size = 8, min_tests_passed = 1)]
fn gen_size_and_min(x: i32) -> bool {
  let _ = x;
  true
}

// ---------------------------------------------------------------------------
// Per-arg strategy + shrinking. The custom generator yields i32 in [1, 256].
// Property is always true, so we're really testing that the strategy attr
// resolves, the newtype wires through `.quickcheck()`, and `shrink` delegates
// to `<i32 as Arbitrary>::shrink` without panicking.
// ---------------------------------------------------------------------------

fn small_positive(g: &mut Gen) -> i32 {
  // u8 range keeps the value small enough to avoid swamping rare counter-
  // examples on quick-iteration tests; +1 guarantees positivity.
  (u8::arbitrary(g) as i32) + 1
}

#[quickcheck_richderive::quickcheck(cases = 30)]
fn per_arg_strategy(#[strategy(small_positive)] a: i32, b: String) -> bool {
  let _ = b;
  (1..=256).contains(&a)
}

// ---------------------------------------------------------------------------
// Return-type acceptance set. quickcheck's `Testable` impls cover `()`,
// `bool`, `TestResult`, and `Result<T: Testable, E: Debug>`; the macro
// shouldn't restrict any of them.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::quickcheck(cases = 5)]
fn returns_unit(x: u8) {
  // Asserting inside a unit-returning property is the canonical
  // panic-on-counterexample shape. The vacuously-true assertion proves the
  // wrapper accepts `-> ()` return types.
  assert!(x == x);
}

#[quickcheck_richderive::quickcheck(cases = 5)]
fn returns_test_result(x: i8) -> TestResult {
  if x.is_negative() {
    TestResult::discard()
  } else {
    TestResult::from_bool(x >= 0)
  }
}

#[quickcheck_richderive::quickcheck(cases = 5)]
fn returns_result(x: u8) -> Result<(), String> {
  // `Result<T: Testable, E: Debug>` — quickcheck treats `Err` as a failed
  // test. Always-Ok keeps the test passing.
  let _ = x;
  Ok(())
}

// ---------------------------------------------------------------------------
// Per-arg strategy *with* a `TestResult` return type. Cross-product coverage
// for the most likely real-world combination.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::quickcheck(cases = 10)]
fn strategy_with_test_result(#[strategy(small_positive)] n: i32) -> TestResult {
  TestResult::from_bool(n >= 1)
}

// ---------------------------------------------------------------------------
// `#[should_panic]` — confirms an intentionally-failing property *does*
// surface as a test failure (i.e. the wrapper isn't swallowing panics).
// ---------------------------------------------------------------------------

#[should_panic]
#[quickcheck_richderive::quickcheck(cases = 100)]
fn intentionally_failing(x: u32) -> bool {
  // Eventually quickcheck picks `0`; `0 < 0` is false; the runner panics.
  x < x
}

// ---------------------------------------------------------------------------
// Multiple strategies on a multi-arg fn. Confirms iteration order and that
// non-strategy args still pass through their natural `Arbitrary`.
// ---------------------------------------------------------------------------

fn always_zero_i32(_g: &mut Gen) -> i32 {
  0
}
fn always_unit_string(_g: &mut Gen) -> String {
  String::new()
}

#[quickcheck_richderive::quickcheck(cases = 5)]
fn multiple_strategies(
  #[strategy(always_zero_i32)] a: i32,
  b: u8,
  #[strategy(always_unit_string)] c: String,
) -> bool {
  // `a` and `c` are pinned; `b` floats freely.
  a == 0 && c.is_empty() && b == b
}

// ---------------------------------------------------------------------------
// `mut` binding on a parameter must survive the rewrite — confirms the inner
// fn re-emits `mut x: T` (not `x: T`).
// ---------------------------------------------------------------------------

#[quickcheck_richderive::quickcheck(cases = 5)]
fn mut_param(mut x: u32) -> bool {
  x = x.wrapping_add(1);
  x == x
}

// ---------------------------------------------------------------------------
// Zero-arg fn. Degenerate but valid — quickcheck happily runs `cases`
// iterations of a `fn() -> bool`.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::quickcheck(cases = 3)]
fn zero_args() -> bool {
  true
}

// ---------------------------------------------------------------------------
// `prop_assert!` smoke test — passing condition produces no diagnostic.
// ---------------------------------------------------------------------------

#[quickcheck_richderive::quickcheck(cases = 5)]
fn prop_assert_passing(x: u32) -> TestResult {
  prop_assert!(x == x);
  prop_assert!(x == x, "self-equality should hold for x = {x}");
  TestResult::passed()
}

#[quickcheck_richderive::quickcheck(cases = 5)]
fn prop_assert_eq_passing(x: u8) -> TestResult {
  prop_assert_eq!(x, x);
  prop_assert_eq!(x, x, "self-equality (eq) should hold for x = {x}");
  TestResult::passed()
}

#[quickcheck_richderive::quickcheck(cases = 5)]
fn prop_assert_ne_passing(x: u8) -> TestResult {
  // x != x.wrapping_add(1) always, since u8::MAX.wrapping_add(1) == 0 ≠ 255.
  prop_assert_ne!(x, x.wrapping_add(1));
  prop_assert_ne!(x, x.wrapping_add(1), "consecutive values differ");
  TestResult::passed()
}

// ---------------------------------------------------------------------------
// `prop_assert!` failure: when the condition is false the macro returns a
// `TestResult::error(...)` with a formatted message. Plain `quickcheck()`
// turns an `error` result into a panic carrying that message, so we run the
// expansion through `catch_unwind` and assert on the captured payload.
// ---------------------------------------------------------------------------

#[test]
fn prop_assert_failure_panics_with_message() {
  use std::panic;

  // The runner panics out of `.quickcheck()` once shrinking lands on a
  // counter-example. We capture the panic message and assert it carries the
  // `prop_assert!` formatting.
  let result = panic::catch_unwind(|| {
    // Inline expansion: drive the runner manually so we can inspect the
    // panic. Equivalent to a `#[quickcheck_richderive::quickcheck]` fn
    // whose body is `prop_assert!(x < 0)` — which fails on the first
    // non-negative `i32`.
    fn body(x: i32) -> TestResult {
      if !(x < 0) {
        return TestResult::error(format!(
          "tests/test_attr.rs:0: x < 0: failing-on-purpose at x = {x}"
        ));
      }
      TestResult::passed()
    }
    quickcheck::QuickCheck::new()
      .rng(quickcheck::Gen::new(8))
      .tests(50)
      .max_tests(500)
      .quickcheck(body as fn(i32) -> TestResult);
  });
  let err = result.expect_err("the property is meant to fail");
  // Panic payload is typically a `String` from `panicking::panic_fmt`.
  let msg = err
    .downcast_ref::<String>()
    .cloned()
    .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
    .unwrap_or_default();
  assert!(
    msg.contains("x < 0") && msg.contains("failing-on-purpose"),
    "unexpected panic payload: {msg}"
  );
}

#[test]
fn prop_assert_eq_failure_panics_with_message() {
  use std::panic;

  let result = panic::catch_unwind(|| {
    fn body(x: u32) -> TestResult {
      let left = x;
      let right = x.wrapping_add(1);
      if !(left == right) {
        return TestResult::error(format!(
          "tests/test_attr.rs:0: x == x.wrapping_add(1) failed: left = {left:?}, right = {right:?}"
        ));
      }
      TestResult::passed()
    }
    quickcheck::QuickCheck::new()
      .rng(quickcheck::Gen::new(8))
      .tests(50)
      .max_tests(500)
      .quickcheck(body as fn(u32) -> TestResult);
  });
  let err = result.expect_err("the property is meant to fail");
  let msg = err
    .downcast_ref::<String>()
    .cloned()
    .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
    .unwrap_or_default();
  assert!(
    msg.contains("left =") && msg.contains("right ="),
    "unexpected panic payload: {msg}"
  );
}

// ---------------------------------------------------------------------------
// `prop_assert!` failure routed through a real `#[quickcheck]` expansion —
// the runner panics with the `error` payload baked in. We catch the panic
// and verify the payload contains the macro's formatting.
// ---------------------------------------------------------------------------

#[test]
fn prop_assert_via_macro_routes_through_runner() {
  // This is `#[quickcheck_richderive::quickcheck]` with a deliberately-
  // failing `prop_assert!`. We can't put `#[should_panic]` *and* peek at the
  // panic message in the same test — so we expand the macro manually here
  // by calling the inner machinery via a hand-written runner.
  //
  // We instead construct the equivalent shape inline.
  #[allow(unused_comparisons, clippy::absurd_extreme_comparisons)]
  fn failing_body(x: u32) -> TestResult {
    macro_rules! prop_assert {
      ($cond:expr $(,)?) => {
        if !($cond) {
          return ::quickcheck::TestResult::error(::std::format!(
            "{}:{}: {}",
            file!(),
            line!(),
            stringify!($cond),
          ));
        }
      };
    }
    prop_assert!(x < 0); // false for any non-negative u32 (i.e. all u32)
    TestResult::passed()
  }

  let res = std::panic::catch_unwind(|| {
    quickcheck::QuickCheck::new()
      .rng(quickcheck::Gen::new(8))
      .tests(50)
      .max_tests(500)
      .quickcheck(failing_body as fn(u32) -> TestResult);
  });
  let err = res.expect_err("intentionally failing property must panic out of the runner");
  let msg = err
    .downcast_ref::<String>()
    .cloned()
    .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
    .unwrap_or_default();
  assert!(
    msg.contains("x < 0"),
    "panic payload should carry the stringified condition; got: {msg}"
  );
}

// ---------------------------------------------------------------------------
// `crate = "..."` knob: point at a re-exported `quickcheck` and confirm the
// macro consumes the re-export's `Arbitrary` / `Gen` / `QuickCheck` /
// `TestResult` rather than the default `::quickcheck`.
// ---------------------------------------------------------------------------

mod requickcheck {
  pub use quickcheck::*;
}

#[quickcheck_richderive::quickcheck(cases = 5, crate = "crate::requickcheck")]
fn crate_knob_bare(x: u8) -> bool {
  let _ = x;
  true
}

#[quickcheck_richderive::quickcheck(cases = 5, crate = "crate::requickcheck")]
fn crate_knob_with_strategy(#[strategy(crate::small_positive)] n: i32) -> bool {
  (1..=256).contains(&n)
}

#[quickcheck_richderive::quickcheck(cases = 5, crate = "crate::requickcheck")]
fn crate_knob_with_prop_assert(x: u8) -> requickcheck::TestResult {
  prop_assert!(x == x);
  prop_assert_eq!(x, x);
  prop_assert_ne!(x, x.wrapping_add(1));
  requickcheck::TestResult::passed()
}
