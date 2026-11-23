pub fn sub(a: Int, b: Int) -> Int {
  let assert <<result:64-signed>> = <<{ a - b }:64>>
  result
}
