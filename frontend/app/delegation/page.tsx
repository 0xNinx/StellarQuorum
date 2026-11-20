import Link from "next/link";
import { formatDate, formatNumber, t } from "@/lib/i18n";

type DelegationStatus = "active" | "revoked";

const HISTORY: { date: string; delegator: string; delegate: string; amount: number; status: DelegationStatus }[] = [
  { date: "2026-05-14T09:30:00Z", delegator: "GABC...3XZK", delegate: "GDEF...8YPQ", amount: 150000, status: "active" },
  { date: "2026-04-02T12:00:00Z", delegator: "GABC...3XZK", delegate: "GHIJ...2WNM", amount: 75000, status: "revoked" },
];

export default function DelegationPage() {
  return (
    <main className="mx-auto max-w-5xl px-4 py-10 sm:px-6">
      <Link href="/" className="text-sm text-blue-400 hover:text-blue-300">← {t("common.quorum")}</Link>
      <h1 className="mt-6 text-3xl font-bold text-white">{t("delegation.title")}</h1>
      <p className="mt-2 text-slate-400">{t("delegation.intro")}</p>
      <div className="mt-8 overflow-x-auto rounded-xl border border-[#1a2535] bg-[#0d1520]">
        <table className="w-full text-left text-sm">
          <caption className="sr-only">{t("delegation.title")}</caption>
          <thead className="border-b border-[#1a2535] text-slate-400"><tr><th className="p-4">{t("delegation.date")}</th><th className="p-4">{t("delegation.delegator")}</th><th className="p-4">{t("delegation.delegate")}</th><th className="p-4">{t("delegation.amount")}</th><th className="p-4">{t("delegation.status")}</th></tr></thead>
          <tbody>{HISTORY.map(row => <tr key={`${row.date}-${row.delegate}`} className="border-b border-[#1a2535] last:border-0"><td className="p-4 text-slate-300">{formatDate(row.date)}</td><td className="p-4 font-mono text-slate-400">{row.delegator}</td><td className="p-4 font-mono text-slate-400">{row.delegate}</td><td className="p-4 text-white">{t("delegation.amountValue", { amount: formatNumber(row.amount) })}</td><td className="p-4 text-slate-300">{t(`delegation.statuses.${row.status}`)}</td></tr>)}</tbody>
        </table>
      </div>
    </main>
  );
}
