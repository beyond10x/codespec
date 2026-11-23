defmodule Codespec.BeamCLI do
  @moduledoc false
  @version System.fetch_env!("CODESPEC_ADAPTER_VERSION")
  @adapters %{
    {"read", "elixir"} => Codespec.ElixirReader,
    {"write", "elixir"} => Codespec.ElixirWriter,
    {"run", "elixir"} => Codespec.ElixirRunner,
    {"read", "gleam"} => Codespec.GleamReader,
    {"write", "gleam"} => Codespec.GleamWriter,
    {"run", "gleam"} => Codespec.GleamRunner
  }

  def main([name | args]) do
    ["codespec", role, lang] = String.split(name, "-")
    adapter = Map.fetch!(@adapters, {role, lang})

    try do
      case {role, args} do
        {r, ["--describe"]} when r in ["read", "write"] ->
          %{
            "format" => "codespec-adapter/1",
            "lang" => lang,
            "role" => if(r == "read", do: "reader", else: "writer"),
            "version" => @version,
            "cir" => "cir/1",
            "features" => apply(adapter, :features, [])
          }
          |> JSON.encode!()
          |> IO.puts()

        {"read", [path]} ->
          adapter |> apply(:read, [path, @version]) |> JSON.encode!() |> IO.puts()

        {"write", [out]} ->
          module = :stdio |> IO.binread(:eof) |> JSON.decode!()
          apply(adapter, :write, [module, out])

        {"run", ["--build", build, source, calls]} ->
          run(adapter, source, calls, build)

        {"run", [source, calls]} ->
          run(adapter, source, calls, nil)

        _ ->
          raise ArgumentError, "invalid arguments for #{name}"
      end
    rescue
      e ->
        IO.puts(:stderr, "#{name}: #{Exception.message(e)}")
        System.halt(if(role == "run" and is_struct(e, ArgumentError), do: 2, else: 1))
    end
  end

  defp run(Codespec.ElixirRunner, source, calls, _build),
    do: Codespec.ElixirRunner.run(source, calls)

  defp run(Codespec.GleamRunner, source, calls, build),
    do: Codespec.GleamRunner.run(source, calls, build)
end
