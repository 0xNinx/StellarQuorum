# Architecture Decision Records (ADRs)

This folder contains recorded decisions about Quorum's architecture, design, and implementation. Each ADR documents a significant choice, the reasoning behind it, and its consequences.

## Why ADRs?

As the protocol evolves, design questions become case law: "why did we do it this way?" ADRs keep the reasoning discoverable after the PR merges, so contributors and maintainers can judge whether a constraint still holds or whether a decision can be revisited.

## When to Write an ADR

Write an ADR when:
- You are making a **significant design choice** that affects multiple systems or long-term maintainability
- The choice involves **trade-offs** between competing approaches
- The decision is **unlikely to change** once made
- Future contributors would benefit from understanding the **reasoning**, not just the **outcome**

Do not write an ADR for:
- Routine code changes or bug fixes
- Decisions that are obvious from the code
- Reversible choices with no long-term impact

**Examples of ADR-worthy decisions:**
- Protocol-level choices (e.g., staking model, conviction voting mechanics)
- Ownership boundaries between contracts (e.g., who manages delegation state)
- Constraint decisions (e.g., checkpoint retention windows, frozen timelock minimums)
- Security model choices (e.g., guardian veto window, proposal deposit handling)

## Index

| # | Title | Status | Date |
|---|---|---|---|
| [001](001-delegation-owned-by-token.md) | Delegation Owned by the Token | Accepted | 2024-10-21 |

## How to Propose an ADR

1. Create a new file `docs/adr/NNN-short-title.md` using the [TEMPLATE.md](TEMPLATE.md) as a starting point
2. Fill in the Context, Decision, Rationale, and Consequences sections
3. Open a PR; discuss the decision openly before merging
4. Once accepted, update the index above and merge

## References

- [CONTRIBUTING.md](../CONTRIBUTING.md#contributing-to-documentation) — when to document decisions
- [ADR Repository (ADR GitHub org)](https://adr.github.io/) — external reference on ADR best practices
