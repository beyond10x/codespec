defmodule Codespec.GleamReaderTest do
  use ExUnit.Case, async: true
  alias Codespec.GleamReader

  @tag :tmp_dir
  test "source order, forward calls and an escaped string survive the native parser", %{
    tmp_dir: dir
  } do
    file = Path.join(dir, "sample.gleam")

    File.write!(file, ~S"""
    pub fn greeting(s: String) -> String { join(s) }
    fn join(s: String) -> String { "hello\n" <> s }
    """)

    m = GleamReader.read(file, "test")
    assert [] == m["refusals"]
    assert ["func:Greeting", "func:join"] == m["layout"]
    assert [first, second] = m["funcs"]
    assert [%{"results" => [%{"k" => "call", "name" => "join"}]}] = first["body"]
    assert [%{"results" => [%{"x" => %{"value" => "\"hello\\n\""}}]}] = second["body"]
  end

  @tag :tmp_dir
  test "the writer's grouped conditional subject stays a comparison", %{tmp_dir: dir} do
    file = Path.join(dir, "sample.gleam")

    File.write!(
      file,
      "pub fn sign(n: Int) -> Int { case { n > 0 } { True -> { 1 } False -> { 0 } } }"
    )

    m = GleamReader.read(file, "test")
    assert [] == m["refusals"]
    assert [%{"body" => [%{"k" => "if", "cond" => %{"op" => ">"}}]}] = m["funcs"]
  end

  @tag :tmp_dir
  test "an unbounded result stays foreign at its original line", %{tmp_dir: dir} do
    file = Path.join(dir, "sample.gleam")
    File.write!(file, "pub fn sum(a: Int, b: Int) -> Int {\n  a + b\n}\n")
    m = GleamReader.read(file, "test")
    assert [%{"what" => "a + b", "pos" => "sample.gleam:2"}] = m["refusals"]
    assert [%{"body" => [%{"k" => "foreign", "src" => "a + b"}]}] = m["funcs"]
  end
end
