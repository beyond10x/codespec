defmodule Codespec.ElixirRunnerTest do
  use ExUnit.Case, async: false
  import ExUnit.CaptureIO

  alias Codespec.ElixirRunner, as: Runner

  setup do
    dir =
      Path.join(
        "target",
        "elixir-runner-test-#{System.pid()}-#{System.unique_integer([:positive])}"
      )

    File.mkdir_p!(dir)
    on_exit(fn -> File.rm_rf!(dir) end)
    %{dir: dir, calls: Path.join(dir, "calls.json")}
  end

  defp calls(file, func, args) do
    body =
      case args do
        [] ->
          "[]"

        args ->
          "[\n" <> Enum.map_join(args, ",\n", &("        " <> JSON.encode!(&1))) <> "\n      ]"
      end

    File.write!(file, """
    {
      "format": "codespec-calls/1",
      "calls": [
        {
          "func": #{JSON.encode!(func)},
          "args": #{body}
        }
      ]
    }
    """)
  end

  test "runs the compiled subject and prints tuple values after its own output", ctx do
    File.write!(Path.join(ctx.dir, "case.ex"), """
    defmodule ElixirRunnerPair do
      @spec make_pair(integer(), String.t()) :: {boolean(), String.t()}
      def make_pair(n, s) do
        IO.puts("called")
        {n < 0, s <> "\\n"}
      end
    end
    """)

    calls(ctx.calls, "MakePair", ["-9223372036854775808", ~s("with 'quotes'")])

    assert capture_io(fn -> assert :ok = Runner.run(ctx.dir, ctx.calls) end) ==
             "called\nok true \"with 'quotes'\\n\"\n"
  end

  test "an empty source directory cannot produce the expected line", ctx do
    calls(ctx.calls, "Add", ["2", "3"])
    assert_raise ArgumentError, ~r/one.*\.ex/, fn -> Runner.run(ctx.dir, ctx.calls) end
  end

  test "calls must be canonical, nonempty, and have only the protocol fields", ctx do
    calls(ctx.calls, "Five", [])
    doc = File.read!(ctx.calls)

    for invalid <- [
          String.trim_trailing(doc),
          String.replace(doc, "\"func\"", "\"extra\": 1, \"func\""),
          String.replace(doc, "\"func\"", "\"func\": \"First\", \"func\""),
          String.replace(doc, "\"args\": []", "\"args\": null"),
          ~s({"format":"codespec-calls/1","calls":[]}),
          doc <> "trailing"
        ] do
      File.write!(ctx.calls, invalid)
      assert_raise ArgumentError, ~r/calls/, fn -> Runner.run(ctx.dir, ctx.calls) end
    end
  end

  test "arguments match the function's actual declared kinds", ctx do
    File.write!(Path.join(ctx.dir, "case.ex"), """
    defmodule ElixirRunnerInput do
      @spec echo(integer()) :: integer()
      def echo(n), do: n
      @spec quote_text(String.t()) :: String.t()
      def quote_text(s), do: s
    end
    """)

    for {name, invalid} <- [
          {"Echo", "01"},
          {"Echo", "-0"},
          {"Echo", "+1"},
          {"Echo", "9223372036854775808"},
          {"Echo", "true"},
          {"QuoteText", "1"},
          {"QuoteText", ~s("a\\nb")}
        ] do
      calls(ctx.calls, name, [invalid])
      assert_raise ArgumentError, ~r/argument/, fn -> Runner.run(ctx.dir, ctx.calls) end
    end
  end

  test "module metadata and private functions cannot be called", ctx do
    File.write!(Path.join(ctx.dir, "case.ex"), """
    defmodule ElixirRunnerPrivate do
      @spec five() :: integer()
      def five, do: hidden()
      @spec hidden() :: integer()
      defp hidden, do: 5
    end
    """)

    for name <- ["ModuleInfo", "__info__", "Hidden"] do
      calls(ctx.calls, name, [])
      assert_raise ArgumentError, ~r/exported/, fn -> Runner.run(ctx.dir, ctx.calls) end
    end
  end

  test "an exception from the subject is an execution failure", ctx do
    File.write!(Path.join(ctx.dir, "case.ex"), """
    defmodule ElixirRunnerFailure do
      @spec bad() :: integer()
      def bad, do: raise(ArgumentError, "from source")
    end
    """)

    calls(ctx.calls, "Bad", [])
    assert_raise RuntimeError, ~r/from source/, fn -> Runner.run(ctx.dir, ctx.calls) end
  end
end
