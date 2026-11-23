defmodule Codespec.ElixirWriter do
  @moduledoc """
  Writes scalar CIR functions as a single Elixir module. Integer operations
  use the CIR width on the BEAM, including overflow of the minimum integer
  divided by -1. A refusal precedes any write to the output directory.
  """

  alias Codespec.ScalarWriter

  @features ~w(assign.add assign.and assign.and_not assign.div assign.mul assign.or assign.rem
               assign.shl assign.shr assign.sub assign.xor expr.binop expr.call expr.lit
               expr.unop expr.var op.add op.and op.and_not op.div op.eq op.ge op.gt op.land op.le
               op.lor op.lt op.mul op.ne op.or op.rem op.shl op.shr op.sub op.xor sem.int.wrap
               sem.rem.sign stmt.assign stmt.decr stmt.if stmt.incr stmt.let stmt.return
               stmt.var type.bool type.int type.string unop.complement unop.neg unop.not unop.plus)
  @escaped ~w(and or quote)

  def features, do: @features

  def write(module, out_dir) do
    {id, functions} = ScalarWriter.compile(module, __MODULE__)
    name = Macro.camelize(id)

    if name == "Elixir" or ScalarWriter.existing_module?("Elixir." <> name),
      do: raise(ArgumentError, "module name #{inspect(name)} already belongs to the runtime")

    code = [
      "defmodule ",
      name,
      " do\n",
      imports(functions),
      Enum.map_join(functions, "\n", &function/1),
      "\nend"
    ]

    source = [code |> IO.iodata_to_binary() |> Code.format_string!(), "\n"]
    File.mkdir_p!(out_dir)
    File.write!(Path.join(out_dir, "module.ex"), source)
  end

  @keywords ~w(after and catch do else end false in nil not or rescue true when fn)
  @special (Kernel.SpecialForms.__info__(:macros) ++ Kernel.__info__(:macros))
           |> Keyword.keys()
           |> Enum.map(&Atom.to_string/1)
  def reserved?(name, :variable), do: name in @keywords

  def reserved?(name, :function),
    do: name not in @escaped and (name in @keywords or name in @special or name == "module_info")

  defp imports(fs) do
    kernel = Kernel.__info__(:functions) ++ Kernel.__info__(:macros)

    except =
      for f <- fs,
          {name, arity} <- kernel,
          Atom.to_string(name) == f.name and arity == length(f.params),
          do: "#{f.name}: #{arity}"

    if except == [], do: "", else: "import Kernel, except: [#{Enum.join(except, ", ")}]\n"
  end

  defp function(f) do
    if not f.public and f.name in @escaped,
      do: raise(ArgumentError, "unsupported private function #{inspect(f.name)}")

    name = if f.name in @escaped, do: "unquote(:#{f.name})", else: f.name
    params = Enum.map_join(f.params, ", ", &elem(&1, 0))
    types = Enum.map_join(f.params, ", ", &type(elem(&1, 1)))
    defn = if f.public, do: "def", else: "defp"

    "@spec #{name}(#{types}) :: #{type(f.result)}\n" <>
      "#{defn} #{name}(#{params}) do\n#{render(f.body)}\nend\n"
  end

  defp type({:int, _}), do: "integer()"
  defp type(:string), do: "String.t()"
  defp type(:bool), do: "boolean()"
  defp type(:unit), do: ":ok"
  defp type({:tuple, ts}), do: "{" <> Enum.map_join(ts, ", ", &type/1) <> "}"
  defp tuple(xs), do: "{" <> Enum.join(xs, ", ") <> "}"

  defp render(:unit), do: ":ok"

  defp render({:lit, x}),
    do: inspect(x, binaries: :as_strings, printable_limit: :infinity, limit: :infinity)

  defp render({:var, id}), do: id
  defp render({:tuple, xs}), do: tuple(Enum.map(xs, &render/1))

  defp render({:call, name, args}) do
    callee = if name in @escaped, do: "__MODULE__.#{name}", else: name
    "#{callee}(#{Enum.map_join(args, ", ", &render/1)})"
  end

  defp render({:bind, names, rhs, next}) do
    lhs =
      case names do
        [] -> "_"
        [n] -> n
        ns -> tuple(ns)
      end

    "#{lhs} = #{render(rhs)}\n#{render(next)}"
  end

  defp render({:if, cond, yes, no}),
    do: "if #{render(cond)} do\n#{render(yes)}\nelse\n#{render(no)}\nend"

  defp render({:unary, "!", x, :bool}), do: "(not (#{render(x)}))"
  defp render({:unary, "+", x, _}), do: render(x)
  defp render({:unary, "-", x, {:int, bits}}), do: wrap("-(#{render(x)})", bits)
  defp render({:unary, "^", x, _}), do: "Bitwise.bnot(#{render(x)})"

  defp render({:binary, op, a, b, t}) do
    binary(op, render(a), render(b), t)
  end

  defp binary("+", a, b, :string), do: "(#{a} <> #{b})"
  defp binary("&&", a, b, :bool), do: "(#{a} && #{b})"
  defp binary("||", a, b, :bool), do: "(#{a} || #{b})"
  defp binary(op, a, b, _) when op in ~w(== != < <= > >=), do: "(#{a} #{op} #{b})"
  defp binary(op, a, b, {:int, bits}) when op in ~w(+ - *), do: wrap("(#{a} #{op} #{b})", bits)
  defp binary("/", a, b, {:int, bits}), do: wrap("Kernel.div(#{a}, #{b})", bits)
  defp binary("%", a, b, _), do: "Kernel.rem(#{a}, #{b})"
  defp binary("&", a, b, _), do: "Bitwise.band(#{a}, #{b})"
  defp binary("|", a, b, _), do: "Bitwise.bor(#{a}, #{b})"
  defp binary("^", a, b, _), do: "Bitwise.bxor(#{a}, #{b})"
  defp binary("&^", a, b, _), do: "Bitwise.band(#{a}, Bitwise.bnot(#{b}))"

  defp binary(op, a, b, {:int, bits}) when op in ["<<", ">>"] do
    fun = if op == "<<", do: "bsl", else: "bsr"
    shifted = "Bitwise.#{fun}(number, Kernel.min(count, #{bits}))"
    result = if op == "<<", do: wrap(shifted, bits), else: shifted

    "(case {#{a}, #{b}} do\n{_, count} when count < 0 -> raise ArithmeticError\n" <>
      "{number, count} -> #{result}\nend)"
  end

  defp wrap(x, bits) do
    "(case <<(#{x})::signed-#{bits}>> do\n<<value::signed-#{bits}>> -> value\nend)"
  end
end
