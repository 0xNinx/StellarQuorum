# ADR 001: Delegation Owned by the Token

**Status:** Accepted

**Date:** 2024-10-21

**Signed-off by:** sanmipaul

## Context

Governance proposals require voting power calculation at a fixed ledger (the proposal's `snapshot_ledger`) to prevent flash-loan and buy-the-vote attacks. Voting power depends on token balances and whether a holder has delegated to a representative.

The question: should delegation be managed by the governance contract or the token contract?

## Decision

**Delegation is owned by the token contract.**

The governance contract exposes facade methods (`delegate`, `undelegate`, `get_delegate`) that forward to the token. The token contract is the authority for delegation state.

## Rationale

1. **Single source of truth:** The token contract already holds balance checkpoints and is the authoritative record of an address's holdings. Delegation—the choice of who should vote that balance—is a property of the holder, not the proposal, so it belongs with the token's other holder properties (balance, allowance).

2. **Historical accuracy:** `get_past_votes(account, ledger)` reads the voting power an address had at a past ledger, including the representative they had delegated to at that time. This requires checkpoint history of both balance and delegation. Keeping both in one place (the token) simplifies the history model.

3. **Prevents inconsistency:** If governance owned delegation, it would need to call the token for balance history but maintain its own delegation history. Any divergence (missed events, failed writes) would create stale voting-power calculations. A single write path in the token eliminates this risk.

4. **Delegation is portable:** The delegation decision is about the holder's choice, not specific to any one governance contract. Keeping it in the token allows future governance instances or other protocols to reuse the same delegation state.

## Consequences

- **Positive:**
  - Delegation state is always consistent with the token's balance history.
  - Other contracts (and future governance instances) can query the same delegation without duplication.
  - Snapshots and historical lookups are deterministic: `get_past_votes` has one implementation.

- **Negative:**
  - Governance exposes facade methods that forward to the token, adding a layer of indirection.
  - Governance cannot locally control or veto delegation (though this is a feature, not a bug: delegation is a holder right, not a governance right).

- **Neutral:**
  - The token contract's scope is broader (it is now responsible for more than just SEP-41 transfers), but it remains a single, focused contract.

## References

- [contracts/governance/src/lib.rs](../../contracts/governance/src/lib.rs#L224) — governance's facade methods
- [contracts/token/README.md](../../contracts/token/README.md#delegation-and-snapshots) — token's delegation implementation
- [docs/architecture.md](../architecture.md#balance-checkpoints-and-snapshot-voting-power) — voting power architecture
