## Description

<!-- Provide a brief summary of the changes in this pull request and the rationale behind them. -->

## Related Issue

<!-- Link the issue(s) resolved or referenced by this pull request. -->
<!-- Examples: Closes #123, Fixes #456. If contributing through Drips Wave: /claim #123 -->
Closes #

## Changes Made

- 

## Verification & Test Evidence

<!-- Describe how you verified your changes and provide relevant output or evidence. -->
<!-- Examples:
  - Smart contracts: cd contracts && cargo test
  - Smart contracts clippy: cd contracts && cargo clippy --all-targets --all-features -- -D warnings
  - Frontend: cd frontend && npm test && npm run lint
  - SDK: cd sdk && npm run build && npm test
-->

```text
<!-- Paste test execution output, build logs, or add screenshots for UI changes -->
```

## Checklist

- [ ] Code follows the project conventions and style guidelines
- [ ] Tests have been added or updated to cover the changes
- [ ] All relevant test suites pass locally (`cargo test`, `npm test`)
- [ ] Linting checks pass with no warnings or errors (`cargo clippy`, `npm run lint`)
- [ ] Documentation has been updated where applicable (README, JSDoc, docs/)
- [ ] No temporary debug code or extraneous logging left in
- [ ] Commits follow the Conventional Commits specification

## Contract Security Checklist

<!-- Complete this section if the PR changes anything under contracts/. Otherwise delete it. -->

- [ ] Every state change emits an event, and new or changed events are documented
- [ ] `require_auth()` is called on the right address before any state is read for a decision or written
- [ ] Arithmetic on balances, votes and ledger numbers uses checked operations (`checked_add`, `checked_mul`, ...) or is proven not to overflow
- [ ] New or touched persistent and instance entries extend their TTL where they are written and read
- [ ] The TypeScript and Python SDKs are updated for any changed function, argument, error or event
- [ ] WASM size is within the baseline (`scripts/check-wasm-size.sh`, `contracts/wasm-size-baseline.txt`)
