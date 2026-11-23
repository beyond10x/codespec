defmodule Codespec.GleamWriter do
  @moduledoc """
  Writes a Gleam package for the Erlang target. Only the integer operations
  used by the module introduce helpers. The package has no dependencies.
  """

  alias Codespec.ScalarWriter

  @keywords ~w(as assert auto case const delegate derive echo else fn if implement import let macro opaque panic pub test todo type use)
  @features ~w(assign.add assign.and assign.and_not assign.div assign.mul assign.or assign.rem
               assign.shl assign.shr assign.sub assign.xor expr.binop expr.call expr.lit
               expr.unop expr.var op.add op.and op.and_not op.div op.eq op.ge op.gt op.land op.le
               op.lor op.lt op.mul op.ne op.or op.rem op.shl op.shr op.sub op.xor sem.int.wrap
               sem.rem.sign stmt.assign stmt.decr stmt.if stmt.incr stmt.let stmt.var
               type.bool type.int type.string unop.complement unop.neg unop.not unop.plus)
  @externs %{
    "and" => {"band", ["Int", "Int"], "Int"},
    "or" => {"bor", ["Int", "Int"], "Int"},
    "xor" => {"bxor", ["Int", "Int"], "Int"},
    "not" => {"bnot", ["Int"], "Int"},
    "divide" => {"div", ["Int", "Int"], "Int"},
    "remainder" => {"rem", ["Int", "Int"], "Int"},
    "bsl" => {"bsl", ["Int", "Int"], "Int"},
    "bsr" => {"bsr", ["Int", "Int"], "Int"},
    "string_lt" => {"<", ["String", "String"], "Bool"},
    "string_le" => {"=<", ["String", "String"], "Bool"},
    "string_gt" => {">", ["String", "String"], "Bool"},
    "string_ge" => {">=", ["String", "String"], "Bool"}
  }

  def features, do: @features

  def reserved?(name, _),
    do: name in @keywords or name == "module_info" or String.starts_with?(name, "codespec_")

  def write(module, out_dir) do
    {id, functions} = ScalarWriter.compile(module, __MODULE__)

    if id in @keywords or id in ["erlang", "gleam"] or ScalarWriter.existing_module?(id),
      do: raise(ArgumentError, "reserved Gleam/Erlang module name #{inspect(id)}")

    {definitions, needed} = Enum.map_reduce(functions, MapSet.new(), &function/2)
    helpers = needed |> Enum.sort() |> Enum.map(&helper/1)
    source = Enum.join(definitions ++ helpers, "\n\n") <> "\n"
    manifest = "name = \"codespec_output\"\nversion = \"1.0.0\"\ntarget = \"erlang\"\n"
    src = Path.join(out_dir, "src")
    File.mkdir_p!(src)
    File.write!(Path.join(src, id <> ".gleam"), source)
    File.write!(Path.join(out_dir, "gleam.toml"), manifest)
  end

  defp function(f, needed) do
    {body, needed} = render(f.body, needed)
    visibility = if f.public, do: "pub ", else: ""
    params = Enum.map_join(f.params, ", ", fn {name, t} -> "#{name}: #{type(t)}" end)
    {"#{visibility}fn #{f.name}(#{params}) -> #{type(f.result)} {\n#{indent(body)}\n}", needed}
  end

  defp type({:int, _}), do: "Int"
  defp type(:bool), do: "Bool"
  defp type(:string), do: "String"
  defp type(:unit), do: "Nil"
  defp type({:tuple, ts}), do: tuple(Enum.map(ts, &type/1))
  defp tuple(xs), do: "#(" <> Enum.join(xs, ", ") <> ")"
  defp indent(s), do: "  " <> String.replace(s, "\n", "\n  ")

  defp render(:unit, h), do: {"Nil", h}
  defp render({:var, n}, h), do: {n, h}
  defp render({:lit, true}, h), do: {"True", h}
  defp render({:lit, false}, h), do: {"False", h}
  defp render({:lit, i}, h) when is_integer(i), do: {Integer.to_string(i), h}

  defp render({:lit, s}, h) when is_binary(s) do
    body = for <<c::utf8 <- s>>, into: "", do: escape(c)
    {"\"" <> body <> "\"", h}
  end

  defp render({:tuple, xs}, h) do
    {parts, h} = Enum.map_reduce(xs, h, &render/2)
    {tuple(parts), h}
  end

  defp render({:call, name, xs}, h) do
    {parts, h} = Enum.map_reduce(xs, h, &render/2)
    {"#{name}(#{Enum.join(parts, ", ")})", h}
  end

  defp render({:bind, names, rhs, next}, h) do
    {value, h} = render(rhs, h)
    {rest, h} = render(next, h)

    lhs =
      case names do
        [] -> "_"
        [id] -> id
        ids -> tuple(ids)
      end

    {"let #{lhs} = #{value}\n#{rest}", h}
  end

  defp render({:if, cond, yes, no}, h) do
    {cond, h} = render(cond, h)
    {yes, h} = render(yes, h)
    {no, h} = render(no, h)
    branch = "True -> {\n#{indent(yes)}\n}\nFalse -> {\n#{indent(no)}\n}"
    {"case #{cond} {\n#{indent(branch)}\n}", h}
  end

  defp render({:unary, op, x, t}, h) do
    {x, h} = render(x, h)

    case op do
      "+" -> {x, h}
      "-" -> wrap("-{ #{x} }", t, h)
      "!" -> {"!{ #{x} }", h}
      "^" -> call_helper("not", [x], h)
    end
  end

  defp render({:binary, op, a, b, t}, h) do
    {a, h} = render(a, h)
    {b, h} = render(b, h)
    binary(op, a, b, t, h)
  end

  defp binary("+", a, b, :string, h), do: {"{ #{a} <> #{b} }", h}

  defp binary(op, a, b, :string, h) when op in ["<", "<=", ">", ">="] do
    name = %{"<" => "lt", "<=" => "le", ">" => "gt", ">=" => "ge"}[op]
    call_helper("string_" <> name, [a, b], h)
  end

  defp binary(op, a, b, _, h) when op in ~w(== != < <= > >= && ||), do: {"{ #{a} #{op} #{b} }", h}
  defp binary(op, a, b, t, h) when op in ~w(+ - *), do: wrap("#{a} #{op} #{b}", t, h)

  defp binary("/", a, b, t, h) do
    {n, h} = call_helper("divide", [a, b], h)
    wrap(n, t, h)
  end

  defp binary("%", a, b, _, h), do: call_helper("remainder", [a, b], h)
  defp binary("&", a, b, _, h), do: call_helper("and", [a, b], h)
  defp binary("|", a, b, _, h), do: call_helper("or", [a, b], h)
  defp binary("^", a, b, _, h), do: call_helper("xor", [a, b], h)

  defp binary("&^", a, b, _, h) do
    {n, h} = call_helper("not", [b], h)
    call_helper("and", [a, n], h)
  end

  defp binary(op, a, b, {:int, bits} = t, h) when op in ["<<", ">>"] do
    name = if op == "<<", do: "bsl", else: "bsr"
    {count, h} = call_helper("shift_count", [b, Integer.to_string(bits)], h)
    {shifted, h} = call_helper(name, [a, count], h)
    if op == "<<", do: wrap(shifted, t, h), else: {shifted, h}
  end

  defp wrap(expr, {:int, bits}, h) do
    {"{\n  let assert <<codespec_value:#{bits}-signed>> = <<{ #{expr} }:#{bits}>>\n  codespec_value\n}",
     h}
  end

  defp call_helper(name, args, h),
    do: {"codespec_#{name}(#{Enum.join(args, ", ")})", MapSet.put(h, name)}

  defp helper("shift_count") do
    "fn codespec_shift_count(n: Int, bits: Int) -> Int {\n" <>
      "  case n {\n    n if n < 0 -> panic as \"negative shift amount\"\n" <>
      "    n if n >= bits -> bits\n    _ -> n\n  }\n}"
  end

  defp helper(name) do
    {fun, ts, result} = Map.fetch!(@externs, name)
    parameters = Enum.zip_with(["a", "b"], ts, &"#{&1}: #{&2}")

    "@external(erlang, \"erlang\", \"#{fun}\")\nfn codespec_#{name}(#{Enum.join(parameters, ", ")}) -> #{result}"
  end

  defp escape(?"), do: "\\\""
  defp escape(?\\), do: "\\\\"
  defp escape(?\n), do: "\\n"
  defp escape(?\r), do: "\\r"
  defp escape(?\t), do: "\\t"
  defp escape(c) when c < 32 or c in 0x7F..0x9F, do: "\\u{" <> Integer.to_string(c, 16) <> "}"
  defp escape(c), do: <<c::utf8>>
end
