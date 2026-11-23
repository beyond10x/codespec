defmodule Codespec.GleamSource do
  @moduledoc false

  # A cir/1 document represents one source module. A project may also be read
  # by passing one src module explicitly.
  def file!(path) do
    case File.lstat(path) do
      {:ok, %{type: :regular}} ->
        if Path.extname(path) != ".gleam", do: raise(ArgumentError, "expected a .gleam source")
        path

      {:ok, %{type: :directory}} ->
        src = Path.join(path, "src")
        root = if File.dir?(src), do: src, else: path

        case modules(root, root == src) do
          [file] -> file
          [] -> raise ArgumentError, "no .gleam source module"
          _ -> raise ArgumentError, "multiple Gleam modules: pass one .gleam file"
        end

      _ ->
        raise ArgumentError, "expected a regular .gleam file or source directory"
    end
  end

  defp modules(root, recursive) do
    unless File.lstat!(root).type == :directory,
      do: raise(ArgumentError, "invalid Gleam source directory")

    root
    |> File.ls!()
    |> Enum.sort()
    |> Enum.flat_map(fn name ->
      path = Path.join(root, name)

      case {File.lstat!(path).type, Path.extname(path)} do
        {:directory, _} when recursive -> modules(path, true)
        {:regular, ".gleam"} -> [path]
        {_, ".gleam"} -> raise ArgumentError, "expected regular source file #{name}"
        _ -> []
      end
    end)
  end
end
