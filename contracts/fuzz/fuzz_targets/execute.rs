//! Fuzzes `set_proposal_action` and `execute` — the most security-sensitive
//! code path in the contract.
//!
//! Every iteration:
//! 1. Deploys a fresh token + governor from fuzzed config.
//! 2. Opens a proposal, votes it to passing, and queues it.
//! 3. Calls `set_proposal_action` with a fuzzed target / function / args.
//! 4. Advances past the timelock and calls `execute`.
//!
//! Invariants checked after every complete iteration:
//! * If execute returned `Ok`, the proposal status is `Executed`.
//! * If execute returned `Err`, the proposal is still `Queued` (no partial
//!   side effects committed).
//! * No path ever traps (panics / arithmetic overflows treated as failures).
#![no_main]

mod common;

use common::*;
use libfuzzer_sys::fuzz_target;
use quorum_governance::{GovernanceContractClient, ProposalStatus};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Symbol};

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);
    let cfg = FuzzConfig::decode(&mut cursor);
    // Which function name to attach (clamped to printable ASCII, 1–32 chars)
    let fn_len = (cursor.u8() % 32) + 1;
    let fn_raw: u8 = cursor.u8();
    // Build a short symbol from the raw byte (soroban Symbol allows a-z A-Z 0-9 _)
    let fn_name_char = match fn_raw % 4 {
        0 => "transfer",
        1 => "approve",
        2 => "nonexistent_fn",
        _ => "set_data",
    };
    let _ = fn_len; // may refine in future iterations

    // Pick the action target: a known contract, a random address, or the
    // governance contract itself (re-entrant execute attempt).
    let target_sel = cursor.u8() % 4;
    let adv_raw = cursor.u32();

    let (env, h) = deploy(&cfg);
    let governance = GovernanceContractClient::new(&env, &h.governance_id);

    // Create proposal; skip iteration if governor rejects the config.
    let proposal_id = match create_proposal(&env, &h) {
        Some(id) => id,
        None => return,
    };
    let end = match proposal_end(&env, &h, proposal_id) {
        Some(e) => e,
        None => return,
    };

    // Vote as all funded voters so the proposal is likely to pass quorum.
    for voter_idx in 0u8..3 {
        set_sequence(&env, end.saturating_sub(2).saturating_add(u32::from(voter_idx)));
        fuzz_call!("vote", governance.try_vote(h.voter(voter_idx), &proposal_id, &1u32));
    }

    // Advance past the voting deadline and finalize.
    set_sequence(&env, end.saturating_add(1).saturating_add(adv_raw % 8));
    fuzz_call!("finalize", governance.try_finalize(&proposal_id));

    // Check status before attaching actions — if the proposal did not pass
    // (too low quorum given the fuzzed config) there is nothing more to test.
    let proposal_after_finalize = match fuzz_call!("get_proposal", governance.try_get_proposal(&proposal_id)) {
        Some(p) => p,
        None => return,
    };
    if proposal_after_finalize.status != ProposalStatus::Queued {
        return;
    }
    let queue_ledger = proposal_after_finalize.queue_ledger;

    // Pick the action target from the available contracts or a random address.
    let action_target: Address = match target_sel {
        0 => h.token_id.clone(),
        1 => h.governance_id.clone(),
        2 => Address::generate(&env),
        _ => h.voters[0].clone(),
    };

    // Attach up to two actions; errors are acceptable (target not allowlisted,
    // voting period ended, etc.) — only traps fail the iteration.
    let fn_sym = Symbol::new(&env, fn_name_char);
    fuzz_call!(
        "set_proposal_action#1",
        governance.try_set_proposal_action(
            &h.admin,
            &proposal_id,
            &action_target,
            &fn_sym,
            &soroban_sdk::Vec::new(&env),
        )
    );

    // Advance past the timelock to allow execute.
    let after_timelock = queue_ledger.saturating_add(adv_raw % 64);
    set_sequence(&env, after_timelock.min(SAFE_MAX_SEQUENCE));

    let execute_result = governance.try_execute(&h.admin, &proposal_id);

    // Invariant: after execute the proposal is either Executed or still Queued.
    if let Some(proposal_after) = fuzz_call!("get_proposal_post_execute", governance.try_get_proposal(&proposal_id)) {
        match &execute_result {
            Ok(Ok(())) => {
                assert_eq!(
                    proposal_after.status,
                    ProposalStatus::Executed,
                    "execute returned Ok but proposal is not Executed"
                );
            }
            Err(Ok(_)) => {
                // Contract returned a declared error — proposal must remain Queued.
                assert_eq!(
                    proposal_after.status,
                    ProposalStatus::Queued,
                    "execute failed but proposal left non-Queued status"
                );
            }
            Ok(Err(err)) => panic!("execute: return conversion failed: {:?}", err),
            Err(Err(err)) => panic!("execute: invoke-level failure (trap): {:?}", err),
        }
    }
});
