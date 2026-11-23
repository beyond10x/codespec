import glance
import gleam/list
import gleam/string

/// Return unexpanded function definitions in source order. A cir/1 module
/// must account for all five declaration lists that Glance parses.
pub fn functions(
  source: String,
) -> Result(List(glance.Definition(glance.Function)), String) {
  case glance.module(source) {
    Ok(glance.Module([], [], [], [], functions)) -> Ok(list.reverse(functions))
    Ok(_) ->
      Error("imports, types and module constants are outside this reader")
    Error(reason) -> Error("invalid Gleam syntax: " <> string.inspect(reason))
  }
}
