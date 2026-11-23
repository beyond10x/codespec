defmodule Subject do
  @spec neither(boolean(), boolean()) :: boolean()
  def neither(a, b) do
    not a and not b
  end
end
