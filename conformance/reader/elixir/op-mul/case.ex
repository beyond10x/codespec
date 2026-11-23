defmodule Subject do
  @spec mul(integer(), integer()) :: integer()
  def mul(a, b) do
    <<result::signed-64>> = <<a * b::signed-64>>
    result
  end
end
