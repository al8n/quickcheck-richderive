use quickcheck_richderive::quickcheck;

fn my_gen(_g: &mut quickcheck::Gen) -> i32 {
  0
}

#[quickcheck]
fn t(#[strategy(my_gen)] (a, b): (i32, i32)) -> bool {
  let _ = (a, b);
  true
}

fn main() {}
