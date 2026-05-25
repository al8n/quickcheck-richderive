use quickcheck_richderive::test;

#[test(cases = "100")]
fn t(x: u8) -> bool {
  let _ = x;
  true
}

fn main() {}
