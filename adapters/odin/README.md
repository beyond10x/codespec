# Odin adapter

The reader uses `core:odin/parser` on a file or package. It lifts free procedures with signed integer, boolean and string values. It collects signatures before lowering bodies, so a call may refer to a later procedure or a procedure in another file.

For signed division, the minimum integer divided by −1 wraps in `cir/1`. The reader accepts a safe literal divisor or a branch that rules out −1. Assigning to the divisor invalidates the branch condition. [Guarded division fixture](../../conformance/cases/op-div/odin/subject.odin)

The writer emits `module.odin` and handles the minimum-value division and remainder cases explicitly. The runner asks the sibling reader for exported signatures and uses the Odin compiler to build and run a driver against the package. [Translation guide](../../website/docs/guides/elixir-gleam-odin.md)
