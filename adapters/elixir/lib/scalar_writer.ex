defmodule Codespec.ScalarWriter do
  @moduledoc """
  The scalar subset shared by the Elixir and Gleam writers. Checks every
  declaration, resolves names and types, and turns CIR control flow into
  expression-valued blocks before either backend writes a file.
  """

  @unsupported ~w(types consts errors vars externs refusals opaque methods linknames inits
                  interfaces entries beans injects transactional persistence)
  @module_fields ~w(format name reader doc funcs layout facts path left_out) ++ @unsupported
  @integer_ops ~w(+ - * / % & | ^ &^ << >>)
  @compound Enum.map(@integer_ops, &(&1 <> "="))
  @widths %{"8" => 8, "16" => 16, "32" => 32, "64" => 64, "platform" => 64}
  @signed_min -9_223_372_036_854_775_808
  @signed_max 9_223_372_036_854_775_807

  def compile(%{"format" => "cir/1"} = module, backend) do
    unless is_map(module["facts"]), do: refuse("module has no facts; run codespec pass first")

    case Map.keys(module) -- @module_fields do
      [] -> :ok
      fields -> refuse("unknown module fields #{inspect(Enum.sort(fields))}")
    end

    for key <- @unsupported,
        Map.get(module, key, []) != [],
        do: refuse("unsupported module #{key}")

    name = native(module["name"])
    signatures = Enum.map(module["funcs"], &signature(&1, backend))
    function_names = Enum.map(signatures, & &1.name)
    unique!(function_names, "function names")
    # Gleam resolves a local name before a function, including inside a
    # continuation placed in a branch. Reserve all module function names
    # when allocating locals, before rearranging any control flow.
    occupied =
      if backend == Codespec.GleamWriter, do: MapSet.new(function_names), else: MapSet.new()

    functions =
      Map.new(Enum.zip(module["funcs"], signatures), fn {f, sig} -> {f["name"], sig} end)

    compiled =
      Enum.zip_with(module["funcs"], signatures, fn f, sig ->
        state = %{
          vars: %{},
          functions: functions,
          result: sig.result,
          backend: backend,
          occupied: occupied
        }

        params = Enum.map(sig.params, fn {id, t} -> {local_name(id, occupied), t} end)

        env =
          Enum.zip(f["params"], params)
          |> Enum.reduce(state, fn {p, {id, t}}, e -> bind(e, p["name"], id, t) end)

        tail = if sig.result == :unit, do: :unit, else: :missing_return
        {body, used} = live(block(f["body"], env, tail))

        params =
          Enum.map(params, fn {id, t} ->
            {if(MapSet.member?(used, id), do: id, else: "_" <> id), t}
          end)

        sig |> Map.replace!(:params, params) |> Map.put(:body, body)
      end)

    {name, compiled}
  end

  def compile(_, _), do: refuse("expected cir/1")

  def existing_module?(beam_name) do
    :code.where_is_file(String.to_charlist(beam_name <> ".beam")) != :non_existing
  end

  defp signature(f, backend) do
    if f["recv"] != nil or f["tparams"] not in [nil, []] or f["raises"] not in [nil, []],
      do: refuse("unsupported receiver, type parameters or raises in #{inspect(f["name"])}")

    params =
      Enum.map(f["params"], fn p ->
        if p["variadic"], do: refuse("variadic parameter")
        {identifier(p["name"], backend, :variable), scalar_type(p["t"])}
      end)

    %{
      name: identifier(f["name"], backend, :function),
      public: Map.fetch!(f, "exported"),
      params: params,
      result: results(Enum.map(f["results"], &scalar_type/1))
    }
  end

  defp native(name) do
    unless is_binary(name) and byte_size(name) <= 200 and
             Regex.match?(~r/\A[A-Za-z][A-Za-z0-9_]*\z/, name),
           do: refuse("unsupported identifier #{inspect(name)}")

    Macro.underscore(name)
  end

  defp identifier(name, backend, kind) do
    native =
      if kind == :variable and is_binary(name) and String.starts_with?(name, "_"),
        do: native("local" <> name),
        else: native(name)

    if backend.reserved?(native, kind), do: refuse("reserved #{kind} name #{inspect(name)}")
    native
  end

  # An unused binding still evaluates its right-hand side. Discarding only
  # its name also keeps compiled output free of unused-variable warnings.
  defp live({:bind, names, rhs, rest}) do
    {rest, after_names} = live(rest)
    {rhs, read} = live(rhs)
    kept = Enum.map(names, &if(MapSet.member?(after_names, &1), do: &1, else: "_"))
    before_names = Enum.reduce(names, after_names, &MapSet.delete(&2, &1))
    {{:bind, kept, rhs, rest}, MapSet.union(read, before_names)}
  end

  defp live({:var, id} = var), do: {var, MapSet.new([id])}

  defp live(xs) when is_list(xs) do
    Enum.map_reduce(xs, MapSet.new(), fn x, used ->
      {x, reads} = live(x)
      {x, MapSet.union(used, reads)}
    end)
  end

  defp live(node) when is_tuple(node) do
    {parts, used} = live(Tuple.to_list(node))
    {List.to_tuple(parts), used}
  end

  defp live(node), do: {node, MapSet.new()}

  defp unique!(names, what) do
    if length(names) != MapSet.size(MapSet.new(names)), do: refuse("#{what} collide")
  end

  defp local_name(id, occupied) do
    if MapSet.member?(occupied, id), do: "codespec_local_" <> id, else: id
  end

  defp bind(env, name, id, t) do
    if Enum.any?(env.vars, fn {_, {v, _}} -> v == id end),
      do: refuse("local names collide or shadow a binding: #{inspect(name)}")

    %{env | vars: Map.put(env.vars, name, {id, t})}
  end

  defp scalar_type(%{"k" => "int", "signed" => true, "width" => w}) when is_map_key(@widths, w),
    do: {:int, Map.fetch!(@widths, w)}

  defp scalar_type(%{"k" => "bool"}), do: :bool
  defp scalar_type(%{"k" => "string"}), do: :string
  defp scalar_type(t), do: refuse("unsupported type #{inspect(t)}")

  defp type(%{"k" => "tuple", "elems" => ts}), do: results(Enum.map(ts, &scalar_type/1))
  defp type(%{"k" => "unit"}), do: :unit
  defp type(t), do: scalar_type(t)

  defp results([]), do: :unit
  defp results([t]), do: t
  defp results(ts), do: {:tuple, ts}

  defp values([]), do: :unit
  defp values([x]), do: x
  defp values(xs), do: {:tuple, xs}

  defp destructure({:tuple, types}), do: types
  defp destructure(t), do: [t]

  defp same!(a, b) do
    if a != b, do: refuse("type mismatch: #{inspect(a)} and #{inspect(b)}")
  end

  defp block([], _, :missing_return), do: refuse("a path reaches the end without a result")
  defp block([], _, tail), do: tail

  defp block([%{"k" => "return", "results" => xs} | rest], env, _) do
    if rest != [], do: refuse("statements after an unconditional return")
    {t, value} = expressions(xs, env)
    same!(t, env.result)
    value
  end

  defp block([%{"k" => "if"} = s | rest], env, finish) do
    {cond_type, cond} = expression(s["cond"], env)
    same!(cond_type, :bool)
    yes = s["then"]
    no = Map.get(s, "else", [])
    yes_returns = returns?(yes)
    no_returns = returns?(no)

    cond do
      yes_returns and no_returns ->
        if rest != [], do: refuse("statements after an unconditional return")
        {:if, cond, block(yes, env, :missing_return), block(no, env, :missing_return)}

      yes_returns or no_returns ->
        next = block(rest, env, finish)
        {:if, cond, block(yes, env, next), block(no, env, next)}

      true ->
        # Branch-local declarations go out of scope. Each branch returns any
        # outer locals it updates, so rebinding them survives the branch.
        if has_return?(yes) or has_return?(no),
          do: refuse("two falling-through branches with an embedded early return")

        names =
          (assigned(yes) ++ assigned(no))
          |> Enum.uniq()
          |> Enum.filter(&Map.has_key?(env.vars, &1))
          |> Enum.map(fn n -> elem(env.vars[n], 0) end)

        saved = values(Enum.map(names, &{:var, &1}))
        branch = {:if, cond, block(yes, env, saved), block(no, env, saved)}
        {:bind, names, branch, block(rest, env, finish)}
    end
  end

  defp block([%{"k" => "let", "names" => names, "rhs" => xs} | rest], env, finish) do
    {t, rhs} = expressions(xs, env)
    types = destructure(t)
    if names == [] or length(types) != length(names), do: refuse("let arity mismatch")

    {ids, env} =
      Enum.map_reduce(Enum.zip(names, types), env, fn
        {"_", _}, e ->
          {"_", e}

        {name, kind}, e ->
          id = local_name(identifier(name, e.backend, :variable), e.occupied)
          {id, bind(e, name, id, kind)}
      end)

    {:bind, ids, rhs, block(rest, env, finish)}
  end

  defp block([%{"k" => "var", "name" => name, "t" => t} | rest], env, finish) do
    kind = scalar_type(t)
    id = local_name(identifier(name, env.backend, :variable), env.occupied)
    {:bind, [id], zero(kind), block(rest, bind(env, name, id, kind), finish)}
  end

  defp block([%{"k" => "assign", "lhs" => lhs, "rhs" => xs} = s | rest], env, finish) do
    if lhs == [], do: refuse("assignment requires a target")
    pairs = Enum.map(lhs, &target(&1, env))
    {lhs_types, ids} = Enum.unzip(pairs)
    unique!(ids, "assignment targets")
    {rhs_type, rhs} = expressions(xs, env)

    value =
      case {Map.get(s, "op", ""), pairs} do
        {"", _} ->
          same!(results(lhs_types), rhs_type)
          rhs

        {op, [{t, id}]} when op in @compound ->
          binary(String.slice(op, 0..-2//1), t, {:var, id}, rhs_type, rhs, t)

        _ ->
          refuse("unsupported compound assignment #{inspect(s["op"])}")
      end

    {:bind, ids, value, block(rest, env, finish)}
  end

  defp block([%{"k" => k, "x" => x} | rest], env, finish) when k in ["incr", "decr"] do
    {t, id} = target(x, env)
    unless match?({:int, _}, t), do: refuse("#{k} requires an integer variable")
    op = if k == "incr", do: "+", else: "-"
    {:bind, [id], binary(op, t, {:var, id}, t, {:lit, 1}, t), block(rest, env, finish)}
  end

  defp block([%{"k" => "expr", "x" => x} | rest], env, finish) do
    {_, val} = expression(x, env)
    {:bind, [], val, block(rest, env, finish)}
  end

  defp block([s | _], _, _),
    do: refuse("unsupported statement #{inspect(s["k"])} at #{inspect(s["pos"])}")

  defp returns?(ss) do
    case List.last(ss) do
      %{"k" => "return"} -> true
      %{"k" => "if", "then" => yes, "else" => no} -> returns?(yes) and returns?(no)
      _ -> false
    end
  end

  defp has_return?(ss) do
    Enum.any?(ss, fn
      %{"k" => "return"} -> true
      %{"k" => "if"} = s -> has_return?(s["then"]) or has_return?(Map.get(s, "else", []))
      _ -> false
    end)
  end

  defp assigned(ss) do
    Enum.flat_map(ss, fn
      %{"k" => "assign", "lhs" => lhs} -> Enum.map(lhs, & &1["name"])
      %{"k" => k, "x" => x} when k in ["incr", "decr"] -> [x["name"]]
      %{"k" => "if"} = s -> assigned(s["then"]) ++ assigned(Map.get(s, "else", []))
      _ -> []
    end)
  end

  defp target(%{"k" => "var", "name" => name, "t" => t}, env) do
    case Map.fetch(env.vars, name) do
      {:ok, {id, actual}} ->
        same!(actual, type(t))
        {actual, id}

      :error ->
        refuse("undeclared variable #{inspect(name)}")
    end
  end

  defp target(_, _), do: refuse("assignment target must be a local variable")

  defp expressions(xs, env) do
    {ts, values} = xs |> Enum.map(&expression(&1, env)) |> Enum.unzip()
    {results(ts), values(values)}
  end

  defp expression(e, env) do
    t = type(e["t"])
    {t, term(e, t, env)}
  end

  defp term(%{"k" => "var"} = x, _, env), do: {:var, elem(target(x, env), 1)}
  defp term(%{"k" => "zero"}, t, _), do: zero(t)
  defp term(%{"k" => "lit", "value" => val}, t, _), do: literal(val, t)

  defp term(%{"k" => "binop", "op" => op, "x" => x, "y" => y}, t, env) do
    {xt, a} = expression(x, env)
    {yt, b} = expression(y, env)
    binary(op, xt, a, yt, b, t)
  end

  defp term(%{"k" => "unop", "op" => op, "x" => x}, t, env) do
    {xt, a} = expression(x, env)
    same!(xt, t)

    unless (op == "!" and t == :bool) or (op in ["+", "-", "^"] and match?({:int, _}, t)),
      do: refuse("unsupported unary #{inspect(op)} on #{inspect(t)}")

    {:unary, op, a, t}
  end

  defp term(%{"k" => "call", "name" => name, "args" => xs} = e, t, env) do
    if e["targs"] not in [nil, []], do: refuse("generic call")
    f = env.functions[name] || refuse("undeclared function #{inspect(name)}")

    {types, args} = xs |> Enum.map(&expression(&1, env)) |> Enum.unzip()
    same!(types, Enum.map(f.params, &elem(&1, 1)))
    same!(t, f.result)
    {:call, f.name, args}
  end

  defp term(%{"k" => k}, _, _), do: refuse("unsupported expression #{inspect(k)}")

  defp binary(op, xt, a, yt, b, result) do
    valid =
      cond do
        op in ["==", "!="] ->
          xt == yt and result == :bool

        op in ["<", "<=", ">", ">="] ->
          xt == yt and result == :bool and (xt == :string or match?({:int, _}, xt))

        op in ["&&", "||"] ->
          xt == :bool and yt == :bool and result == :bool

        op == "+" and xt == :string ->
          yt == :string and result == :string

        op in ["<<", ">>"] ->
          match?({:int, _}, xt) and match?({:int, _}, yt) and result == xt

        op in @integer_ops ->
          match?({:int, _}, xt) and xt == yt and result == xt

        true ->
          false
      end

    unless valid,
      do: refuse("unsupported operator #{inspect(op)} for #{inspect({xt, yt, result})}")

    {:binary, op, a, b, xt}
  end

  defp zero({:int, _}), do: {:lit, 0}
  defp zero(:bool), do: {:lit, false}
  defp zero(:string), do: {:lit, ""}
  defp zero(t), do: refuse("unsupported zero #{inspect(t)}")

  defp literal(value, {:int, bits}) do
    unless is_binary(value) and byte_size(value) <= 20 and
             Regex.match?(~r/\A-?(?:0|[1-9][0-9]*)\z/, value),
           do: refuse("expected a decimal integer literal")

    case Integer.parse(value) do
      {n, ""} when n >= @signed_min and n <= @signed_max ->
        half = Integer.pow(2, bits - 1)
        if n < -half or n >= half, do: refuse("literal outside its integer width")
        {:lit, n}

      _ ->
        refuse("unsupported integer spelling #{inspect(value)}")
    end
  end

  defp literal("true", :bool), do: {:lit, true}
  defp literal("false", :bool), do: {:lit, false}

  defp literal(value, :string) do
    case JSON.decode(value) do
      {:ok, s} when is_binary(s) -> {:lit, s}
      _ -> refuse("unsupported string literal spelling")
    end
  end

  defp literal(_, t), do: refuse("unsupported literal for #{inspect(t)}")
  defp refuse(message), do: raise(ArgumentError, message)
end
