defmodule Codespec.GleamReader do
  @moduledoc false

  @int %{"k" => "int", "width" => "platform", "signed" => true}
  @bool %{"k" => "bool"}
  @string %{"k" => "string"}
  @min -9_223_372_036_854_775_808
  @max 9_223_372_036_854_775_807
  @limit 1_048_576
  @ops %{
    eq: "==",
    not_eq: "!=",
    lt_int: "<",
    lt_eq_int: "<=",
    gt_int: ">",
    gt_eq_int: ">=",
    and: "&&",
    or: "||",
    concatenate: "+",
    add_int: "+",
    sub_int: "-",
    mult_int: "*",
    div_int: "/",
    remainder_int: "%"
  }

  def features, do: ~w(type.int type.bool type.string expr.lit expr.var expr.binop expr.unop
                        expr.call stmt.let stmt.assign stmt.return stmt.if op.eq op.ne op.lt
                        op.le op.gt op.ge op.land op.lor unop.not op.add op.sub op.mul unop.neg)

  def read(path, version) do
    file = Codespec.GleamSource.file!(path)
    source = File.open!(file, [:read, :binary], &IO.binread(&1, @limit + 1))

    unless is_binary(source) and byte_size(source) <= @limit and String.valid?(source),
      do: raise(ArgumentError, "expected UTF-8 Gleam source within 1 MiB")

    name = Path.basename(file, ".gleam")

    unless Regex.match?(~r/\A[a-z][a-z0-9_]*\z/, name),
      do: raise(ArgumentError, "invalid Gleam module name")

    file = Path.basename(file)

    definitions =
      case :codespec_gleam_parser.functions(source) do
        {:ok, functions} -> functions
        {:error, reason} -> raise ArgumentError, "#{file}: #{reason}"
      end

    signatures = Map.new(definitions, &signature(&1, file))
    names = Enum.map(signatures, fn {_name, sig} -> sig.name end)

    unless length(definitions) == map_size(signatures) and names == Enum.uniq(names),
      do: raise(ArgumentError, "#{file}: function names collide in cir/1")

    {functions, refusals} =
      Enum.map_reduce(definitions, [], fn {:definition, [],
                                           {:function, span, name, _, _, _, body}},
                                          refs ->
        sig = Map.fetch!(signatures, name)

        env = %{
          file: file,
          source: source,
          signatures: signatures,
          result: sig.result,
          vars: Map.new(sig.params, &{&1["name"], &1["t"]})
        }

        {statements, refs} =
          try do
            {block(body, env), refs}
          catch
            {:foreign, node, reason} ->
              span = if length(body) == 1, do: location(hd(body)), else: span
              src = snippet(source, span)
              pos = pos(env, span)
              site = location(node)

              stmt = %{
                "k" => "foreign",
                "lang" => "gleam",
                "src" => src,
                "reason" => reason,
                "pos" => pos
              }

              {[stmt],
               [
                 %{"what" => snippet(source, site), "pos" => pos(env, site), "reason" => reason}
                 | refs
               ]}
          end

        {%{
           "name" => sig.name,
           "exported" => sig.public,
           "params" => sig.params,
           "results" => List.wrap(sig.result),
           "pos" => pos(env, span),
           "body" => statements
         }, refs}
      end)

    %{
      "format" => "cir/1",
      "name" => name,
      "types" => [],
      "funcs" => functions,
      "reader" => %{"language" => "gleam", "name" => "codespec-read-gleam", "version" => version},
      "layout" => Enum.map(functions, &("func:" <> &1["name"])),
      "refusals" => Enum.reverse(refusals)
    }
  end

  defp signature(
         {:definition, [], {:function, _, name, visibility, params, result, [_ | _]}},
         file
       ) do
    public = visibility == :public

    params =
      Enum.map(params, fn
        {:function_parameter, :none, {kind, n}, t} when kind in [:named, :discarded] ->
          %{"name" => if(kind == :discarded, do: "_" <> n, else: n), "t" => type!(t, file)}

        _ ->
          raise ArgumentError, "#{file}: labelled parameters are outside this reader"
      end)

    names = Enum.map(params, & &1["name"])

    if length(names) != length(Enum.uniq(names)),
      do: raise(ArgumentError, "#{file}: duplicate parameter")

    {name,
     %{
       name: if(public, do: Macro.camelize(name), else: name),
       public: public,
       params: params,
       result: result_type!(result, file)
     }}
  end

  defp signature(_, file),
    do: raise(ArgumentError, "#{file}: an attributed or bodyless function is outside this reader")

  defp result_type!({:some, {:tuple_type, _, items}}, file) when length(items) > 1,
    do: Enum.map(items, &type!({:some, &1}, file))

  defp result_type!(annotation, file), do: type!(annotation, file)

  defp type!({:some, {:named_type, _, n, :none, []}}, _file) when n in ["Int", "Bool", "String"],
    do: %{"Int" => @int, "Bool" => @bool, "String" => @string}[n]

  defp type!(_, file),
    do: raise(ArgumentError, "#{file}: expected an explicit Int, Bool or String annotation")

  defp snippet(source, {:span, start, stop}), do: binary_part(source, start, stop - start)
  defp location({:expression, expr}), do: location(expr)
  defp location(node), do: elem(node, 1)

  defp pos(env, {:span, start, _}),
    do: "#{env.file}:#{1 + length(:binary.matches(binary_part(env.source, 0, start), "\n"))}"

  defp foreign(node, reason), do: throw({:foreign, node, reason})

  defp block([{:expression, expr}], env), do: tail(expr, env)
  # Gleam's native bit-array form explicitly truncates the result on the BEAM.
  defp block(
         [
           {:assignment, _, {:let_assert, :none},
            {:pattern_bit_string, _, [{{:pattern_variable, _, n}, options}]}, :none,
            {:bit_string, _, [{operand, [{:size_option, 64}]}]}} = node
           | rest
         ],
         env
       )
       when rest != [] do
    unless Enum.sort(options) == Enum.sort([:signed_option, {:size_option, 64}]) and
             not Map.has_key?(env.vars, n),
           do: foreign(node, "expected a fresh signed 64-bit bit-array binding")

    value = wrapped(operand, env)

    [
      %{"k" => "let", "names" => [n], "rhs" => [value], "pos" => pos(env, location(node))}
      | block(rest, %{env | vars: Map.put(env.vars, n, @int)})
    ]
  end

  defp block(
         [{:assignment, _, :let, {:pattern_variable, _, name}, annotation, rhs} = node | rest],
         env
       )
       when rest != [] do
    value = expression(rhs, env)
    t = value["t"]

    unless annotation == :none or type!(annotation, env.file) == t,
      do: foreign(node, "local annotation differs from its value")

    stmt =
      case Map.fetch(env.vars, name) do
        :error -> %{"k" => "let", "names" => [name], "rhs" => [value]}
        {:ok, ^t} -> %{"k" => "assign", "lhs" => [var(name, t)], "rhs" => [value]}
        _ -> foreign(node, "a rebinding changes the variable's type")
      end

    [
      Map.put(stmt, "pos", pos(env, location(node)))
      | block(rest, %{env | vars: Map.put(env.vars, name, t)})
    ]
  end

  defp block([node | _], _), do: foreign(node, "unsupported Gleam statement or control flow")

  defp tail({:block, _, stmts}, env), do: block(stmts, env)
  defp tail({:case, _, _, _} = node, env), do: conditional(node, env)

  defp tail({:tuple, _, items} = node, env) do
    values = Enum.map(items, &expression(&1, env))

    unless Enum.map(values, & &1["t"]) == env.result,
      do: foreign(node, "tuple result differs from the signature")

    [%{"k" => "return", "results" => values, "pos" => pos(env, location(node))}]
  end

  defp tail(node, env) do
    e = expression(node, env)
    unless e["t"] == env.result, do: foreign(node, "return value differs from its declared type")
    [%{"k" => "return", "results" => [e], "pos" => pos(env, location(node))}]
  end

  defp conditional({_, _, [subject], [a, b]} = node, env) do
    cond_expr = expression(subject, env)

    branches =
      for {:clause, [[{:pattern_variant, _, :none, n, [], false}]], :none, body} <- [a, b],
          do: {n, body}

    case {cond_expr["t"], Map.new(branches)} do
      {@bool, %{"True" => yes, "False" => no}} ->
        [
          %{
            "k" => "if",
            "cond" => cond_expr,
            "then" => tail(yes, env),
            "else" => tail(no, env),
            "pos" => pos(env, location(node))
          }
        ]

      _ ->
        foreign(node, "expected exhaustive True and False branches on a Bool")
    end
  end

  defp conditional(node, _), do: foreign(node, "unsupported case patterns")

  defp wrapped({:block, _, [{:expression, e}]}, env), do: wrapped(e, env)

  defp wrapped({:binary_operator, _, op, a, b} = node, env)
       when op in [:add_int, :sub_int, :mult_int] do
    x = expression(a, env)
    y = expression(b, env)
    unless x["t"] == @int and y["t"] == @int, do: foreign(node, "expected integer operands")
    %{"k" => "binop", "op" => @ops[op], "x" => x, "y" => y, "t" => @int}
  end

  defp wrapped({:negate_int, _, a} = node, env) do
    x = expression(a, env)
    unless x["t"] == @int, do: foreign(node, "expected an integer operand")
    %{"k" => "unop", "op" => "-", "x" => x, "t" => @int}
  end

  defp wrapped(node, _), do: foreign(node, "unsupported wrapped expression")

  defp var(name, t), do: %{"k" => "var", "name" => name, "t" => t}
  defp lit(value, t), do: %{"k" => "lit", "value" => value, "t" => t}

  defp expression({:block, _, [{:expression, inner}]}, env), do: expression(inner, env)

  defp expression({:variable, _, n}, _) when n in ["True", "False"],
    do: lit(String.downcase(n), @bool)

  defp expression({:variable, _, n} = node, env) do
    case Map.fetch(env.vars, n) do
      {:ok, t} -> var(n, t)
      :error -> foreign(node, "unknown local variable or unsupported constructor")
    end
  end

  defp expression({:int, _, raw} = node, _) do
    digits = String.replace(raw, "_", "")
    if byte_size(digits) > 20, do: foreign(node, "integer outside the signed 64-bit domain")

    case Integer.parse(digits) do
      {n, ""} when n >= @min and n <= @max -> lit(Integer.to_string(n), @int)
      _ -> foreign(node, "expected a signed 64-bit decimal integer")
    end
  end

  defp expression({:string, _, raw} = node, _) do
    case :glexer.unescape_string(raw) do
      {:ok, value} -> lit(JSON.encode!(value), @string)
      {:error, _} -> foreign(node, "invalid Gleam string escape")
    end
  end

  defp expression({:negate_bool, _, arg} = node, env) do
    value = expression(arg, env)
    unless value["t"] == @bool, do: foreign(node, "expected a Bool operand")
    %{"k" => "unop", "op" => "!", "x" => value, "t" => @bool}
  end

  defp expression({:binary_operator, _, op, lhs, rhs} = node, env) when is_map_key(@ops, op) do
    left = expression(lhs, env)
    right = expression(rhs, env)
    t = left["t"]
    unless t == right["t"], do: foreign(node, "operand types differ")

    result =
      cond do
        op in [:eq, :not_eq] ->
          @bool

        t == @int and op in [:lt_int, :lt_eq_int, :gt_int, :gt_eq_int] ->
          @bool

        t == @bool and op in [:and, :or] ->
          @bool

        t == @string and op == :concatenate ->
          @string

        t == @int and safe_arithmetic?(op, left, right) ->
          @int

        true ->
          foreign(
            node,
            "operator is outside the signed cir/1 domain or has incompatible operands"
          )
      end

    %{"k" => "binop", "op" => Map.fetch!(@ops, op), "x" => left, "y" => right, "t" => result}
  end

  defp expression({:call, _, {:variable, _, name}, fields} = node, env) do
    sig = Map.get(env.signatures, name)

    if is_nil(sig) or Map.has_key?(env.vars, name) or is_list(sig.result),
      do: foreign(node, "expected an unshadowed scalar local function")

    args =
      Enum.map(fields, fn
        {:unlabelled_field, arg} -> expression(arg, env)
        _ -> foreign(node, "labelled arguments are outside this reader")
      end)

    unless Enum.map(args, & &1["t"]) == Enum.map(sig.params, & &1["t"]),
      do: foreign(node, "arguments differ from the function signature")

    %{"k" => "call", "name" => sig.name, "args" => args, "t" => sig.result}
  end

  defp expression(node, _), do: foreign(node, "unsupported Gleam expression")

  # BEAM arithmetic is unbounded; Gleam also defines division by zero as zero.
  # Only use a CIR operator when both semantics agree on every admissible input.
  defp safe_arithmetic?(op, left, right) do
    a = if left["k"] == "lit", do: String.to_integer(left["value"])
    b = if right["k"] == "lit", do: String.to_integer(right["value"])

    case {op, a, b} do
      {:div_int, _, b} when is_integer(b) and b not in [0, -1] ->
        true

      {:remainder_int, _, b} when is_integer(b) and b != 0 ->
        true

      {op, a, b} when is_integer(a) and is_integer(b) ->
        value =
          case op do
            :add_int -> a + b
            :sub_int -> a - b
            :mult_int -> a * b
            :div_int when b != 0 -> div(a, b)
            _ -> nil
          end

        is_integer(value) and value >= @min and value <= @max

      _ ->
        false
    end
  end
end
