pub fn neg(n: Int) -> Int {
  let assert <<result:64-signed>> = <<{ -n }:64>>
  result
}
