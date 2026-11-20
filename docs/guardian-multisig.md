# Guardian multisig

The governance contract stores the guardian as a single `Address`. `veto()` only
checks `caller == config.guardian` and `caller.require_auth()`, so the guardian
can be any address whose authorization needs several people. There are two
ways to set that up.

## Option A: a Stellar account with signer weights (2-of-3)

A classic Stellar account satisfies `require_auth()` when the transaction's
signatures meet the account's **medium** threshold. Give three signers weight 1,
set the thresholds to 2, and drop the master key's weight to 0 so no single key
can act alone.

```bash
# Keys for the guardian account and its three signers
stellar keys generate guardian --network testnet --fund
stellar keys generate signer1 --network testnet
stellar keys generate signer2 --network testnet
stellar keys generate signer3 --network testnet

# Add the signers (weight 1 each)
for s in signer1 signer2 signer3; do
  stellar tx new set-options --source guardian --network testnet \
    --signer "$(stellar keys address $s)" --signer-weight 1
done

# Require 2 signatures for low/medium/high operations and disable the master key.
# Do this last: after it runs, the guardian key alone can no longer sign.
stellar tx new set-options --source guardian --network testnet \
  --low-threshold 2 --med-threshold 2 --high-threshold 2 --master-weight 0
```

Point the governance contract at the account (admin only):

```bash
stellar contract invoke --id "$GOVERNANCE_ID" --source admin --network testnet \
  -- set_guardian --new_guardian "$(stellar keys address guardian)"
```

### Vetoing with two of the three signers

Build the veto with the guardian as the source account, collect two signatures,
then submit:

```bash
stellar contract invoke --id "$GOVERNANCE_ID" --source guardian --network testnet \
  --build-only \
  -- veto --caller "$(stellar keys address guardian)" --proposal_id 1 \
         --reason "Action drains the treasury; see the incident report" > veto.xdr

stellar tx simulate --network testnet < veto.xdr > veto.sim.xdr
stellar tx sign --sign-with-key signer1 --network testnet < veto.sim.xdr > veto.s1.xdr
stellar tx sign --sign-with-key signer2 --network testnet < veto.s1.xdr > veto.s2.xdr
stellar tx send --network testnet < veto.s2.xdr
```

With only one signature the transaction fails the medium threshold and the
veto is rejected. CLI flag names change between `stellar-cli` releases; check
`stellar tx --help` for your version.

## Option B: a multisig contract

The guardian can also be a contract that collects approvals and then calls
`veto()` itself, passing its own address as `caller`. A contract is always
authorized for calls it makes directly, so `require_auth()` passes once the
contract decides the threshold is met.

`a_two_of_three_multisig_guardian_can_veto` in
`contracts/governance/src/test.rs` demonstrates this with a minimal 2-of-3
contract:

- a single signer calling `veto()` directly gets `Unauthorized`;
- one approval leaves the proposal `Queued`;
- the second approval triggers the veto and the proposal becomes `Vetoed`.

That test contract is an illustration, not an audited multisig. For production,
prefer Option A or an audited multisig contract.

## Rotating the guardian

`set_guardian()` takes effect immediately, so a compromised signer set can be
replaced mid-timelock. For Option A you can also rotate individual signers with
`set-options --signer <G...> --signer-weight 0` without changing the guardian
address.
