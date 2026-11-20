'use client';

import { Calendar } from "lucide-react";
import type { Proposal } from "@/lib/types";
import { t } from "@/lib/i18n";

/**
 * "Add to calendar" button for proposal deadlines.
 *
 * Issue #289: generates an iCalendar (.ics) file with the voting end time
 * and, for queued proposals, the moment the timelock expires and the proposal
 * becomes executable. The .ics file is downloaded client-side via a Blob URL —
 * no server route needed, as the proposal data is already available in the
 * browser.
 *
 * The button is only rendered when a voting end time is present.
 */
export default function AddToCalendar({ proposal }: { proposal: Proposal }) {
  if (!proposal.endTime) {
    return null;
  }

  const handleDownload = () => {
    const icsContent = generateIcs(proposal);
    const blob = new Blob([icsContent], { type: 'text/calendar;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `proposal-${proposal.id}.ics`;
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
    URL.revokeObjectURL(url);
  };

  return (
    <button
      onClick={handleDownload}
      aria-label={t("proposals.addToCalendar")}
      className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#162032] hover:bg-[#1a2a40] text-slate-300 text-sm border border-[#1a2535] transition-colors"
    >
      <Calendar size={14} />
      <span className="hidden sm:inline">{t("proposals.addToCalendarShort")}</span>
    </button>
  );
}

/**
 * Generates an RFC 5545 iCalendar file for a proposal's deadlines.
 *
 * Issue #289: the voting end time and the timelock expiry (`queueTime`, set
 * only once a proposal is queued) are already ISO 8601 strings in the Proposal
 * data model, produced by the ledger-to-date conversion in `toUiProposal()`.
 * No additional conversion is needed here — we only need to format them as
 * iCalendar DTSTART/DTEND values.
 *
 * @param proposal The proposal to generate calendar events for.
 * @returns RFC 5545 compliant iCalendar content.
 */
function generateIcs(proposal: Proposal): string {
  const now = new Date();
  
  // DTSTAMP and DTSTART/DTEND in UTC format: yyyyMMddTHHmmssZ
  const formatIcalDate = (date: Date): string => {
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${date.getUTCFullYear()}${pad(date.getUTCMonth() + 1)}${pad(date.getUTCDate())}T${pad(date.getUTCHours())}${pad(date.getUTCMinutes())}${pad(date.getUTCSeconds())}Z`;
  };

  const dtstamp = formatIcalDate(now);
  
  // Truncate description to 200 chars for calendar apps
  const description = escapeIcal(proposal.description.slice(0, 200) + (proposal.description.length > 200 ? '...' : ''));
  const url = `https://quorum.stellar.org/proposals/${proposal.id}`;

  // Each event is a one-hour block starting at the deadline.
  const vevent = (uid: string, start: string, summary: string): string => {
    const startDate = new Date(start);
    const endDate = new Date(startDate.getTime() + 60 * 60 * 1000);
    return `BEGIN:VEVENT
UID:${uid}
DTSTAMP:${dtstamp}
DTSTART:${formatIcalDate(startDate)}
DTEND:${formatIcalDate(endDate)}
SUMMARY:${escapeIcal(summary)}
DESCRIPTION:${description}
URL:${url}
END:VEVENT
`;
  };
  
  let icsContent = `BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//StellarQuorum//StellarQuorum//EN
`;

  icsContent += vevent(`${proposal.id}@stellarquorum`, proposal.endTime, `Vote on: ${proposal.title}`);

  // Queued proposals also get an event for when the timelock expires. The UI
  // shows a queued proposal as "passed"; `queueTime` is only set once it has
  // been queued on-chain, and an executed proposal no longer needs a reminder.
  if (proposal.status === 'passed' && proposal.queueTime) {
    icsContent += vevent(`${proposal.id}-executable@stellarquorum`, proposal.queueTime, `Executable: ${proposal.title}`);
  }

  icsContent += 'END:VCALENDAR\r\n';
  
  return icsContent;
}

/**
 * Escapes special characters in iCalendar text fields per RFC 5545.
 *
 * Commas, semicolons, and newlines must be escaped. Proposal titles and
 * descriptions are user-generated content and may contain these characters.
 */
function escapeIcal(text: string): string {
  return text
    .replace(/\\/g, '\\\\')
    .replace(/;/g, '\\;')
    .replace(/,/g, '\\,')
    .replace(/\n/g, '\\n');
}


  const readIcs = (): Promise<string> =>
    new Promise((resolve) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.readAsText(createObjectURLSpy.mock.calls[0][0] as Blob);
    });

  it("adds an executable event for queued proposals", async () => {
    render(<AddToCalendar proposal={{ ...mockProposal, status: "passed", queueTime: "2026-05-21T10:00:00Z" }} />);
    fireEvent.click(screen.getByRole("button", { name: /add voting deadline to calendar/i }));

    const ics = await readIcs();
    expect(ics.match(/BEGIN:VEVENT/g)).toHaveLength(2);
    expect(ics).toContain("UID:QIP-001-executable@stellarquorum");
    expect(ics).toContain("SUMMARY:Executable: Test Proposal");
    expect(ics).toContain("DTSTART:20260521T100000Z");
    expect(ics).toContain("DTEND:20260521T110000Z");
  });

  it("emits only the voting event when the proposal is not queued", async () => {
    render(<AddToCalendar proposal={{ ...mockProposal, status: "executed", queueTime: "2026-05-21T10:00:00Z" }} />);
    fireEvent.click(screen.getByRole("button", { name: /add voting deadline to calendar/i }));

    const ics = await readIcs();
    expect(ics.match(/BEGIN:VEVENT/g)).toHaveLength(1);
    expect(ics).not.toContain("Executable:");
  });
