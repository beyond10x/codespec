defmodule Codespec.GleamWriterTest do
  use ExUnit.Case, async: true

  alias Codespec.GleamWriter, as: Writer

  setup do
    dir =
      Path.join(
        "target",
        "gleam-writer-test-#{System.pid()}-#{System.unique_integer([:positive])}"
      )

    File.mkdir_p!(dir)
    on_exit(fn -> File.rm_rf!(dir) end)
    %{dir: dir}
  end

  defp kit(name) do
    path = Path.expand("../../../conformance/cases/#{name}/case.cir.json", __DIR__)
    JSON.decode!(File.read!(path))
  end

  # The else block ends before the call of Helper in CIR. A local called
  # helper must not capture that call when it moves into the Gleam branch.
  defp scope_example do
    [helper] = kit("expr-lit")["funcs"]
    [probe] = kit("stmt-if")["funcs"]
    [branch, rest] = probe["body"]
    zero = branch["cond"]["y"]
    local = %{"k" => "var", "name" => "helper", "t" => zero["t"]}

    let = fn name, value ->
      %{"k" => "let", "names" => [name], "rhs" => [value], "pos" => rest["pos"]}
    end

    branch = %{branch | "else" => [let.("helper", zero), let.("discard", local)]}
    call = %{"k" => "call", "name" => "Helper", "args" => [], "t" => zero["t"]}
    [%{helper | "name" => "Helper"}, %{probe | "body" => [branch, %{rest | "results" => [call]}]}]
  end

  test "the Erlang target runs signed overflow and refuses a zero divisor", %{dir: dir} do
    sample = kit("sem-int-wrap")
    [f] = kit("expr-lit")["funcs"]
    [ret] = f["body"]
    string = %{"k" => "string"}
    text = %{"k" => "lit", "value" => ~S("\u0041\\u0041"), "t" => string}
    f = %{f | "name" => "Text", "results" => [string], "body" => [%{ret | "results" => [text]}]}
    Writer.write(%{sample | "funcs" => [f | sample["funcs"]] ++ scope_example()}, dir)

    assert {_, 0} =
             System.cmd("gleam", ["build", "--target", "erlang"], cd: dir, stderr_to_stdout: true)

    beam = Path.join(dir, "build/dev/erlang/codespec_output/ebin/sem_int_wrap")
    assert {:module, :sem_int_wrap} = :code.load_abs(String.to_charlist(beam))

    on_exit(fn ->
      :code.delete(:sem_int_wrap)
      :code.purge(:sem_int_wrap)
    end)

    assert apply(:sem_int_wrap, :text, []) == ~S(A\u0041)
    assert apply(:sem_int_wrap, :sign, [-1]) == 5
    assert apply(:sem_int_wrap, :sign, [1]) == 1
    min = -9_223_372_036_854_775_808
    assert apply(:sem_int_wrap, :add, [-min - 1, 1]) == min
    assert apply(:sem_int_wrap, :div, [min, -1]) == min
    assert apply(:sem_int_wrap, :rem, [min, -1]) == 0
    assert_raise ArithmeticError, fn -> apply(:sem_int_wrap, :div, [1, 0]) end
  end

  test "refuses declarations or names it cannot write before creating a project", %{dir: dir} do
    assert_raise ArgumentError, ~r/unsupported module types/, fn ->
      Writer.write(kit("decl-struct"), dir)
    end

    assert_raise ArgumentError, ~r/reserved/, fn ->
      Writer.write(%{kit("op-add") | "name" => "erlang"}, dir)
    end

    assert File.ls!(dir) == []
  end
end
