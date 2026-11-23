defmodule Subject do
  @spec add(integer(), integer()) :: integer()
  def add(a, b) do
    <<result::signed-64>> = <<a + b::signed-64>>
    result
  end

  @spec sub(integer(), integer()) :: integer()
  def sub(a, b) do
    <<result::signed-64>> = <<a - b::signed-64>>
    result
  end

  @spec mul(integer(), integer()) :: integer()
  def mul(a, b) do
    <<result::signed-64>> = <<a * b::signed-64>>
    result
  end

  @spec neg(integer()) :: integer()
  def neg(n) do
    <<result::signed-64>> = <<-n::signed-64>>
    result
  end
end
