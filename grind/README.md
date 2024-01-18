# Grind

Offchain program for grinding auxiliary program pdas that result in a multisig pda with the targets defined in `TARGETS` in `src/lib.rs`.

It may be better to do this via a `include_str!` or a dynamic target management system that acceps requests and removes them if a seed was found.

This is intended to be a reference implementation. The happy path of the tight loop (not finding a key, lol) is allocation free. Parallelism can be obtained by running multiple instances of the grinder, although I have not tested `logfather` behavior in this case (it may overlap writes in a weird way). We leave other features such as handling case insensitivity and target suffixes as an exercise to the reader. In `src/bin/pda.rs`, you must be careful about whether you are grinding keys for the staging or prod squads v4 program.

TODO/NOTE: The auxiliary program used by this grinder will most definitely not be the one corresponding to the test keypair `AuxokT3REMom8yP5TvuJQaUQUjtkHooQ48hTSQZiYd7W.json` that can be found at the root of the repo.