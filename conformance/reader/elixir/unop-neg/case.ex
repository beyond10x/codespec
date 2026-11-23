defmodule Subject do
  @spec neg(integer()) :: integer()
  def neg(n) do
    <<result::signed-64>> = <<-n::signed-64>>
    result
  end
end
