import { getProposals } from "./proposals";
import type { Proposal } from "./types";

/**
 * How long one RPC read of the proposal list is shared between requests.
 *
 * `/api/proposals` is read by every open notification center, so without a
 * shared read the RPC load grows with the number of visitors. Fifteen seconds
 * keeps that load flat while a status change still shows up quickly.
 */
export const PROPOSALS_TTL_MS = 15_000;

export interface TtlCache<T> {
  /** The cached value, or a fresh one once it has expired. */
  get(): Promise<T>;
  /** Forget the cached value and any read in flight. */
  clear(): void;
}

/**
 * Caches the result of `load` for `ttlMs`.
 *
 * Concurrent callers share one in-flight `load`, so a burst of requests on an
 * empty or expired cache still makes a single read. A failed `load` is not
 * cached: the error goes to everyone waiting on it, and the next call tries
 * again.
 */
export function createTtlCache<T>(
  load: () => Promise<T>,
  ttlMs: number,
  now: () => number = Date.now,
): TtlCache<T> {
  let cached: { value: T; expiresAt: number } | null = null;
  let inflight: Promise<T> | null = null;

  return {
    get() {
      if (cached && now() < cached.expiresAt) return Promise.resolve(cached.value);
      if (!inflight) {
        const read = load().then(
          value => {
            if (inflight === read) {
              cached = { value, expiresAt: now() + ttlMs };
              inflight = null;
            }
            return value;
          },
          error => {
            if (inflight === read) inflight = null;
            throw error;
          },
        );
        inflight = read;
      }
      return inflight;
    },
    clear() {
      cached = null;
      inflight = null;
    },
  };
}

const proposalsCache = createTtlCache<Proposal[]>(() => getProposals(), PROPOSALS_TTL_MS);

/** The proposal list, read from RPC at most once per `PROPOSALS_TTL_MS`. */
export function getCachedProposals(): Promise<Proposal[]> {
  return proposalsCache.get();
}

/** Drops the cached list. For tests. */
export function clearProposalsCache(): void {
  proposalsCache.clear();
}
