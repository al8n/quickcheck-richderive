use quickcheck_richderive::quickcheck;

#[quickcheck(case = 10)]
fn t(x: u8) -> bool {
  let _ = x;
  true
}

fn main() {}
