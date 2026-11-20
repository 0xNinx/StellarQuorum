// Issue #279: preferences for the settings page. They are kept in this browser
// only, so a failed read or write falls back to the defaults rather than
// blocking the page.

/** One localStorage entry for every preference on the settings page. */
export const SETTINGS_KEY = "quorum:settings:v1";

/** Written by the settings page and read by ProposalList, so a change applies on the next render. */
export const DENSITY_KEY = "proposal-density";

export interface Settings {
  /** Optional RPC endpoint override. Empty means the deployment's default. */
  rpcUrl: string;
  notifyStatusChanges: boolean;
}

export const DEFAULT_SETTINGS: Settings = { rpcUrl: "", notifyStatusChanges: true };

/** Empty is allowed, since it means "keep the default". Anything else must be an absolute http or https URL. */
export function isValidRpcUrl(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed) return true;
  try {
    const { protocol } = new URL(trimmed);
    return protocol === "http:" || protocol === "https:";
  } catch {
    return false;
  }
}

/**
 * Issue #510: the CSP `connect-src` (next.config.ts) only permits
 * `https://*.stellar.org` and the deployment's own RPC origin, so a saved
 * override outside those would be blocked by the browser. Empty is allowed.
 */
export function isRpcUrlAllowedByCsp(
  value: string,
  deploymentRpcUrl: string | undefined = process.env.NEXT_PUBLIC_STELLAR_RPC_URL,
): boolean {
  const trimmed = value.trim();
  if (!trimmed) return true;
  try {
    const url = new URL(trimmed);
    if (url.protocol === "https:" && url.hostname.endsWith(".stellar.org")) return true;
    return !!deploymentRpcUrl && url.origin === new URL(deploymentRpcUrl).origin;
  } catch {
    return false;
  }
}
