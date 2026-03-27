import { CaptureStatus } from "../lib/ipc";

interface Props {
  status: CaptureStatus;
  onToggle: () => void;
}

export function StatusIndicator({ status, onToggle }: Props) {
  const stateColor =
    status.state === "Recording"
      ? "var(--ws-recording)"
      : status.state === "Paused"
        ? "var(--ws-paused)"
        : "var(--ws-text-muted)";

  const stateLabel =
    status.state === "Recording"
      ? "Recording"
      : status.state === "Paused"
        ? "Paused"
        : "Idle";

  const buttonLabel =
    status.state === "Recording"
      ? "Pause"
      : status.state === "Paused"
        ? "Resume"
        : "Start";

  return (
    <div className="flex items-center gap-3">
      <div className="flex items-center gap-2">
        <span
          className="w-2.5 h-2.5 rounded-full"
          style={{
            backgroundColor: stateColor,
            boxShadow: status.state === "Recording" ? `0 0 8px ${stateColor}` : "none",
          }}
        />
        <span className="text-sm font-medium">{stateLabel}</span>
      </div>
      {status.state === "Recording" && (
        <span className="text-xs text-[var(--ws-text-muted)]">
          {status.frames_captured} frames
        </span>
      )}
      <button
        onClick={onToggle}
        className="px-3 py-1 text-xs border border-[var(--ws-border)] rounded hover:bg-[var(--ws-surface-hover)] transition-colors"
      >
        {buttonLabel}
      </button>
    </div>
  );
}
