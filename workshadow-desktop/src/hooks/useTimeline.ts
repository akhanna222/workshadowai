import { useState, useCallback } from "react";
import { ipc, FrameSummary } from "../lib/ipc";

export function useTimeline() {
  const [frames, setFrames] = useState<FrameSummary[]>([]);
  const [loading, setLoading] = useState(false);

  const loadRange = useCallback(async (startMs: number, endMs: number) => {
    setLoading(true);
    try {
      const f = await ipc.getTimelineRange(startMs, endMs);
      setFrames(f);
    } catch (err) {
      console.error("Failed to load timeline:", err);
      setFrames([]);
    } finally {
      setLoading(false);
    }
  }, []);

  return { frames, loading, loadRange };
}
