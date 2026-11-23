defmodule Codespec.GleamRunnerTest do
  use ExUnit.Case, async: false
  import ExUnit.CaptureIO

  alias Codespec.GleamRunner, as: Runner

  setup do
    root =
      Path.join(
        "target",
        "gleam-runner-test-#{System.pid()}-#{System.unique_integer([:positive])}"
      )

    src = Path.join(root, "src")
    File.mkdir_p!(src)
    on_exit(fn -> File.rm_rf!(root) end)
    %{root: root, src: src, build: Path.join(root, "build")}
  end

  test "executes Gleam's Erlang backend with full integer precision", ctx do
    # The second addend would disappear in IEEE-754 arithmetic.
    File.write!(Path.join(ctx.src, "run_subject.gleam"), """
    pub fn add(a: Int, b: Int) -> Int { a + b }
    """)

    calls = Path.join(ctx.root, "calls.json")

    File.write!(calls, """
    {
      "format": "codespec-calls/1",
      "calls": [
        {
          "func": "Add",
          "args": [
            "9007199254740992",
            "1"
          ]
        }
      ]
    }
    """)

    assert capture_io(fn -> assert :ok = Runner.run(ctx.root, calls, ctx.build) end) ==
             "ok 9007199254740993\n"
  end

  test "refuses an empty source directory", ctx do
    calls = "conformance/cases/op-add/calls.json"

    assert_raise ArgumentError, ~r/no \.gleam source/, fn ->
      Runner.run(ctx.src, calls, ctx.build)
    end
  end
end
