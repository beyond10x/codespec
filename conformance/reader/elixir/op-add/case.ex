defmodule Subject do
  @spec add(integer(), integer()) :: integer()
  def add(a, b) do
    <<result::signed-64>> = <<a + b::signed-64>>
    result
  end
end
