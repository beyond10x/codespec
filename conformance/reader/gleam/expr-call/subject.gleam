fn twice(n: Int) -> Int {
  let assert <<result:64-signed>> = <<{ n + n }:64>>
  result
}

pub fn quad(n: Int) -> Int {
  twice(twice(n))
}
