defmodule Subject do
  @spec sub(integer(), integer()) :: integer()
  def sub(a, b) do
    <<result::signed-64>> = <<a - b::signed-64>>
    result
  end
end
