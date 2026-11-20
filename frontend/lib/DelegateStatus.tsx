'use client';

import { useEffect, useState } from "react";
import Link from "next/link";
import { getDelegate } from "@/lib/delegation";
import { t } from "@/lib/i18n";

function shorten(address: string): string {
  // Fixture addresses are already shortened ("GDEF...8YPQ").
  if (address.length <= 10 || address.includes("...")) return address;
  return `…${address.slice(-6)}`;
}

/**
 * Navbar indicator of who votes with the connected address's tokens: "Self",
 * or the delegate's address. Links to /delegation. Renders nothing while the
 * read is in flight or if it fails, so a slow or broken RPC never clutters
 * the header.
 */
export default function DelegateStatus({ address }: { address: string }) {
  // Keyed by holder so a read for a previous account is never shown after a switch.
  const [result, setResult] = useState<{ holder: string; delegate: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    getDelegate(address)
      .then(delegate => {
        if (!cancelled) setResult({ holder: address, delegate });
      })
      .catch(() => {
        if (!cancelled) setResult(null);
      });
    return () => {
      cancelled = true;
    };
  }, [address]);

  if (!result || result.holder !== address) return null;

  const self = result.delegate === address;
  return (
    <Link
      href="/delegation"
      aria-label={self ? t("nav.delegateSelfLabel") : t("nav.delegateToLabel", { delegate: result.delegate })}
      className="px-2 py-1 rounded-md border border-[#1a2535] text-xs text-slate-300 hover:text-slate-100 hover:border-slate-600 focus-visible:text-slate-100 transition-colors"
    >
      {self ? t("nav.delegateSelf") : t("nav.delegateTo", { address: shorten(result.delegate) })}
    </Link>
  );
}
