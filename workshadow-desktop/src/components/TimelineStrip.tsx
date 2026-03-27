import { FrameSummary } from "../lib/ipc";

interface Props {
  frames: FrameSummary[];
  label: string;
  onFrameClick?: (frameId: number) => void;
}

export function TimelineStrip({ frames, label, onFrameClick }: Props) {
  return (
    <div className="mb-4">
      <h3 className="text-xs font-medium text-[var(--ws-text-muted)] mb-2 uppercase tracking-wider">
        {label}
      </h3>
      <div className="flex gap-1 overflow-x-auto pb-2">
        {frames.length === 0 ? (
          <div className="text-sm text-[var(--ws-text-muted)] py-4">
            No frames captured for this period
          </div>
        ) : (
          frames.map((frame) => (
            <div
              key={frame.frame_id}
              onClick={() => onFrameClick?.(frame.frame_id)}
              className="w-16 h-10 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded cursor-pointer hover:border-[var(--ws-accent)] transition-colors flex-shrink-0 flex items-center justify-center"
              title={`${frame.window_title} - ${new Date(frame.timestamp_ms).toLocaleTimeString()}`}
            >
              <span className="text-[8px] text-[var(--ws-text-muted)] truncate px-0.5">
                {frame.app_id.split(".").pop()}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
