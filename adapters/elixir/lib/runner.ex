defmodule Codespec.BeamCalls do
  @moduledoc """
  The conformance call protocol for BEAM runners. Signatures come from the compiled
  subject. A call name is looked up among its exports without creating an atom.
  """

  @min_int -9_223_372_036_854_775_808
  @max_int 9_223_372_036_854_775_807

  def read(path) do
    with {:ok, bytes} <- File.read(path),
         {:ok, %{"format" => "codespec-calls/1", "calls" => [_ | _] = calls} = doc} <-
           JSON.decode(bytes),
         true <- map_size(doc) == 2 and Enum.all?(calls, &call?/1) do
      if canonical(calls) != bytes,
        do: raise(ArgumentError, "#{path}: calls are not canonical")

      calls
    else
      _ -> raise ArgumentError, "#{path}: expected a nonempty codespec-calls/1 document"
    end
  end

  defp call?(%{"func" => name, "args" => args} = call) do
    map_size(call) == 2 and is_binary(name) and is_list(args) and Enum.all?(args, &is_binary/1)
  end

  defp call?(_), do: false

  # serde_json's field order is part of the kit format. Comparison with the original
  # bytes also catches duplicate keys, trailing data, and alternative string escaping.
  defp canonical(calls) do
    rows =
      Enum.map_join(calls, ",\n", fn %{"func" => name, "args" => args} ->
        entries =
          case args do
            [] ->
              "[]"

            _ ->
              "[\n" <>
                Enum.map_join(args, ",\n", &("        " <> JSON.encode!(&1))) <> "\n      ]"
          end

        "    {\n      \"func\": #{JSON.encode!(name)},\n      \"args\": #{entries}\n    }"
      end)

    "{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n#{rows}\n  ]\n}\n"
  end

  def execute(calls, module, binary, exports) do
    specs =
      case Code.Typespec.fetch_specs(binary) do
        {:ok, entries} -> Map.new(entries)
        :error -> raise ArgumentError, "no compiled function signatures for #{inspect(module)}"
      end

    functions =
      exports
      |> Enum.reject(fn {name, arity} = entry ->
        (name == :module_info and arity in [0, 1]) or
          (name == :behaviour_info and not Map.has_key?(specs, entry)) or
          String.starts_with?(Atom.to_string(name), "__")
      end)
      |> Enum.group_by(fn {name, arity} -> {Macro.underscore(Atom.to_string(name)), arity} end)

    # Validate all calls before the first invocation.
    calls
    |> Enum.map(fn %{"func" => name, "args" => args} ->
      case Map.get(functions, {Macro.underscore(name), length(args)}, []) do
        [{function, _} = sig] ->
          {params, result} = signature(Map.get(specs, sig), name)
          {function, Enum.zip_with(args, params, &argument/2), result}

        [] ->
          raise ArgumentError, "no exported function #{inspect(name)}/#{length(args)}"

        _ ->
          raise ArgumentError, "ambiguous exported function #{inspect(name)}/#{length(args)}"
      end
    end)
    |> Enum.each(fn {function, args, result} ->
      value = native(fn -> apply(module, function, args) end)
      IO.puts(result_line(value, result))
    end)
  end

  defp signature([{:type, _, :fun, [{:type, _, :product, params}, result]}], _name),
    do: {Enum.map(params, &kind/1), kind(result)}

  defp signature(_, name),
    do: raise(ArgumentError, "expected one concrete signature for #{inspect(name)}")

  defp kind({:ann_type, _, [_name, type]}), do: kind(type)
  defp kind({:type, _, :integer, []}), do: :integer
  defp kind({:type, _, :boolean, []}), do: :boolean
  defp kind({:type, _, :binary, []}), do: :string
  defp kind({:remote_type, _, [{:atom, _, String}, {:atom, _, :t}, []]}), do: :string
  defp kind({:atom, _, atom}) when atom in [nil, :ok], do: {:unit, atom}
  defp kind({:type, _, :tuple, items}) when is_list(items), do: {:tuple, Enum.map(items, &kind/1)}
  defp kind(_), do: raise(ArgumentError, "unsupported type in called function")

  defp argument(text, :integer) when byte_size(text) <= 20 do
    case Integer.parse(text) do
      {i, ""} when i in @min_int..@max_int ->
        if Integer.to_string(i) == text, do: i, else: invalid(text, :integer)

      _ ->
        invalid(text, :integer)
    end
  end

  defp argument("true", :boolean), do: true
  defp argument("false", :boolean), do: false

  defp argument(<<"\"", rest::binary>> = text, :string) when byte_size(rest) >= 1 do
    value = binary_part(rest, 0, byte_size(rest) - 1)

    if :binary.last(rest) == ?" and
         Enum.all?(:binary.bin_to_list(value), &(&1 in 32..126 and &1 not in [?", ?\\])) do
      value
    else
      invalid(text, :string)
    end
  end

  defp argument(text, type), do: invalid(text, type)

  defp invalid(text, type),
    do: raise(ArgumentError, "invalid #{inspect(type)} argument #{inspect(text)}")

  defp result_line(value, {:unit, value}), do: "ok"

  defp result_line(value, {:tuple, types}) when tuple_size(value) == length(types) do
    ["ok" | Enum.zip_with(Tuple.to_list(value), types, &show/2)] |> Enum.join(" ")
  end

  defp result_line(value, type), do: "ok " <> show(value, type)
  defp show(value, :integer) when value in @min_int..@max_int, do: Integer.to_string(value)
  defp show(value, :boolean) when is_boolean(value), do: Atom.to_string(value)

  defp show(value, :string) when is_binary(value) do
    IO.iodata_to_binary([?", quoted(value), ?"])
  end

  defp show(_value, type), do: raise(ArgumentError, "cannot print result as #{inspect(type)}")

  # Go strconv.Quote: printable Unicode categories pass through. A malformed UTF-8
  # byte uses a byte escape, so binary() results preserve every byte.
  @escapes %{
    ?" => "\\\"",
    ?\\ => "\\\\",
    7 => "\\a",
    8 => "\\b",
    9 => "\\t",
    10 => "\\n",
    11 => "\\v",
    12 => "\\f",
    13 => "\\r"
  }
  defp quoted(<<>>), do: []

  defp quoted(<<codepoint::utf8, rest::binary>>) do
    encoded = <<codepoint::utf8>>

    part =
      cond do
        Map.has_key?(@escapes, codepoint) -> @escapes[codepoint]
        codepoint in 32..126 -> encoded
        codepoint < 32 or codepoint == 127 -> escape("\\x", codepoint, 2)
        Regex.match?(~r/[\p{L}\p{M}\p{N}\p{P}\p{S}]/u, encoded) -> encoded
        codepoint <= 0xFFFF -> escape("\\u", codepoint, 4)
        true -> escape("\\U", codepoint, 8)
      end

    [part | quoted(rest)]
  end

  defp quoted(<<byte, rest::binary>>), do: [escape("\\x", byte, 2) | quoted(rest)]

  defp escape(prefix, number, length) do
    prefix <>
      (number |> Integer.to_string(16) |> String.downcase() |> String.pad_leading(length, "0"))
  end

  # ArgumentError denotes protocol refusal at the CLI; an exception raised by the
  # program itself always denotes failed execution, even if it is an ArgumentError.
  def native(fun) do
    fun.()
  rescue
    exception -> raise RuntimeError, Exception.message(exception)
  catch
    kind, reason -> raise RuntimeError, "#{kind}: #{inspect(reason)}"
  end
end

defmodule Codespec.ElixirRunner do
  @moduledoc "Compile one Elixir module in memory and run the conformance calls against it."
  alias Codespec.BeamCalls

  def run(source, calls_path) do
    calls = BeamCalls.read(calls_path)
    {file, _label} = Codespec.ElixirReader.source_file!(source)
    ast = BeamCalls.native(fn -> Code.string_to_quoted!(File.read!(file), file: file) end)

    case ast do
      {:defmodule, _, [{:__aliases__, _, parts}, _]} ->
        module = Module.concat(parts)

        if Code.ensure_loaded?(module),
          do: raise(ArgumentError, "subject module #{inspect(module)} is already loaded")

        compiled = BeamCalls.native(fn -> Code.compile_quoted(ast, file) end)

        try do
          case compiled do
            [{^module, binary}] ->
              BeamCalls.execute(calls, module, binary, module.__info__(:functions))

            _ ->
              raise ArgumentError, "expected one Elixir subject module"
          end
        after
          for {name, _} <- compiled do
            :code.delete(name)
            :code.purge(name)
          end
        end

      _ ->
        raise ArgumentError, "expected one Elixir subject module"
    end
  end
end
