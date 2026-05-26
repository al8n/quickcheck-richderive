use quickcheck_richderive::quickcheck;

#[quickcheck]
async fn t(x: u8) -> bool {
  let _ = x;
  true
}

fn main() {}
