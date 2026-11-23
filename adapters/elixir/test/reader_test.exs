defmodule Codespec.ElixirReaderTest do
  use ExUnit.Case, async: true

  alias Codespec.ElixirReader, as: Reader

  setup do
    dir =
      Path.join(
        "target/elixir-reader-test",
        Integer.to_string(System.unique_integer([:positive]))
      )

    File.mkdir_p!(dir)
    on_exit(fn -> File.rm_rf!(dir) end)
    %{dir: dir}
  end

  defp source(dir, text) do
    file = Path.join(dir, "sample.ex")
    File.write!(file, text)
    file
  end

  test "types, lexical bindings, calls to a later private definition and tail branches", %{
    dir: dir
  } do
    file =
      source(dir, """
      defmodule Sample do
        @spec choose(boolean(), integer()) :: integer()
        def choose(flag, n) do
          x = n
          x = +x
          if flag, do: keep(x), else: 3
        end
        @spec keep(integer()) :: integer()
        defp keep(n), do: n
      end
      """)

    assert doc = Reader.read(dir, "test")
    assert doc == Reader.read(file, "test")
    assert doc["name"] == "sample"
    assert doc["layout"] == ["func:Choose", "func:keep"]
    assert [f, %{"exported" => false}] = doc["funcs"]

    assert [
             %{"k" => "let"},
             %{"k" => "assign"},
             %{"k" => "if", "then" => [%{"results" => [%{"k" => "call", "name" => "keep"}]}]}
           ] = f["body"]

    assert Map.get(doc, "refusals", []) == []
  end

  test "arbitrary precision needs explicit wrapping; the original statement survives byte for byte",
       %{dir: dir} do
    file =
      source(dir, """
      defmodule Numbers do
        @spec add(integer(), integer()) :: integer()
        def add(a, b), do: a  +   b
        @spec wrapped(integer(), integer()) :: integer()
        def wrapped(a, b) do
          <<r::signed-64>> = <<(a + b)::signed-64>>
          r
        end
        @spec quotient(integer(), integer()) :: integer()
        def quotient(a, b) do
          case <<Kernel.div(a, b)::signed-64>> do
            <<result::signed-64>> -> result
          end
        end
      end
      """)

    doc = Reader.read(file, "test")
    assert [%{"what" => "a  +   b", "pos" => "sample.ex:3"}] = doc["refusals"]
    assert [a, b, c] = doc["funcs"]
    assert [%{"k" => "foreign", "src" => "a  +   b"}] = a["body"]
    assert [%{"k" => "let", "rhs" => [%{"k" => "binop", "op" => "+"}]}, _] = b["body"]
    assert [%{"k" => "return", "results" => [%{"op" => "/"}]}] = c["body"]
  end

  test "near misses of a signed wrap, an untyped return, and a shadowed intrinsic stay foreign",
       %{dir: dir} do
    file =
      source(dir, """
      defmodule NearMiss do
        @spec wrong(integer()) :: integer()
        def wrong(n) do
          case <<n::unsigned-64>> do
            <<value::signed-64>> -> value
          end
        end
        @spec bad_type(integer()) :: boolean()
        def bad_type(n), do: n
        @spec fake(integer()) :: integer()
        def fake(n), do: Unexpected.rem(n, 3)
      end
      """)

    doc = Reader.read(file, "test")

    assert Enum.map(doc["refusals"], & &1["pos"]) == [
             "sample.ex:4",
             "sample.ex:9",
             "sample.ex:11"
           ]

    assert [%{"what" => block} | _] = doc["refusals"]
    assert block == "case <<n::unsigned-64>> do\n      <<value::signed-64>> -> value\n    end"
  end

  test "an inline branch preserves its parentheses and stops at the next keyword", %{dir: dir} do
    file =
      source(dir, """
      defmodule Text do
        @spec choose(flag :: boolean()) :: String.t()
        def choose(flag), do: if(flag, do: (Other.read("é")), else: "ok")
        @spec choose_else(boolean()) :: String.t()
        def choose_else(flag), do: if(flag, do: "ok", else: (Other.read("é")))
      end
      """)

    doc = Reader.read(file, "test")

    assert Enum.map(doc["refusals"], &{&1["pos"], &1["what"]}) == [
             {"sample.ex:3", "(Other.read(\"é\"))"},
             {"sample.ex:5", "(Other.read(\"é\"))"}
           ]

    assert [%{"body" => [%{"else" => [%{"results" => [%{"value" => "\"ok\""}]}]}]} | _] =
             doc["funcs"]
  end

  test "unsupported top-level code is never run", %{dir: dir} do
    marker = Path.join(dir, "executed")
    file = source(dir, "defmodule Bad do\n  File.touch!(#{inspect(marker)})\nend\n")
    assert_raise ArgumentError, ~r/sample\.ex:2/, fn -> Reader.read(file, "test") end
    refute File.exists?(marker)
  end

  test "overloaded functions, multiple modules and missing signatures are refused", %{dir: dir} do
    file =
      source(dir, """
      defmodule Overloaded do
        @spec f(integer()) :: integer()
        def f(x), do: x
        @spec f() :: integer()
        def f(), do: 1
      end
      """)

    assert_raise ArgumentError, ~r/overload|collision|duplicate/, fn ->
      Reader.read(file, "test")
    end

    File.write!(file, "defmodule A do end\ndefmodule B do end")
    assert_raise ArgumentError, ~r/one module/, fn -> Reader.read(file, "test") end
    File.write!(file, "defmodule A do\n def foo(x), do: x\nend")
    assert_raise ArgumentError, ~r/@spec/, fn -> Reader.read(file, "test") end
  end

  test "a foreign inline expression ends before its sibling definition", %{dir: dir} do
    file =
      source(dir, """
      defmodule Inline do
        @spec f(integer()) :: integer()
        @spec g(integer()) :: integer()
        def f(x), do: Other.work(x); def g(x), do: x
      end
      """)

    assert [%{"what" => "Other.work(x)"}] = Reader.read(file, "test")["refusals"]
  end
end
