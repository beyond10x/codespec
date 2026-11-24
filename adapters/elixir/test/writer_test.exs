defmodule Codespec.ElixirWriterTest do
  use ExUnit.Case, async: true

  alias Codespec.ElixirWriter, as: Writer

  setup do
    dir =
      Path.join(
        "target",
        "elixir-writer-test-#{System.pid()}-#{System.unique_integer([:positive])}"
      )

    File.mkdir_p!(dir)
    on_exit(fn -> File.rm_rf!(dir) end)
    %{dir: dir}
  end

  defp kit(name) do
    path = Path.expand("../../../conformance/cases/#{name}/case.cir.json", __DIR__)
    JSON.decode!(File.read!(path))
  end

  defp output(dir), do: Path.join(dir, "module.ex")

  test "runs signed arithmetic at the limits", %{dir: dir} do
    Writer.write(kit("sem-int-wrap"), dir)
    [{module, _}] = Code.compile_file(output(dir))
    min = -9_223_372_036_854_775_808
    assert apply(module, :add, [-min - 1, 1]) == min
    assert apply(module, :div, [min, -1]) == min
    assert apply(module, :rem, [min, -1]) == 0
    assert apply(module, :neg, [min]) == min
    assert_raise ArithmeticError, fn -> apply(module, :div, [1, 0]) end
  end

  test "wraps a signed eight-bit result at its own width", %{dir: dir} do
    source = kit("op-add") |> JSON.encode!() |> String.replace("\"platform\"", "\"8\"")
    Writer.write(%{JSON.decode!(source) | "name" => "narrow_add"}, dir)
    [{module, _}] = Code.compile_file(output(dir))
    assert apply(module, :add, [127, 1]) == -128
  end

  test "early returns leave nested branches", %{dir: dir} do
    Writer.write(kit("stmt-if"), dir)
    [{module, _}] = Code.compile_file(output(dir))
    assert Enum.map([-1, 0, 1], &apply(module, :sign, [&1])) == [2, 0, 1]
  end

  test "exported special-form spellings are callable by name", %{dir: dir} do
    sample = kit("type-string")
    funcs = Enum.flat_map(["type-string", "op-and", "op-or"], &kit(&1)["funcs"])
    Writer.write(%{sample | "name" => "reserved_exports", "funcs" => funcs}, dir)
    [{module, _}] = Code.compile_file(output(dir))
    assert apply(module, :quote, ["text"]) == "text"
    assert apply(module, :and, [3, 5]) == 1
    assert apply(module, :or, [3, 5]) == 7
  end

  test "a rebinding crosses the end of a conditional", %{dir: dir} do
    sample = kit("stmt-if")
    [f] = sample["funcs"]
    [conditional, otherwise] = f["body"]
    n = conditional["cond"]["x"]
    [%{"results" => one}] = conditional["then"]
    assign = %{"k" => "assign", "lhs" => [n], "rhs" => one, "pos" => "case.go:6"}
    branch = conditional |> Map.put("then", [assign]) |> Map.delete("else")
    body = [branch, %{otherwise | "results" => [n]}]
    Writer.write(%{sample | "name" => "branch_update", "funcs" => [%{f | "body" => body}]}, dir)
    [{module, _}] = Code.compile_file(output(dir))
    assert apply(module, :sign, [22]) == 1
    assert apply(module, :sign, [-5]) == -5
  end

  test "a local and a local function can share a spelling", %{dir: dir} do
    sample = kit("expr-call")
    [twice, quad] = sample["funcs"]
    [p] = quad["params"]
    arg = Map.put(p, "k", "var")
    local = %{arg | "name" => "twice"}
    let = %{"k" => "let", "names" => ["twice"], "rhs" => [arg], "pos" => quad["pos"]}
    call = %{"k" => "call", "name" => "twice", "args" => [local], "t" => p["t"]}
    ret = %{"k" => "return", "results" => [call], "pos" => quad["pos"]}
    funcs = [twice, %{quad | "body" => [let, ret]}]
    Writer.write(%{sample | "name" => "local_call", "funcs" => funcs}, dir)
    [{module, _}] = Code.compile_file(output(dir))
    assert apply(module, :quad, [3]) == 6
  end

  test "a string literal cannot become Elixir interpolation", %{dir: dir} do
    sample = kit("expr-lit")
    [f] = sample["funcs"]
    [ret] = f["body"]
    string = %{"k" => "string"}
    text = ~S(#{raise "interpolation"})

    cases = [
      {"Text", JSON.encode!(text), text},
      {"Unicode", ~S("\u0041"), "A"},
      {"Backslash", ~S("\\u0041"), ~S(\u0041)}
    ]

    funcs =
      for {name, spelling, _} <- cases do
        literal = %{"k" => "lit", "value" => spelling, "t" => string}
        %{f | "name" => name, "results" => [string], "body" => [%{ret | "results" => [literal]}]}
      end

    Writer.write(%{sample | "name" => "literal_text", "funcs" => funcs}, dir)
    [{module, _}] = Code.compile_file(output(dir))

    for {name, _, expected} <- cases do
      assert apply(module, String.to_existing_atom(String.downcase(name)), []) == expected
    end
  end

  test "refuses an ambiguous source-language integer spelling", %{dir: dir} do
    sample = kit("expr-lit")
    [f] = sample["funcs"]
    [ret] = f["body"]
    [x] = ret["results"]
    body = [%{ret | "results" => [%{x | "value" => "010"}]}]

    assert_raise ArgumentError, ~r/decimal integer/, fn ->
      Writer.write(%{sample | "funcs" => [%{f | "body" => body}]}, dir)
    end

    assert File.ls!(dir) == []
  end

  test "requires facts and never writes a partial module", %{dir: dir} do
    sample = kit("op-add")

    assert_raise ArgumentError, ~r/codespec pass/, fn ->
      Writer.write(Map.delete(sample, "facts"), dir)
    end

    [f] = sample["funcs"]
    bad = %{f | "name" => "Other", "body" => [%{"k" => "future", "pos" => "test:9"}]}

    assert_raise ArgumentError, ~r/future/, fn ->
      Writer.write(%{sample | "funcs" => [f, bad]}, dir)
    end

    assert File.ls!(dir) == []
  end

  test "ignores what a normal build left out", %{dir: dir} do
    left = [%{"pos" => "lib.rs:2", "what" => "fn helper", "reason" => "test-only"}]
    sample = %{kit("op-add") | "name" => "left_out_add"}
    Writer.write(Map.put(sample, "left_out", left), dir)
    [{module, _}] = Code.compile_file(output(dir))
    assert apply(module, :add, [2, 3]) == 5
  end

  test "refuses collisions after converting identifiers", %{dir: dir} do
    sample = kit("op-add")
    [f] = sample["funcs"]

    assert_raise ArgumentError, ~r/collid/, fn ->
      Writer.write(
        %{sample | "funcs" => [%{f | "name" => "GetURL"}, %{f | "name" => "GetUrl"}]},
        dir
      )
    end

    assert File.ls!(dir) == []
  end
end
