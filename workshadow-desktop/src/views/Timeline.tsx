import { useEffect } from "react";
import { useTimeline } from "../hooks/useTimeline";
import { TimelineStrip } from "../components/TimelineStrip";

export function Timeline() {
  const { frames, loading, loadRange } = useTimeline();

  useEffect(() => {
    // Load today's frames
    const now = Date.now();
    const startOfDay = now - (now % 86400000);
    loadRange(startOfDay, now);
  }, [loadRange]);

  // Group frames by hour
  const hourGroups = new Map<string, typeof frames>();
  frames.forEach((frame) => {
    const hour = new Date(frame.timestamp_ms).getHours();
    const label = `${hour.toString().padStart(2, "0")}:00`;
    if (!hourGroups.has(label)) hourGroups.set(label, []);
    hourGroups.get(label)!.push(frame);
  });

  return (
    <div>
      <h2 className="text-lg font-semibold mb-4">Timeline</h2>
      {loading ? (
        <p className="text-[var(--ws-text-muted)]">Loading...</p>
      ) : frames.length === 0 ? (
        <div className="text-center py-16">
          <p className="text-[var(--ws-text-muted)] text-lg mb-2">No captures yet</p>
          <p className="text-[var(--ws-text-muted)] text-sm">
            Start recording to see your activity timeline here.
          </p>
        </div>
      ) : (
        Array.from(hourGroups.entries()).map(([hour, hourFrames]) => (
          <TimelineStrip key={hour} label={hour} frames={hourFrames} />
        ))
      )}
    </div>
  );
}
