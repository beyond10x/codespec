defmodule Codespec.GleamRunner do
  @moduledoc """
  Run a single Gleam module on Erlang. Only its .gleam source is copied to an
  isolated package. compile-package receives an empty library directory and
  never resolves dependencies or runs the source project's build scripts.
  """
  alias Codespec.BeamCalls

  def run(source, calls_path, build_dir \\ nil) do
    calls = BeamCalls.read(calls_path)
    file = Codespec.GleamSource.file!(source)
    name = Path.basename(file, ".gleam")

    unless Regex.match?(~r/\A[a-z][a-z0-9_]*\z/, name),
      do: raise(ArgumentError, "invalid Gleam module name #{inspect(name)}")

    base = Path.expand(build_dir || System.tmp_dir!())
    File.mkdir_p!(base)

    scratch =
      Path.join(
        base,
        "gleam-" <> Base.url_encode64(:crypto.strong_rand_bytes(12), padding: false)
      )

    File.mkdir!(scratch)

    try do
      {module, binary} = compile(file, name, scratch)

      if :code.which(module) != :non_existing,
        do: raise(ArgumentError, "Gleam subject would replace module #{inspect(module)}")

      case :code.load_binary(module, String.to_charlist(file), binary) do
        {:module, ^module} -> :ok
        {:error, reason} -> raise "cannot load compiled Gleam subject: #{inspect(reason)}"
      end

      try do
        BeamCalls.execute(calls, module, binary, module.module_info(:exports))
      after
        :code.delete(module)
        :code.purge(module)
      end
    after
      if is_nil(build_dir), do: File.rm_rf!(scratch)
    end
  end

  defp compile(file, name, scratch) do
    pkg = Path.join(scratch, "package")
    File.mkdir_p!(Path.join(pkg, "src"))
    File.mkdir!(Path.join(scratch, "lib"))

    File.write!(
      Path.join(pkg, "gleam.toml"),
      "name = \"codespec_subject\"\nversion = \"0.0.0\"\n"
    )

    File.cp!(file, Path.join([pkg, "src", name <> ".gleam"]))

    args = [
      "compile-package",
      "--target",
      "erlang",
      "--no-dev",
      "--package",
      "package",
      "--lib",
      "lib",
      "--out",
      "out"
    ]

    # Conformance runs several packages in parallel. The compiler's short-lived
    # Erlang VM needs only one scheduler per package, as does the runner itself.
    flags = System.get_env("ERL_FLAGS", "+S 1:1 +SDcpu 1 +SDio 1 +A 1")

    case System.cmd("gleam", args,
           cd: scratch,
           stderr_to_stdout: true,
           env: [{"ERL_FLAGS", flags}]
         ) do
      {_, 0} -> :ok
      {output, status} -> raise "gleam compile-package exited #{status}: #{String.trim(output)}"
    end

    binary = File.read!(Path.join([scratch, "out", "ebin", name <> ".beam"]))

    case :beam_lib.chunks(binary, [:exports]) do
      {:ok, {module, _}} -> {module, binary}
      _ -> raise "Gleam emitted an unreadable BEAM"
    end
  end
end
