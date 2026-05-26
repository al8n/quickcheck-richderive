use quickcheck_richderive::quickcheck;

#[quickcheck(cases = 10, cases = 20)]
fn t(x: u8) -> bool {
  let _ = x;
  true
}

fn main() {}
