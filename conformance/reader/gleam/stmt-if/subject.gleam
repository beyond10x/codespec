pub fn sign(n: Int) -> Int {
  case n > 0 {
    True -> 1
    False ->
      case n < 0 {
        True -> 2
        False -> 0
      }
  }
}
