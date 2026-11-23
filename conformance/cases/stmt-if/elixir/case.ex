defmodule Subject do
  @spec sign(integer()) :: integer()
  def sign(n) do
    if n > 0 do
      1
    else
      if n < 0, do: 2, else: 0
    end
  end
end
