defmodule Subject do
  @spec twice(integer()) :: integer()
  defp twice(n) do
    <<result::signed-64>> = <<n + n::signed-64>>
    result
  end

  @spec quad(integer()) :: integer()
  def quad(n) do
    twice(twice(n))
  end
end
