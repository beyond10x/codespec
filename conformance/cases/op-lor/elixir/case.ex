defmodule Subject do
  @spec either(boolean(), boolean()) :: boolean()
  def either(a, b) do
    a or b
  end
end
