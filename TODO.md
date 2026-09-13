# TODO

Poor man's issue tracker, as brenn's is: an entry here and a `TODO(slug)`
comment at the spot the work happens. The slugs are the join key.

## walk-peer-calls

Brenn's peer-call surface is unwalked from out of tree. Nothing here declares
a `call` port, answers one with a `sync` port, takes `requires = [calls]`, uses
the generated `CallPort` handle, or holds the acyclicity refusal as a fit
fixture. Which component would call which, and why, is its own design; the
0.23 migration adapted to the cut without taking the feature.
