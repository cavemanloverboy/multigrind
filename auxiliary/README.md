# Auxiliary Contract

This contract carries out two instructions

- **Initialize multisig:** Given a 16-byte seed (which you've presumably grinded out to produce a neat vanity address), initializes an auxiliary account owned by this program and a Squads V4 multisig that is controlled by the auxiliary account. This means that only the auxiliary account can carry out operations with the multisig, which is what happens in the **Sell multisig** instruction.
- **Sell multisig** A buyer can buy a multisig that is owned by an auxiliary program account. This does a lot in a single atomic instruction:
  - Adds the buyer as a multisig member
  - Removes the auxiliary account as a `config_authority`
  - Creates a Squads V4 `ConfigTransaction` to remove the auxiliary pda as a member
  - Creates and activates a Squads V4 `Proposal` proposing the aforementioned config transaction
  - Auxiliary account votes to approve the proposal
  - Auxiliary account executes the transaction to remove itself as a member
  - Collect SOL payment from buyer.

  When all is said and done, the buyer is the sole member of the 1/1 Squads V4 multisig and can do as they wish.



**NOTES**
1) THIS CONTRACT IS CURRENTLY UNAUDITED. USE AT YOUR OWN RISK. THE AUTHOR IS NOT LIABLE FOR ANY LOSS OF FUNDS OR EXPLOITS DISCOVERED.
2) This contract is unfinished because some design choices have to be made by the deployer. The choices that need to be made are relevant to whether this will be a permissioned (one or several approved grinders/sellers), or fully permissionless. This affects a few points in the code which are all documented with `TODO`.
3) This contract uses the squads v4 staging contract `STAG3...` on mainnet. Once the production contract is live, all references to `STAG3...` need to be replaced with the production contract `SQDS4...`.
  
