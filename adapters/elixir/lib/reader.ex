defmodule Codespec.ElixirReader do
  @moduledoc """
  Reads one Elixir module without expanding macros or loading the source.
  Signatures come from local specs over integer(), boolean() and String.t().
  The integer boundary is CIR's signed platform integer. Plain arithmetic
  needs an in-range result on its operands' whole interval. A signed-64
  bitstring conversion can give an operation explicit wrapping semantics.
  Unsupported statements retain their source text and position.
  """

  @int %{"k" => "int", "width" => "platform", "signed" => true}
  @bool %{"k" => "bool"}
  @string %{"k" => "string"}
  @min -9_223_372_036_854_775_808
  @max 9_223_372_036_854_775_807
  @domain {@min, @max}
  @max_bytes 262_144

  def features do
    ~w(type.int type.bool type.string expr.lit expr.var expr.call expr.binop expr.unop
       stmt.return stmt.if stmt.let stmt.assign op.add op.sub op.mul op.div op.rem
       op.eq op.ne op.lt op.le op.gt op.ge op.land op.lor unop.not unop.plus unop.neg
       sem.int.wrap sem.rem.sign)
  end

  def read(path, version) do
    {file, label} = source_file!(path)

    text = File.open!(file, [:read, :binary], &IO.binread(&1, @max_bytes + 1))

    unless is_binary(text) and byte_size(text) <= @max_bytes and String.valid?(text),
      do:
        raise(
          ArgumentError,
          "#{label}: expected UTF-8 Elixir source of at most #{@max_bytes} bytes"
        )

    ctx = %{file: label, lines: text |> String.split("\n") |> List.to_tuple()}
    # Scalar literals normally have no location in Elixir's quoted syntax.
    literal = fn
      :else, m ->
        {:ok, {:__codespec_else__, m, []}}

      key, m when is_atom(key) ->
        if key in [:do, :after, :rescue, :catch] or m[:format] == :keyword,
          do: {:ok, key},
          else: {:ok, {:__codespec_literal__, m, [key]}}

      x, m ->
        {:ok, {:__codespec_literal__, m, [x]}}
    end

    case Code.string_to_quoted(text,
           columns: true,
           token_metadata: true,
           literal_encoder: literal
         ) do
      {:ok, {:defmodule, meta, [{:__aliases__, _, parts}, [do: body]]} = ast}
      when is_list(parts) and parts != [] ->
        check_depth!(ast, 0, label)

        unless Enum.all?(
                 parts,
                 &(is_atom(&1) and Atom.to_string(&1) =~ ~r/\A[A-Z][A-Za-z0-9_]*\z/)
               ),
               do: invalid!(ctx, meta, "expected a static module alias")

        module = Enum.map_join(parts, ".", &Atom.to_string/1)

        if module in ["Kernel", "Elixir.Kernel"],
          do: invalid!(ctx, meta, "source redefines Kernel")

        name = Enum.map_join(parts, "_", &(Atom.to_string(&1) |> Macro.underscore()))
        build(ctx, name, module, body, meta[:end], version)

      {:ok, _} ->
        raise ArgumentError, "#{label}: expected exactly one module (defmodule)"

      {:error, {location, message, token}} ->
        line = if is_list(location), do: location[:line], else: location

        detail =
          case message do
            {before, after_token} -> [before, token, after_token]
            plain -> [plain, token]
          end

        raise ArgumentError, "#{label}:#{line}: #{IO.iodata_to_binary(detail)}"
    end
  rescue
    e in File.Error -> raise ArgumentError, Exception.message(e)
  end

  @doc false
  def source_file!(path) do
    case File.lstat(path) do
      {:ok, %{type: :regular}} ->
        {path, Path.basename(path)}

      {:ok, %{type: :directory}} ->
        # Non-recursive source directories and single-module Mix lib trees.
        files = sources(path, false)

        case files do
          [file] ->
            {file, Path.relative_to(file, path)}

          _ ->
            raise ArgumentError,
                  "#{path}: expected exactly one .ex source file, found #{length(files)}"
        end

      _ ->
        raise ArgumentError, "#{path}: expected an Elixir source file or directory"
    end
  end

  defp sources(dir, in_lib) do
    for name <- Enum.sort(File.ls!(dir)), reduce: [] do
      acc ->
        file = Path.join(dir, name)

        case File.lstat!(file).type do
          :directory when in_lib or name == "lib" ->
            sources(file, true) ++ acc

          :regular ->
            if Path.extname(name) == ".ex", do: [file | acc], else: acc

          :symlink when in_lib or name == "lib" ->
            raise ArgumentError, "#{file}: source path is a symbolic link"

          _ ->
            if Path.extname(name) == ".ex",
              do: raise(ArgumentError, "#{file}: expected a regular source file")

            acc
        end
    end
  end

  defp check_depth!(_, n, label) when n > 160,
    do: raise(ArgumentError, "#{label}: quoted syntax exceeds the nesting limit")

  defp check_depth!(t, n, label) when is_tuple(t),
    do: t |> Tuple.to_list() |> Enum.each(&check_depth!(&1, n + 1, label))

  defp check_depth!(xs, n, label) when is_list(xs),
    do: Enum.each(xs, &check_depth!(&1, n + 1, label))

  defp check_depth!(_, _, _), do: :ok

  defp block({:__block__, _, xs}), do: xs
  defp block(x), do: [x]
  defp invalid!(ctx, meta, msg), do: raise(ArgumentError, "#{pos(ctx, meta[:line] || 1)}: #{msg}")
  defp pos(ctx, line), do: "#{ctx.file}:#{line}"
  defp refuse(msg), do: throw({:foreign, msg})
  defp put_doc(map, doc) when is_binary(doc), do: Map.put(map, "doc", doc)
  defp put_doc(map, _), do: map

  defp build(ctx, name, module, body, end_meta, version) do
    init = %{specs: %{}, defs: [], doc: nil, pending_doc: nil}
    data = Enum.reduce(block(body), init, &declaration(&1, &2, ctx))
    defs = Enum.reverse(data.defs)

    if data.pending_doc != nil, do: raise(ArgumentError, "#{ctx.file}: @doc without a function")
    used = MapSet.new(defs, &{&1.name, length(&1.params)})

    if MapSet.size(used) != length(defs),
      do: raise(ArgumentError, "#{ctx.file}: duplicate function clause")

    for key <- Map.keys(data.specs),
        not MapSet.member?(used, key),
        do:
          raise(
            ArgumentError,
            "#{ctx.file}: @spec without a matching definition for #{elem(key, 0)}/#{elem(key, 1)}"
          )

    signatures =
      Map.new(defs, fn f ->
        key = {f.name, length(f.params)}

        {args, result} =
          Map.get(data.specs, key) ||
            invalid!(ctx, f.meta, "missing @spec for #{f.name}/#{length(f.params)}")

        {key, %{id: f.id, args: args, result: result}}
      end)

    if length(Enum.uniq_by(defs, & &1.id)) != length(defs),
      do:
        raise(
          ArgumentError,
          "#{ctx.file}: a function overload or name collision has no single cir/1 identity"
        )

    ctx = Map.put(ctx, :sigs, signatures)

    {functions, refusals} =
      Enum.map_reduce(defs, [], fn f, refused ->
        sig = Map.fetch!(signatures, {f.name, length(f.params)})
        env = Map.new(Enum.zip(f.params, sig.args), fn {n, t} -> {n, {t, range_for(t)}} end)
        limit = location(f.meta[:end] || f.meta[:end_of_expression] || end_meta) || eof(ctx)
        {stmts, rs} = statements(block(f.body), env, sig.result, ctx, limit)
        params = Enum.zip_with(f.params, sig.args, fn n, t -> %{"name" => n, "t" => t} end)

        fun = %{
          "name" => f.id,
          "exported" => f.exported,
          "params" => params,
          "results" => [sig.result],
          "body" => stmts,
          "pos" => pos(ctx, f.meta[:line])
        }

        {put_doc(fun, f.doc), [rs | refused]}
      end)

    doc = %{
      "format" => "cir/1",
      "name" => name,
      "path" => module,
      "reader" => %{
        "language" => "elixir",
        "name" => "codespec-read-elixir",
        "version" => version
      },
      "types" => [],
      "funcs" => functions,
      "layout" => Enum.map(defs, &("func:" <> &1.id))
    }

    refused = refusals |> Enum.reverse() |> List.flatten()
    doc = if refused == [], do: doc, else: Map.put(doc, "refusals", refused)
    put_doc(doc, data.doc)
  end

  defp declaration({:@, meta, [{attr, _, [{:__codespec_literal__, _, [v]}]}]}, acc, ctx)
       when attr in [:doc, :moduledoc] and (is_binary(v) or v == false) do
    key = if attr == :doc, do: :pending_doc, else: :doc
    if acc[key] != nil, do: invalid!(ctx, meta, "duplicate @#{attr}")
    Map.put(acc, key, v)
  end

  defp declaration({:@, meta, [{:spec, _, [{:"::", _, [{name, _, ts}, result]}]}]}, acc, ctx)
       when is_atom(name) and (is_list(ts) or is_nil(ts)) do
    ts = List.wrap(ts)
    key = {name, length(ts)}

    if Map.has_key?(acc.specs, key),
      do: invalid!(ctx, meta, "duplicate @spec for #{name}/#{length(ts)}")

    sig = {Enum.map(ts, &param_type!(&1, ctx, meta)), spec_type!(result, ctx, meta)}
    %{acc | specs: Map.put(acc.specs, key, sig)}
  end

  defp declaration({kind, meta, [{name, _, args}, [do: body]]}, acc, ctx)
       when kind in [:def, :defp] and is_atom(name) and (is_list(args) or is_nil(args)) do
    if name == :when, do: invalid!(ctx, meta, "function guards are outside the typed subset")
    spelling = Atom.to_string(name)

    if not valid_name?(spelling),
      do: invalid!(ctx, meta, "unsupported function name #{inspect(spelling)}")

    params =
      Enum.map(List.wrap(args), fn
        {n, _, nil} when is_atom(n) ->
          str = Atom.to_string(n)

          if not valid_name?(str),
            do: invalid!(ctx, meta, "unsupported parameter name #{inspect(str)}")

          str

        _ ->
          invalid!(ctx, meta, "parameters must be distinct variables")
      end)

    if Enum.uniq(params) != params,
      do: invalid!(ctx, meta, "parameters must be distinct variables")

    public = kind == :def
    id = if public, do: Macro.camelize(spelling), else: spelling
    if id == "", do: invalid!(ctx, meta, "function name has no cir/1 spelling")

    f = %{
      name: name,
      id: id,
      exported: public,
      params: params,
      body: body,
      meta: meta,
      doc: acc.pending_doc
    }

    %{acc | defs: [f | acc.defs], pending_doc: nil}
  end

  defp declaration(ast, _, ctx), do: invalid!(ctx, meta(ast), "unsupported module declaration")
  defp valid_name?(s), do: s =~ ~r/\A[a-z_][a-zA-Z0-9_]*\z/

  defp param_type!({:"::", _, [{name, _, nil}, t]}, ctx, m) when is_atom(name),
    do: spec_type!(t, ctx, m)

  defp param_type!(t, ctx, m), do: spec_type!(t, ctx, m)
  defp spec_type!({:integer, _, args}, _, _) when args in [nil, []], do: @int
  defp spec_type!({:boolean, _, args}, _, _) when args in [nil, []], do: @bool
  defp spec_type!({{:., _, [{:__aliases__, _, [:String]}, :t]}, _, []}, _, _), do: @string

  defp spec_type!(_, ctx, m),
    do: invalid!(ctx, m, "@spec type outside integer(), boolean() and String.t()")

  defp range_for(@int), do: @domain
  defp range_for(_), do: nil
  defp value(obj, t, range \\ nil), do: {Map.put(obj, "t", t), range}
  defp ir({obj, _}), do: obj
  defp type({%{"t" => t}, _}), do: t
  defp check!({%{"t" => t}, _} = v, t), do: v
  defp check!(_, _), do: refuse("expression type does not match its operand or declared result")

  defp statements([], _, _, ctx, {line, _}),
    do: invalid!(ctx, [line: line], "empty function or branch has no scalar result")

  defp statements(xs, env, expected, ctx, limit) do
    last = length(xs) - 1

    {pairs, _env} =
      xs
      |> Enum.with_index()
      |> Enum.map_reduce(env, fn {ast, i}, env ->
        start = start_at(ast, ctx) || limit
        finish = end_at(ast) || limit
        p = pos(ctx, elem(start, 0))

        try do
          {statements, new_env, rs} =
            statement(ast, env, if(i == last, do: expected), ctx, p, finish)

          {{statements, rs}, new_env}
        catch
          {:foreign, why} ->
            src = slice(ctx, start, finish)
            f = %{"k" => "foreign", "lang" => "elixir", "src" => src, "pos" => p, "reason" => why}
            r = %{"pos" => p, "what" => src, "reason" => why}
            {{[f], [r]}, %{}}
        end
      end)

    {stmts, rs} = Enum.unzip(pairs)
    {List.flatten(stmts), List.flatten(rs)}
  end

  defp statement(
         {:if, m, [condition, [{:do, yes}, {{:__codespec_else__, else_m, []}, no}]]},
         env,
         expected,
         ctx,
         p,
         finish
       )
       when expected != nil do
    c = expr(condition, env, ctx) |> check!(@bool)
    {y, yr} = statements(block(yes), env, expected, ctx, location(else_m))
    {n, nr} = statements(block(no), env, expected, ctx, location(m[:end]) || finish)
    {[%{"k" => "if", "cond" => ir(c), "then" => y, "else" => n, "pos" => p}], env, yr ++ nr}
  end

  defp statement({:=, _, [lhs, rhs]}, env, expected, ctx, p, _) do
    {name, v} =
      case lhs do
        {:<<>>, _, _} ->
          {var_name!(signed64!(lhs)), expr(signed64!(rhs), env, ctx, true) |> check!(@int)}

        _ ->
          {var_name!(lhs), expr(rhs, env, ctx)}
      end

    if name == "_", do: refuse("discard binding outside the typed subset")
    var = %{"k" => "var", "name" => name, "t" => type(v)}

    s =
      case env do
        %{^name => {old, _}} ->
          check!(v, old)
          %{"k" => "assign", "lhs" => [var], "rhs" => [ir(v)], "pos" => p}

        _ ->
          %{"k" => "let", "names" => [name], "rhs" => [ir(v)], "pos" => p}
      end

    result =
      if expected do
        # A scalar assignment yields the RHS. A bitstring match yields a binary.
        if match?({:<<>>, _, _}, lhs), do: refuse("a final bitstring match returns a binary")
        check!(v, expected)
        [%{"k" => "return", "results" => [var], "pos" => p}]
      else
        []
      end

    {[s | result], Map.put(env, name, {type(v), elem(v, 1)}), []}
  end

  defp statement(ast, env, nil, ctx, p, _) do
    v = expr(ast, env, ctx)
    if ir(v)["k"] != "call", do: refuse("unused expression outside the typed subset")
    {[%{"k" => "expr", "x" => ir(v), "pos" => p}], env, []}
  end

  defp statement(ast, env, expected, ctx, p, _) do
    v = expr(ast, env, ctx) |> check!(expected)
    {[%{"k" => "return", "results" => [ir(v)], "pos" => p}], env, []}
  end

  defp var_name!({n, _, nil}) when is_atom(n) do
    s = Atom.to_string(n)
    if valid_name?(s), do: s, else: refuse("variable name outside the typed subset")
  end

  defp var_name!(_), do: refuse("expected a variable binding")

  defp signed64!(
         {:<<>>, _,
          [{:"::", _, [x, {:-, _, [{:signed, _, nil}, {:__codespec_literal__, _, [64]}]}]}]}
       ),
       do: x

  defp signed64!(_), do: refuse("expected one signed-64 bitstring segment")

  defp expr(ast, env, ctx, wrap \\ false)
  defp expr({:__codespec_literal__, _, [n]}, _, _, _) when is_integer(n), do: int_lit(n)

  defp expr({:__codespec_literal__, _, [b]}, _, _, _) when is_boolean(b),
    do: value(%{"k" => "lit", "value" => to_string(b)}, @bool)

  defp expr({:__codespec_literal__, _, [s]}, _, _, _) when is_binary(s) do
    if not String.valid?(s), do: refuse("binary literal is not UTF-8")
    value(%{"k" => "lit", "value" => JSON.encode!(s)}, @string)
  end

  defp expr({:-, _, [{:__codespec_literal__, _, [n]}]}, _, _, _) when is_integer(n),
    do: int_lit(-n)

  defp expr({:case, _, [bits, [do: [{:->, _, [[pat], ret]}]]]}, env, ctx, _) do
    name = var_name!(signed64!(pat))

    if name == "_" or var_name!(ret) != name,
      do: refuse("signed-64 case must return its matched value")

    expr(signed64!(bits), env, ctx, true) |> check!(@int)
  end

  defp expr({n, _, nil}, env, ctx, _) when is_atom(n) do
    name = Atom.to_string(n)

    case Map.fetch(env, name) do
      {:ok, {t, r}} -> value(%{"k" => "var", "name" => name}, t, r)
      :error -> call(n, [], env, ctx)
    end
  end

  defp expr({op, _, [arg]}, env, ctx, wrap) when op in [:+, :-, :not, :!] do
    x = expr(arg, env, ctx)

    {operator, t, r} =
      case op do
        op when op in [:not, :!] ->
          check!(x, @bool)
          {"!", @bool, nil}

        :+ ->
          check!(x, @int)
          {"+", @int, elem(x, 1)}

        :- ->
          {_, {lo, hi}} = check!(x, @int)
          {"-", @int, bounded({-hi, -lo}, wrap)}
      end

    value(%{"k" => "unop", "op" => operator, "x" => ir(x)}, t, r)
  end

  defp expr({{:., _, [{:__aliases__, _, [:Kernel]}, op]}, _, args}, env, ctx, wrap)
       when op in [:div, :rem] and length(args) == 2,
       do: binary(op, args, env, ctx, wrap)

  defp expr({name, _, args}, env, ctx, wrap) when is_atom(name) and is_list(args) do
    if Map.has_key?(ctx.sigs, {name, length(args)}) do
      call(name, args, env, ctx)
    else
      binary(name, args, env, ctx, wrap)
    end
  end

  defp expr(_, _, _, _), do: refuse("expression outside the typed Elixir subset")

  defp call(name, args, env, ctx) do
    sig =
      Map.get(ctx.sigs, {name, length(args)}) ||
        refuse("unknown variable or local call #{name}/#{length(args)}")

    values =
      Enum.zip_with(args, sig.args, fn arg, t -> expr(arg, env, ctx) |> check!(t) |> ir() end)

    value(%{"k" => "call", "name" => sig.id, "args" => values}, sig.result, range_for(sig.result))
  end

  @comparisons %{==: "==", ===: "==", !=: "!=", !==: "!=", <: "<", <=: "<=", >: ">", >=: ">="}
  @booleans %{and: "&&", &&: "&&", or: "||", ||: "||"}
  @arithmetic %{+: "+", -: "-", *: "*", div: "/", rem: "%"}

  defp binary(op, [a, b], env, ctx, wrap)
       when is_map_key(@comparisons, op) or is_map_key(@booleans, op) or
              is_map_key(@arithmetic, op) or op == :<> do
    x = expr(a, env, ctx)
    y = expr(b, env, ctx) |> check!(type(x))

    {spelling, t, r} =
      cond do
        Map.has_key?(@comparisons, op) ->
          if type(x) == @bool and op not in [:==, :===, :!=, :!==],
            do: refuse("boolean ordering outside the typed subset")

          {Map.fetch!(@comparisons, op), @bool, nil}

        Map.has_key?(@booleans, op) ->
          check!(x, @bool)
          {Map.fetch!(@booleans, op), @bool, nil}

        op == :<> ->
          check!(x, @string)
          {"+", @string, nil}

        true ->
          check!(x, @int)

          {Map.fetch!(@arithmetic, op), @int,
           bounded(result_range(op, elem(x, 1), elem(y, 1)), wrap)}
      end

    value(%{"k" => "binop", "op" => spelling, "x" => ir(x), "y" => ir(y)}, t, r)
  end

  defp binary(_, _, _, _, _), do: refuse("expression or call outside the typed Elixir subset")

  defp int_lit(n) when n >= @min and n <= @max,
    do: value(%{"k" => "lit", "value" => Integer.to_string(n)}, @int, {n, n})

  defp int_lit(_), do: refuse("integer literal exceeds the signed 64-bit domain")

  # Elixir integers have no overflow. Only emit a plain operation when its entire
  # result interval fits CIR. An explicit signed bitstring conversion permits wrap
  # at that one operation; it says nothing about the operations in its operands.
  defp bounded({lo, hi} = r, _) when lo >= @min and hi <= @max, do: r
  defp bounded(_, true), do: @domain

  defp bounded(_, false),
    do: refuse("Elixir integer result can exceed signed 64 bits without explicit wrapping")

  defp result_range(:+, {a, b}, {c, d}), do: {a + c, b + d}
  defp result_range(:-, {a, b}, {c, d}), do: {a - d, b - c}
  defp result_range(:*, {a, b}, {c, d}), do: Enum.min_max([a * c, a * d, b * c, b * d])

  defp result_range(op, _, {0, 0}) when op in [:div, :rem],
    do: refuse("division or remainder by literal zero")

  defp result_range(:div, {a, b}, {c, d}) do
    divisors = Enum.filter([c, d, -1, 1], &(&1 >= c and &1 <= d and &1 != 0))
    Enum.min_max(for n <- [a, b], m <- divisors, do: div(n, m))
  end

  defp result_range(:rem, {a, b}, {c, d}) do
    m = max(abs(c), abs(d)) - 1
    {max(min(a, 0), -m), min(max(b, 0), m)}
  end

  defp meta({_, m, _}) when is_list(m), do: m
  defp meta(_), do: []
  defp location(m) when is_list(m), do: if(m[:line] && m[:column], do: {m[:line], m[:column]})
  defp location(_), do: nil

  # An infix operator's location is the operator; a remote call's is its last
  # identifier. The first token belongs to the earliest located child.
  defp start_at(ast, ctx) do
    {_, start} =
      Macro.prewalk(ast, nil, fn node, acc ->
        m = meta(node)

        parens =
          for group <- Keyword.get_values(m, :parens),
              {line, col} <- [location(group)],
              String.at(elem(ctx.lines, line - 1), col - 1) == "(",
              do: {line, col}

        {node, Enum.min(Enum.reject([acc, location(m) | parens], &is_nil/1), fn -> nil end)}
      end)

    start
  end

  defp end_at(ast) do
    m = meta(ast)

    location(m[:end_of_expression]) ||
      case location(m[:end]) do
        {line, col} -> {line, col + 3}
        _ -> nil
      end
  end

  defp eof(ctx) do
    n = tuple_size(ctx.lines)
    {n, String.length(elem(ctx.lines, n - 1)) + 1}
  end

  defp slice(ctx, {a, c}, {b, d}) do
    for i <- a..b do
      s = elem(ctx.lines, i - 1)
      left = if i == a, do: c - 1, else: 0
      right = if i == b, do: d - 1, else: String.length(s)
      String.slice(s, left, max(0, right - left))
    end
    |> Enum.join("\n")
    |> String.trim_trailing()
    |> String.trim_trailing(",")
    |> String.trim_trailing()
  end
end
