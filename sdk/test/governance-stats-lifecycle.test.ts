/**
 * Lifecycle invariant test for GovernanceStats (issue #440).
 *
 * Uses the mock-RPC server to simulate `getGovernanceStats()` responses that
 * represent the state after each proposal reaches a terminal status.  After
 * each simulated transition, the test asserts:
 *
 *   active + queued + executed + failed + cancelled + expired + vetoed == total_proposals
 */

import { StrKey, xdr } from '@stellar/stellar-sdk';
import { QuorumClient } from '../src';
import type { GovernanceStats, QuorumClientConfig } from '../src';
import { MockRpcServer, simulationSuccess } from './mockRpc';

const GOVERNANCE_ID = StrKey.encodeContract(Buffer.alloc(32, 0xaa));
const TOKEN_ID = StrKey.encodeContract(Buffer.alloc(32, 0xbb));
const NETWORK_PASSPHRASE = 'Test SDF Network ; September 2015';

let rpc: MockRpcServer;
let client: QuorumClient;

beforeAll(async () => {
  rpc = await MockRpcServer.start();
  client = new QuorumClient({
    rpcUrl: rpc.url,
    networkPassphrase: NETWORK_PASSPHRASE,
    governanceContractId: GOVERNANCE_ID,
    tokenContractId: TOKEN_ID,
  } satisfies QuorumClientConfig);
});

afterAll(async () => {
  await rpc.close();
});

/** Encode a GovernanceStats-shaped object as an XDR simulation result. */
function encodeStats(raw: {
  total_proposals: number;
  active_proposals: number;
  queued_proposals: number;
  executed_proposals: number;
  failed_proposals: number;
  cancelled_proposals: number;
  expired_proposals: number;
  vetoed_proposals: number;
}): string {
  // The contract returns a #[contracttype] struct; scValToNative decodes it
  // to a plain object with snake_case keys.  We reverse that by encoding a
  // Map<Symbol, u64> to represent the struct.
  const entries = Object.entries(raw).map(([k, v]) => {
    return new xdr.ScMapEntry({
      key: xdr.ScVal.scvSymbol(k),
      val: xdr.ScVal.scvU64(xdr.Uint64.fromString(String(v))),
    });
  });
  const val = xdr.ScVal.scvMap(entries);
  return val.toXDR('base64');
}

function serveStats(stats: Parameters<typeof encodeStats>[0]): void {
  const retval = encodeStats(stats);
  rpc.responder = () =>
    simulationSuccess(retval);
}

function invariantHolds(s: GovernanceStats): boolean {
  const sumTerminal =
    s.executedProposals +
    s.failedProposals +
    s.cancelledProposals +
    s.expiredProposals +
    s.vetoedProposals;
  return s.activeProposals + s.queuedProposals + sumTerminal === s.totalProposals;
}

describe('GovernanceStats lifecycle invariant (issue #440)', () => {
  const scenarios: Array<{ label: string; stats: Parameters<typeof encodeStats>[0] }> = [
    {
      label: 'proposal moves to Executed',
      stats: {
        total_proposals: 1,
        active_proposals: 0,
        queued_proposals: 0,
        executed_proposals: 1,
        failed_proposals: 0,
        cancelled_proposals: 0,
        expired_proposals: 0,
        vetoed_proposals: 0,
      },
    },
    {
      label: 'proposal moves to Failed',
      stats: {
        total_proposals: 1,
        active_proposals: 0,
        queued_proposals: 0,
        executed_proposals: 0,
        failed_proposals: 1,
        cancelled_proposals: 0,
        expired_proposals: 0,
        vetoed_proposals: 0,
      },
    },
    {
      label: 'proposal moves to Cancelled',
      stats: {
        total_proposals: 2,
        active_proposals: 1,
        queued_proposals: 0,
        executed_proposals: 0,
        failed_proposals: 0,
        cancelled_proposals: 1,
        expired_proposals: 0,
        vetoed_proposals: 0,
      },
    },
    {
      label: 'proposal moves to Expired',
      stats: {
        total_proposals: 3,
        active_proposals: 1,
        queued_proposals: 1,
        executed_proposals: 0,
        failed_proposals: 0,
        cancelled_proposals: 0,
        expired_proposals: 1,
        vetoed_proposals: 0,
      },
    },
    {
      label: 'proposal moves to Vetoed',
      stats: {
        total_proposals: 4,
        active_proposals: 1,
        queued_proposals: 0,
        executed_proposals: 1,
        failed_proposals: 1,
        cancelled_proposals: 0,
        expired_proposals: 0,
        vetoed_proposals: 1,
      },
    },
    {
      label: 'mix of all terminal statuses',
      stats: {
        total_proposals: 10,
        active_proposals: 2,
        queued_proposals: 1,
        executed_proposals: 3,
        failed_proposals: 1,
        cancelled_proposals: 1,
        expired_proposals: 1,
        vetoed_proposals: 1,
      },
    },
  ];

  for (const { label, stats } of scenarios) {
    test(`invariant holds when: ${label}`, async () => {
      serveStats(stats);
      const result = await client.getGovernanceStats();
      expect(invariantHolds(result)).toBe(true);
      expect(result.totalProposals).toBe(stats.total_proposals);
    });
  }
});
