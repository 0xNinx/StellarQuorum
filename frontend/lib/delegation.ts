import { USE_FIXTURE, createQuorumClient } from "./config";

/**
 * Delegations served in fixture mode, keyed by the last four characters of an
 * address. Matches the fixture history on /delegation, where ...3XZK delegated
 * to GDEF...8YPQ. Every other address votes with its own tokens.
 */
const FIXTURE_DELEGATES: Record<string, string> = {
  "3XZK": "GDEF...8YPQ",
};

/**
 * The account that votes with `address`'s tokens, including delegations made
 * directly on the token. The contract returns the holder itself when it has not
 * delegated, so a result equal to `address` means "Self".
 *
 * @throws Whatever the RPC read throws.
 */
export async function getDelegate(address: string): Promise<string> {
  if (USE_FIXTURE) return FIXTURE_DELEGATES[address.slice(-4)] ?? address;
  const client = await createQuorumClient();
  return client.getDelegate(address);
}
