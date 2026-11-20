//! End-to-end governance cycle driven from the token side.
//!
//! The unit suites cover each contract alone, and the governance suite
//! registers a real token only as a source of voting power. This test starts
//! from the token instead: supply is minted and distributed through every
//! balance-moving path, a proposal snapshots it, balances keep moving, and the
//! vote is finalized and executed. At each step it checks that governance
//! counted exactly what the token's checkpoints recorded.
//!
//! The later tests cover the remaining governance paths end to end with both
//! real contracts: co-sponsored proposals, conviction voting, proposal
//! deposits (refunded and forfeited) and the token's staking vote boost.

use quorum_governance::{GovernanceContract, GovernanceContractClient, GovernanceError, ProposalStatus};
use quorum_token::{QuorumToken, QuorumTokenClient, TokenError};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, String};

const INITIAL_SUPPLY: i128 = 1_000_000;
const QUORUM_BPS: u32 = 500; // 5%
const VOTING_PERIOD: u32 = 100;
const TIMELOCK_PERIOD: u32 = 50;
const PROPOSAL_THRESHOLD: i128 = 100_000;

// Mirrors `LEDGERS_PER_DAY` in both contracts (private there).
const LEDGERS_PER_DAY: u32 = 17_280;
// Mirrors `CONVICTION_HALF_LIFE` in the governance contract (three days).
const CONVICTION_HALF_LIFE: u32 = LEDGERS_PER_DAY * 3;
// Mirrors `MAX_LOCK_LEDGERS` in the token contract (365 days), the scale of
// the stake boost.
const MAX_LOCK_LEDGERS: u32 = LEDGERS_PER_DAY * 365;

const AGAINST: u32 = 0;
const FOR: u32 = 1;
const ABSTAIN: u32 = 2;

struct Dao<'a> {
    env: Env,
    admin: Address,
    token: QuorumTokenClient<'a>,
    governance: GovernanceContractClient<'a>,
}

fn setup<'a>() -> Dao<'a> {
    setup_with(VOTING_PERIOD)
}

fn setup_with<'a>(voting_period: u32) -> Dao<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(100);
    let admin = Address::generate(&env);
    let guardian = Address::generate(&env);

    let token_id = env.register(QuorumToken, ());
    let token = QuorumTokenClient::new(&env, &token_id);
    token.initialize(
        &admin,
        &String::from_str(&env, "Quorum"),
        &String::from_str(&env, "QUORUM"),
        &7,
        &INITIAL_SUPPLY,
    );

    let governance_id = env.register(GovernanceContract, ());
    let governance = GovernanceContractClient::new(&env, &governance_id);
    governance.initialize(
        &admin,
        &guardian,
        &token_id,
        &QUORUM_BPS,
        &voting_period,
        &TIMELOCK_PERIOD,
        &(17_280 * 7),
        &PROPOSAL_THRESHOLD,
    );

    Dao { env, admin, token, governance }
}

fn advance_to(env: &Env, ledger: u32) {
    env.ledger().set_sequence_number(ledger);
}

fn text(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

/// The sum of `holders`' balances must equal total supply — the token is the
/// only ledger of record, so governance activity must never create or destroy
/// tokens.
fn assert_supply_conserved(token: &QuorumTokenClient, holders: &[&Address]) {
    let sum: i128 = holders.iter().map(|h| token.balance(h)).sum();
    assert_eq!(sum, token.total_supply());
}

#[test]
fn full_cycle_mint_distribute_snapshot_vote_finalize_execute() {
    let Dao { env, admin, token, governance } = setup();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    let dave = Address::generate(&env);
    let distributor = Address::generate(&env);
    let erin = Address::generate(&env);
    let frank = Address::generate(&env);
    let everyone = [&admin, &alice, &bob, &carol, &dave, &distributor, &erin, &frank];

    // ── Mint and distribute ─────────────────────────────────────────────────
    // Every balance-moving path is used, so each one has to write the
    // checkpoints the snapshot is later read from.
    advance_to(&env, 101);
    token.mint(&alice, &300_000);
    token.mint(&bob, &200_000);
    token.transfer(&admin, &carol, &100_000);
    token.approve(&admin, &distributor, &50_000, &1_000);
    token.transfer_from(&distributor, &admin, &dave, &50_000);
    assert_eq!(token.allowance(&admin, &distributor), 0);

    advance_to(&env, 110);
    token.transfer(&carol, &alice, &40_000);

    assert_eq!(token.total_supply(), 1_500_000);
    assert_eq!(token.balance(&admin), 850_000);
    assert_eq!(token.balance(&alice), 340_000);
    assert_eq!(token.balance(&bob), 200_000);
    assert_eq!(token.balance(&carol), 60_000);
    assert_eq!(token.balance(&dave), 50_000);
    assert_supply_conserved(&token, &everyone);

    // ── Snapshot ────────────────────────────────────────────────────────────
    // Dave holds less than the proposal threshold; Alice holds more.
    advance_to(&env, 120);
    assert_eq!(
        governance.try_create_proposal(&dave, &text(&env, "t"), &text(&env, "d"), &text(&env, "")),
        Err(Ok(GovernanceError::BelowProposalThreshold))
    );
    let id = governance.create_proposal(
        &alice,
        &text(&env, "Fund the grants round"),
        &text(&env, "Release 50,000 QUORUM to the grants multisig."),
        &text(&env, "ipfs://grants-round"),
    );
    let proposal = governance.get_proposal(&id);
    let snapshot = proposal.snapshot_ledger;
    assert_eq!(snapshot, 120);
    assert_eq!(proposal.status, ProposalStatus::Active);
    // 5% of the supply at creation.
    assert_eq!(proposal.quorum_required, 75_000);

    // ── Balances keep moving after the snapshot ─────────────────────────────
    advance_to(&env, 121);
    token.transfer(&alice, &erin, &100_000); // Erin buys in after the snapshot.
    token.mint(&frank, &500_000); // Dilution after the snapshot.
    token.burn(&bob, &50_000);
    assert_eq!(token.total_supply(), 1_950_000);
    assert_supply_conserved(&token, &everyone);

    // The token still reports the snapshot-era balances, whatever happened
    // since.
    assert_eq!(token.get_past_balance(&alice, &snapshot), 340_000);
    assert_eq!(token.get_past_balance(&bob, &snapshot), 200_000);
    assert_eq!(token.get_past_balance(&carol, &snapshot), 60_000);
    assert_eq!(token.get_past_balance(&dave, &snapshot), 50_000);
    assert_eq!(token.get_past_balance(&erin, &snapshot), 0);
    assert_eq!(token.get_past_balance(&frank, &snapshot), 0);

    // Minting after creation must not move the bar the proposal has to clear.
    assert_eq!(governance.get_proposal(&id).quorum_required, 75_000);

    // ── Vote ────────────────────────────────────────────────────────────────
    advance_to(&env, 130);
    governance.vote(&alice, &id, &FOR);
    governance.vote(&bob, &id, &AGAINST);
    governance.vote(&carol, &id, &ABSTAIN);
    governance.vote(&dave, &id, &FOR);

    // Tokens acquired after the snapshot carry no weight.
    assert_eq!(governance.try_vote(&erin, &id, &FOR), Err(Ok(GovernanceError::NoVotingPower)));
    assert_eq!(governance.try_vote(&frank, &id, &FOR), Err(Ok(GovernanceError::NoVotingPower)));
    // Still inside the window, so a repeat vote replaces the earlier choice.
    // Repeating the same choice leaves the tallies below unchanged.
    assert!(governance.try_vote(&alice, &id, &FOR).is_ok());

    // Each tally is exactly the voters' snapshot balances, not their live
    // ones: Alice now holds 240,000 and Bob 150,000.
    let proposal = governance.get_proposal(&id);
    assert_eq!(proposal.for_votes, 340_000 + 50_000);
    assert_eq!(proposal.against_votes, 200_000);
    assert_eq!(proposal.abstain_votes, 60_000);
    assert_eq!(governance.get_vote(&id, &carol), Some(ABSTAIN));
    assert_eq!(governance.get_vote(&id, &erin), None);

    // Voting reads balances; it never moves them.
    assert_eq!(token.balance(&alice), 240_000);
    assert_eq!(token.balance(&bob), 150_000);
    assert_eq!(token.total_supply(), 1_950_000);
    assert_supply_conserved(&token, &everyone);

    // ── Finalize ────────────────────────────────────────────────────────────
    assert_eq!(governance.try_finalize(&id), Err(Ok(GovernanceError::VotingNotActive)));
    advance_to(&env, proposal.end_ledger + 1);
    assert_eq!(governance.try_vote(&admin, &id, &AGAINST), Err(Ok(GovernanceError::VotingPeriodEnded)));

    assert_eq!(governance.finalize(&id), ProposalStatus::Queued);
    let proposal = governance.get_proposal(&id);
    assert_eq!(proposal.queue_ledger, proposal.end_ledger + 1 + TIMELOCK_PERIOD);

    // ── Execute ─────────────────────────────────────────────────────────────
    let executor = Address::generate(&env);
    advance_to(&env, proposal.queue_ledger - 1);
    assert_eq!(governance.try_execute(&executor, &id), Err(Ok(GovernanceError::TimelockNotExpired)));
    advance_to(&env, proposal.queue_ledger);
    governance.execute(&executor, &id);
    assert_eq!(governance.get_proposal(&id).status, ProposalStatus::Executed);

    // The whole cycle left the token's books intact.
    assert_eq!(token.total_supply(), 1_950_000);
    assert_supply_conserved(&token, &everyone);
}

#[test]
fn snapshot_holders_can_defeat_a_proposal_that_live_balances_would_pass() {
    let Dao { env, admin, token, governance } = setup();
    let proposer = Address::generate(&env);
    let whale = Address::generate(&env);

    advance_to(&env, 101);
    token.transfer(&admin, &proposer, &150_000);
    token.transfer(&admin, &whale, &300_000);

    advance_to(&env, 110);
    let id = governance.create_proposal(
        &proposer,
        &text(&env, "Lower the quorum"),
        &text(&env, "Move quorum_bps from 500 to 100."),
        &text(&env, ""),
    );

    // After the snapshot the whale sells everything to the proposer. Live,
    // the proposer would now outvote the whale 450,000 to 0.
    advance_to(&env, 111);
    token.transfer(&whale, &proposer, &300_000);
    assert_eq!(token.balance(&proposer), 450_000);
    assert_eq!(token.balance(&whale), 0);

    // At the snapshot, though, the whale's 300,000 outweighs the proposer's
    // 150,000.
    governance.vote(&proposer, &id, &FOR);
    governance.vote(&whale, &id, &AGAINST);
    let proposal = governance.get_proposal(&id);
    assert_eq!(proposal.for_votes, 150_000);
    assert_eq!(proposal.against_votes, 300_000);

    advance_to(&env, proposal.end_ledger + 1);
    assert_eq!(governance.finalize(&id), ProposalStatus::Failed);
    assert_eq!(
        governance.try_execute(&proposer, &id),
        Err(Ok(GovernanceError::ProposalNotPassed))
    );
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
}

#[test]
fn cosponsored_proposal_clears_threshold_on_combined_balances() {
    let Dao { env, admin, token, governance } = setup();
    let proposer = Address::generate(&env);
    let cosponsor = Address::generate(&env);

    // Each holds less than the 100,000 threshold alone; together they hold
    // 110,000.
    advance_to(&env, 101);
    token.transfer(&admin, &proposer, &60_000);
    token.transfer(&admin, &cosponsor, &50_000);

    advance_to(&env, 110);
    assert_eq!(
        governance.try_create_proposal(&proposer, &text(&env, "t"), &text(&env, "d"), &text(&env, "")),
        Err(Ok(GovernanceError::BelowProposalThreshold))
    );
    // A proposer cannot also be listed as a co-sponsor.
    assert_eq!(
        governance.try_create_cosponsored_proposal(
            &proposer,
            &soroban_sdk::vec![&env, proposer.clone()],
            &text(&env, "t"),
            &text(&env, "d"),
            &text(&env, ""),
        ),
        Err(Ok(GovernanceError::InvalidCosponsors))
    );

    let id = governance.create_cosponsored_proposal(
        &proposer,
        &soroban_sdk::vec![&env, cosponsor.clone()],
        &text(&env, "Co-sponsored grant"),
        &text(&env, "Needs two holders to open."),
        &text(&env, ""),
    );
    assert_eq!(governance.get_cosponsors(&id), soroban_sdk::vec![&env, cosponsor.clone()]);
    let proposal = governance.get_proposal(&id);
    assert_eq!(proposal.snapshot_ledger, 110);
    assert_eq!(proposal.quorum_required, 50_000);

    // Co-sponsors count toward the threshold at creation; after that each
    // voter, co-sponsor or not, carries only their own snapshot balance.
    advance_to(&env, 115);
    governance.vote(&admin, &id, &FOR); // 890,000 held after the transfers above.
    governance.vote(&proposer, &id, &FOR);
    governance.vote(&cosponsor, &id, &AGAINST);

    let proposal = governance.get_proposal(&id);
    assert_eq!(proposal.for_votes, 890_000 + 60_000);
    assert_eq!(proposal.against_votes, 50_000);

    advance_to(&env, proposal.end_ledger + 1);
    assert_eq!(governance.finalize(&id), ProposalStatus::Queued);
    assert_supply_conserved(&token, &[&admin, &proposer, &cosponsor]);
}

#[test]
fn conviction_decay_reverses_raw_majority_at_close() {
    // A window of two half-lives, so an early vote decays exactly once before
    // the close and the tally is decided on what remains.
    let window = 2 * CONVICTION_HALF_LIFE;
    let Dao { env, admin, token, governance } = setup_with(window);
    let proposer = Address::generate(&env);
    let for_voter = Address::generate(&env);
    let against_voter = Address::generate(&env);

    advance_to(&env, 101);
    token.transfer(&admin, &proposer, &150_000);
    token.transfer(&admin, &for_voter, &400_000);
    token.transfer(&admin, &against_voter, &300_000);

    advance_to(&env, 110);
    let id = governance.create_conviction_proposal(
        &proposer,
        &text(&env, "Continuous grant stream"),
        &text(&env, "Fund a stream of grants."),
        &text(&env, ""),
    );
    let proposal = governance.get_proposal(&id);
    assert_eq!(proposal.start_ledger, 111);
    assert_eq!(proposal.end_ledger, 111 + window);

    advance_to(&env, 112);
    governance.vote(&for_voter, &id, &FOR);
    assert_eq!(governance.get_conviction(&id).unwrap().for_conviction, 400_000);

    // One half-life later the early support has halved; the late vote lands
    // at full weight.
    advance_to(&env, 112 + CONVICTION_HALF_LIFE);
    governance.vote(&against_voter, &id, &AGAINST);
    let tally = governance.get_conviction(&id).unwrap();
    assert_eq!(tally.for_conviction, 200_000);
    assert_eq!(tally.against_conviction, 300_000);

    // Raw totals say For wins 400,000 to 300,000 ...
    let proposal = governance.get_proposal(&id);
    assert_eq!(proposal.for_votes, 400_000);
    assert_eq!(proposal.against_votes, 300_000);

    // ... but no further half-life passes before the close, so decayed support
    // stands at 200,000 to 300,000 and the proposal fails.
    advance_to(&env, proposal.end_ledger + 1);
    assert_eq!(governance.finalize(&id), ProposalStatus::Failed);
    assert_supply_conserved(&token, &[&admin, &proposer, &for_voter, &against_voter]);
}

#[test]
fn deposit_is_refunded_to_proposer_when_proposal_passes() {
    const DEPOSIT: i128 = 10_000;
    let Dao { env, admin, token, governance } = setup();
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let poor = Address::generate(&env);
    let rich = Address::generate(&env);

    governance.set_proposal_deposit(&DEPOSIT);

    advance_to(&env, 101);
    token.transfer(&admin, &proposer, &150_000);
    token.transfer(&admin, &voter, &400_000);
    token.transfer(&admin, &poor, &5_000);
    token.transfer(&admin, &rich, &200_000);

    advance_to(&env, 110);
    // The threshold is met, but the proposer's own balance cannot cover the
    // deposit.
    assert_eq!(
        governance.try_create_cosponsored_proposal(
            &poor,
            &soroban_sdk::vec![&env, rich.clone()],
            &text(&env, "t"),
            &text(&env, "d"),
            &text(&env, ""),
        ),
        Err(Ok(GovernanceError::InsufficientDeposit))
    );

    // Opening takes the deposit out of the proposer's balance into the
    // governance contract.
    let id = governance.create_proposal(
        &proposer,
        &text(&env, "Deposit-backed proposal"),
        &text(&env, "Refunded if it passes."),
        &text(&env, ""),
    );
    assert_eq!(token.balance(&proposer), 150_000 - DEPOSIT);
    assert_eq!(token.balance(&governance.address), DEPOSIT);
    assert_supply_conserved(&token, &[&admin, &proposer, &voter, &poor, &rich, &governance.address]);

    advance_to(&env, 112);
    governance.vote(&voter, &id, &FOR);

    let proposal = governance.get_proposal(&id);
    advance_to(&env, proposal.end_ledger + 1);
    assert_eq!(governance.finalize(&id), ProposalStatus::Queued);

    // A passed proposal gets the deposit back, and the contract holds none.
    assert_eq!(token.balance(&proposer), 150_000);
    assert_eq!(token.balance(&governance.address), 0);
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
    assert_supply_conserved(&token, &[&admin, &proposer, &voter, &poor, &rich, &governance.address]);
}

#[test]
fn deposit_is_forfeited_when_proposal_fails() {
    const DEPOSIT: i128 = 10_000;
    let Dao { env, admin, token, governance } = setup();
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    governance.set_proposal_deposit(&DEPOSIT);

    advance_to(&env, 101);
    token.transfer(&admin, &proposer, &150_000);
    token.transfer(&admin, &voter, &400_000);

    advance_to(&env, 110);
    let id = governance.create_proposal(
        &proposer,
        &text(&env, "Deposit-backed proposal"),
        &text(&env, "Forfeited if it fails."),
        &text(&env, ""),
    );
    assert_eq!(token.balance(&proposer), 150_000 - DEPOSIT);
    assert_eq!(token.balance(&governance.address), DEPOSIT);

    advance_to(&env, 112);
    governance.vote(&voter, &id, &AGAINST);

    let proposal = governance.get_proposal(&id);
    advance_to(&env, proposal.end_ledger + 1);
    assert_eq!(governance.finalize(&id), ProposalStatus::Failed);

    // The deposit is burned: the proposer is not repaid, the contract holds
    // nothing, and total supply shrinks by exactly the deposit.
    assert_eq!(token.balance(&proposer), 150_000 - DEPOSIT);
    assert_eq!(token.balance(&governance.address), 0);
    assert_eq!(token.total_supply(), INITIAL_SUPPLY - DEPOSIT);
    assert_supply_conserved(&token, &[&admin, &proposer, &voter, &governance.address]);

    // A failed proposal cannot be finalized again, so the deposit cannot move
    // twice.
    assert_eq!(governance.try_finalize(&id), Err(Ok(GovernanceError::VotingNotActive)));
    assert_eq!(token.total_supply(), INITIAL_SUPPLY - DEPOSIT);
}

#[test]
fn staking_boost_adds_to_snapshot_vote_power_and_locks_balance() {
    const STAKE: i128 = 150_000;
    const STAKE_LEDGER: u32 = 101;
    const SNAPSHOT: u32 = 110;
    let Dao { env, admin, token, governance } = setup();
    let staker = Address::generate(&env);
    let opponent = Address::generate(&env);

    advance_to(&env, STAKE_LEDGER);
    token.transfer(&admin, &staker, &200_000);
    token.transfer(&admin, &opponent, &260_000);
    // Locked for the maximum term, so the boost starts at the full stake.
    token.stake(&staker, &STAKE, &MAX_LOCK_LEDGERS);
    let unlock_ledger = STAKE_LEDGER + MAX_LOCK_LEDGERS;

    // The stake does not move balances; it only locks part of them.
    assert_eq!(token.balance(&staker), 200_000);
    assert_eq!(token.locked_balance(&staker), STAKE);
    assert_eq!(token.spendable_balance(&staker), 200_000 - STAKE);

    advance_to(&env, SNAPSHOT);
    let id = governance.create_proposal(
        &staker,
        &text(&env, "Boosted vote"),
        &text(&env, "Staker's boosted weight decides."),
        &text(&env, ""),
    );
    let snapshot = governance.get_proposal(&id).snapshot_ledger;
    assert_eq!(snapshot, SNAPSHOT);

    // Without the boost the opponent's 260,000 would beat the staker's 200,000.
    // The boost at the snapshot is the stake scaled by the time left to unlock.
    let remaining = i128::from(unlock_ledger - snapshot);
    let expected_boost = STAKE * remaining / i128::from(MAX_LOCK_LEDGERS);
    assert!(expected_boost > 260_000 - 200_000);

    advance_to(&env, 115);
    governance.vote(&staker, &id, &FOR);
    governance.vote(&opponent, &id, &AGAINST);

    let proposal = governance.get_proposal(&id);
    assert_eq!(proposal.for_votes, 200_000 + expected_boost);
    assert_eq!(proposal.against_votes, 260_000);

    advance_to(&env, proposal.end_ledger + 1);
    assert_eq!(governance.finalize(&id), ProposalStatus::Queued);

    // Voting never moved balances, and the lock still holds: the staked part
    // cannot be spent or released before its unlock ledger.
    assert_eq!(token.balance(&staker), 200_000);
    assert!(token.try_transfer(&staker, &opponent, &60_000).is_err());
    assert_eq!(token.try_unstake(&staker), Err(Ok(TokenError::LockNotExpired)));
    assert_supply_conserved(&token, &[&admin, &staker, &opponent]);
}
