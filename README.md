# Multigrind: Squads V4 Multisig Grinder

### What is it?
Vanity keypairs are of interest to many. Standard ed25519 keypairs cannot be grinded out on behalf of users in a trustless manner because grinding it gives the producer access to the secret key (Threshold signatures are an exception to this but we leave this story for a later time).

Squads v4 multisig pubkeys are based on a `create_key` pubkey that serves as a nonce. This means that a multisig address can be grinded out to obtain a target prefix. Then, since multisigs offer the ability to manage members, it is possible to atomically transfer ownership and collect payment for the multisig. As such, this program intends to be a rudimentary vanity multisig marketplace.

Fun fact: experiments on Apple M2 Max show that griding multisig addresses by searching for an offcurve `create_key` signer (e.g. auxiliary pda) is ≈20-25% faster than generating ed25519 keypairs.

### What's in this repo?
1. A smart contract implementation

2.  Off-chain address grinder.
##### Contract
The contract is found in `auxiliary`. It contains two instructions. One initializes an auxiliary account which manages the multisig. The other allows a buyer to buy a listed multisig. **THIS CONTRACT IS CURRENTLY UNAUDITED. USE AT YOUR OWN RISK. THE AUTHOR IS NOT LIABLE FOR ANY LOSS OF FUNDS OR EXPLOITS DISCOVERED.**

##### Grinder
The grinder is found in `grind`. Presently, only a small fixed set of targets are searched for. This can be expanded. We leave this as an exercise to the reader. In `grind/src/bin/pda.rs`, you must be careful about whether you are grinding keys for the staging or prod squads v4 program.

To grind:
`cargo run --release --bin pda`

TODO/NOTE: The auxiliary program used by this grinder will most definitely not be the one corresponding to the test keypair `AuxokT3REMom8yP5TvuJQaUQUjtkHooQ48hTSQZiYd7W.json` that can be found at the root of the repo.


