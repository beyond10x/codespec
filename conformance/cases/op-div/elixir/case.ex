defmodule Subject do
  @spec div(integer(), integer()) :: integer()
  def div(a, b) do
    <<result::signed-64>> = <<Kernel.div(a, b)::signed-64>>
    result
  end
end
