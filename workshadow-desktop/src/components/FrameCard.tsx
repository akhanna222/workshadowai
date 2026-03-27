import { SearchResult } from "../lib/ipc";

interface Props {
  result: SearchResult;
  onClick?: () => void;
}

export function FrameCard({ result, onClick }: Props) {
  const time = new Date(result.timestamp_ms).toLocaleString();

  return (
    <div
      onClick={onClick}
      className="flex gap-3 p-3 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded-lg cursor-pointer hover:bg-[var(--ws-surface-hover)] transition-colors"
    >
      <div className="w-32 h-20 bg-[var(--ws-border)] rounded flex-shrink-0 flex items-center justify-center text-[var(--ws-text-muted)] text-xs">
        Preview
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <AppBadge appId={result.app_id} />
          <span className="text-sm font-medium truncate">
            {result.window_title || "Unknown Window"}
          </span>
        </div>
        <p className="text-sm text-[var(--ws-text-muted)] line-clamp-2 mb-1">
          {result.matched_text}
        </p>
        <div className="flex items-center gap-3 text-xs text-[var(--ws-text-muted)]">
          <span>{time}</span>
          <span>Score: {result.relevance_score.toFixed(2)}</span>
        </div>
      </div>
    </div>
  );
}

function AppBadge({ appId }: { appId: string }) {
  const color = appId.includes("browser") || appId.includes("chrome") || appId.includes("firefox")
    ? "var(--ws-browser)"
    : appId.includes("code") || appId.includes("idea")
      ? "var(--ws-ide)"
      : appId.includes("terminal") || appId.includes("iterm")
        ? "var(--ws-terminal)"
        : "var(--ws-text-muted)";

  return (
    <span
      className="w-2 h-2 rounded-full flex-shrink-0"
      style={{ backgroundColor: color }}
    />
  );
}
