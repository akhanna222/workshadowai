import { useState } from "react";

interface Props {
  onApplyFilters: (filters: {
    dateFrom?: string;
    dateTo?: string;
    appFilter?: string;
  }) => void;
}

export function FilterPanel({ onApplyFilters }: Props) {
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [appFilter, setAppFilter] = useState("");

  return (
    <div className="flex gap-3 items-end flex-wrap">
      <div>
        <label className="block text-xs text-[var(--ws-text-muted)] mb-1">From</label>
        <input
          type="date"
          value={dateFrom}
          onChange={(e) => setDateFrom(e.target.value)}
          className="px-3 py-1.5 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded text-sm text-[var(--ws-text)] focus:outline-none focus:border-[var(--ws-accent)]"
        />
      </div>
      <div>
        <label className="block text-xs text-[var(--ws-text-muted)] mb-1">To</label>
        <input
          type="date"
          value={dateTo}
          onChange={(e) => setDateTo(e.target.value)}
          className="px-3 py-1.5 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded text-sm text-[var(--ws-text)] focus:outline-none focus:border-[var(--ws-accent)]"
        />
      </div>
      <div>
        <label className="block text-xs text-[var(--ws-text-muted)] mb-1">App</label>
        <input
          type="text"
          value={appFilter}
          onChange={(e) => setAppFilter(e.target.value)}
          placeholder="e.g. chrome"
          className="px-3 py-1.5 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded text-sm text-[var(--ws-text)] placeholder-[var(--ws-text-muted)] focus:outline-none focus:border-[var(--ws-accent)]"
        />
      </div>
      <button
        onClick={() => onApplyFilters({ dateFrom, dateTo, appFilter })}
        className="px-4 py-1.5 bg-[var(--ws-surface)] border border-[var(--ws-border)] hover:bg-[var(--ws-surface-hover)] rounded text-sm transition-colors"
      >
        Apply
      </button>
    </div>
  );
}
