//! Property tests for the vote tallies of `quorum-governance` (issue #441).
//!
//! Votes can change, so a tally is only correct if `remove_weight` and
//! `add_weight` keep it in step with each voter's latest choice. Every case
//! generates a set of voters with fixed weights and a random sequence of
//! vote and change operations spread over the voting window. After each step
//! the stored tallies are compared with a model built from each voter's
//! latest choice.
//!
//! Standard proposals: `for_votes`, `against_votes` and `abstain_votes` must
//! equal the sums of the weights behind each voter's latest choice.
//!
//! Conviction proposals: the same latest-choice property must hold for the
//! raw tallies (conviction proposals keep them too), and the conviction
//! tally is modelled separately. The model mirrors `vote()` and `decay_to()`:
//! each vote on a conviction proposal first halves both sides once per whole
//! `CONVICTION_HALF_LIFE` since the sides were last written, then adds the
//! voter's weight to the side it chose (abstentions add nothing), then
//! records the current ledger as the write time. A change of vote does not
//! retract the weight that was previously added to the conviction tally, so
//! the conviction model is an event model, not a latest-choice model.
//!
//! Lives under `tests/` so it needs no edit to `src/test.rs` or to the
//! manifests. `proptest` is already a governance dev-dependency.

use proptest::prelude::*;
use quorum_governance::{GovernanceContract, GovernanceContractClient};
use quorum_token::{QuorumToken, QuorumTokenClient};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, String};

const LEDGERS_PER_DAY: u32 = 17_280;
/// Mirrors `CONVICTION_HALF_LIFE` in `src/lib.rs`.
const CONVICTION_HALF_LIFE: u32 = LEDGERS_PER_DAY * 3;
/// Long enough for several half-lives, matching the conviction tests in `src/test.rs`.
const VOTING_PERIOD: u32 = LEDGERS_PER_DAY * 10;
const TIMELOCK_PERIOD: u32 = 50;
const EXECUTION_GRACE: u32 = LEDGERS_PER_DAY * 7;
const GENESIS: u32 = 10;
const OPENED: u32 = 20;

const VOTE_AGAINST: u32 = 0;
const VOTE_FOR: u32 = 1;
const VOTE_ABSTAIN: u32 = 2;

const MAX_VOTERS: usize = 5;
const MAX_OPS: usize = 16;
const MAX_WEIGHT: i128 = 1_000_000;
/// Each step advances up to two half-lives, so decay steps are exercised.
const MAX_ADVANCE: u32 = CONVICTION_HALF_LIFE * 2;

/// Model of one conviction tally: both sides plus the ledger they were last written at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Side {
    for_c: i128,
    against_c: i128,
    updated: u32,
}

impl Side {
    /// Same stepwise rule as `decay_to`: one halving per whole half-life since
    /// the last write. A shift of 128 or more leaves nothing.
    fn decayed(self, now: u32) -> Side {
        let halvings = now.saturating_sub(self.updated) / CONVICTION_HALF_LIFE;
        let shift = |v: i128| if halvings >= 128 { 0 } else { v >> halvings };
        Side {
            for_c: shift(self.for_c),
            against_c: shift(self.against_c),
            updated: now,
        }
    }
}

/// Model of one proposal: each voter's weight, latest choice, and (for
/// conviction proposals) the conviction tally.
struct Model {
    weights: Vec<i128>,
    latest: Vec<Option<u32>>,
    conviction: Option<Side>,
}

impl Model {
    fn new(weights: &[i128], conviction_opened_at: Option<u32>) -> Self {
        Model {
            weights: weights.to_vec(),
            latest: vec![None; weights.len()],
            conviction: conviction_opened_at.map(|opened| Side { updated: opened, ..Side::default() }),
        }
    }

    /// Applies one successful `vote(voter, support)` made at ledger `now`.
    fn apply(&mut self, voter: usize, support: u32, now: u32) {
        self.latest[voter] = Some(support);
        if let Some(side) = self.conviction {
            let mut side = side.decayed(now);
            match support {
                VOTE_AGAINST => side.against_c += self.weights[voter],
                VOTE_FOR => side.for_c += self.weights[voter],
                _ => {}
            }
            self.conviction = Some(side);
        }
    }

    /// (for, against, abstain) as sums of each voter's latest choice.
    fn tallies(&self) -> (i128, i128, i128) {
        let mut sums = (0i128, 0i128, 0i128);
        for (weight, choice) in self.weights.iter().zip(&self.latest) {
            match choice {
                Some(VOTE_FOR) => sums.0 += weight,
                Some(VOTE_AGAINST) => sums.1 += weight,
                Some(VOTE_ABSTAIN) => sums.2 += weight,
                _ => {}
            }
        }
        sums
    }
}

fn deploy(env: &Env, supply: i128) -> (Address, Address, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let guardian = Address::generate(env);

    let token_id = env.register(QuorumToken, ());
    QuorumTokenClient::new(env, &token_id).initialize(
        &admin,
        &String::from_str(env, "Quorum"),
        &String::from_str(env, "QUORUM"),
        &7,
        &supply,
    );

    let governance_id = env.register(GovernanceContract, ());
    GovernanceContractClient::new(env, &governance_id).initialize(
        &admin,
        &guardian,
        &token_id,
        &1u32,
        &VOTING_PERIOD,
        &TIMELOCK_PERIOD,
        &EXECUTION_GRACE,
        &0i128,
    );

    (admin, token_id, governance_id)
}

/// Compares the stored state of `id` with the model after one step.
fn check(
    env: &Env,
    governance: &GovernanceContractClient,
    id: u64,
    voters: &[Address],
    model: &Model,
) -> Result<(), TestCaseError> {
    let proposal = governance.get_proposal(&id);
    let (for_sum, against_sum, abstain_sum) = model.tallies();
    prop_assert_eq!(proposal.for_votes, for_sum, "for tally diverged from latest-choice sum");
    prop_assert_eq!(proposal.against_votes, against_sum, "against tally diverged from latest-choice sum");
    prop_assert_eq!(proposal.abstain_votes, abstain_sum, "abstain tally diverged from latest-choice sum");

    for (voter, choice) in voters.iter().zip(&model.latest) {
        prop_assert_eq!(governance.get_vote(&id, voter), *choice, "recorded choice diverged");
    }

    match model.conviction {
        None => prop_assert_eq!(governance.get_conviction(&id), None),
        Some(side) => {
            let now = env.ledger().sequence();
            let expected = side.decayed(now);
            let stored = governance.get_conviction(&id);
            prop_assert!(stored.is_some(), "conviction proposal has no conviction tally");
            let stored = stored.unwrap();
            prop_assert_eq!(stored.for_conviction, expected.for_c, "for conviction diverged from decayed model");
            prop_assert_eq!(stored.against_conviction, expected.against_c, "against conviction diverged from decayed model");
        }
    }
    Ok(())
}

/// Runs one generated scenario against a standard or conviction proposal.
/// `ops` are (voter index, support, ledgers to advance before the vote).
fn run(conviction: bool, weights: &[i128], ops: &[(usize, u32, u32)]) -> Result<(), TestCaseError> {
    let env = Env::default();
    let supply = weights.iter().sum::<i128>() + 1;
    let (admin, token_id, governance_id) = deploy(&env, supply);
    let token = QuorumTokenClient::new(&env, &token_id);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    // Holders are funded before the snapshot, so each carries its full weight.
    env.ledger().set_sequence_number(GENESIS);
    let voters: Vec<Address> = weights.iter().map(|_| Address::generate(&env)).collect();
    for (voter, weight) in voters.iter().zip(weights) {
        token.transfer(&admin, voter, weight);
    }

    env.ledger().set_sequence_number(OPENED);
    let title = String::from_str(&env, "Continuous grant stream");
    let description = String::from_str(&env, "Fund a stream of grants.");
    let metadata_uri = String::from_str(&env, "");
    let opened = env.ledger().sequence();
    let id = if conviction {
        governance.create_conviction_proposal(&admin, &title, &description, &metadata_uri)
    } else {
        governance.create_proposal(&admin, &title, &description, &metadata_uri)
    };

    let proposal = governance.get_proposal(&id);
    let end = proposal.end_ledger;
    let mut now = proposal.start_ledger;
    env.ledger().set_sequence_number(now);

    let mut model = Model::new(weights, conviction.then_some(opened));
    check(&env, &governance, id, &voters, &model)?;

    for &(voter, support, advance) in ops {
        // Clamp to the window so every generated vote is valid input.
        now = now.saturating_add(advance).min(end);
        env.ledger().set_sequence_number(now);

        governance.vote(&voters[voter], &id, &support);
        model.apply(voter, support, now);
        check(&env, &governance, id, &voters, &model)?;
    }
    Ok(())
}

/// Voters with positive weights, and a sequence of votes and changes that
/// reference them by index.
fn scenario() -> impl Strategy<Value = (Vec<i128>, Vec<(usize, u32, u32)>)> {
    (1usize..=MAX_VOTERS).prop_flat_map(|n| {
        (
            prop::collection::vec(1i128..=MAX_WEIGHT, n),
            prop::collection::vec((0..n, 0u32..=2, 0u32..=MAX_ADVANCE), 1..=MAX_OPS),
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Standard proposals: stored tallies equal the sums of latest choices after every step.
    #[test]
    fn standard_tallies_equal_the_sum_of_each_voters_latest_choice((weights, ops) in scenario()) {
        run(false, &weights, &ops)?;
    }

    /// Conviction proposals: the latest-choice tallies hold, and the
    /// conviction tally matches the decayed event model after every step.
    #[test]
    fn conviction_tallies_match_latest_choice_and_the_decayed_model((weights, ops) in scenario()) {
        run(true, &weights, &ops)?;
    }
}
