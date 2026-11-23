defmodule Subject do
  @spec add(integer(), integer()) :: integer()
  def add(a, b) do
    <<result::signed-64>> = <<a + b::signed-64>>
    result
  end

  @spec mul(integer(), integer()) :: integer()
  def mul(a, b) do
    <<result::signed-64>> = <<a * b::signed-64>>
    result
  end

  @spec div(integer(), integer()) :: integer()
  def div(a, b) do
    <<result::signed-64>> = <<Kernel.div(a, b)::signed-64>>
    result
  end

  @spec rem(integer(), integer()) :: integer()
  def rem(a, b) do
    Kernel.rem(a, b)
  end

  @spec neg(integer()) :: integer()
  def neg(n) do
    <<result::signed-64>> = <<-n::signed-64>>
    result
  end
end
