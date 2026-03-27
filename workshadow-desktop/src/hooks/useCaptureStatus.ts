import { useState, useEffect, useCallback } from "react";
import { ipc, CaptureStatus } from "../lib/ipc";

export function useCaptureStatus(pollIntervalMs = 2000) {
  const [status, setStatus] = useState<CaptureStatus>({
    state: "Idle",
    session_id: null,
    frames_captured: 0,
    current_fps: 0,
    recording_since_ms: null,
  });

  const refresh = useCallback(async () => {
    try {
      const s = await ipc.getCaptureStatus();
      setStatus(s);
    } catch (err) {
      console.error("Failed to get capture status:", err);
    }
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, pollIntervalMs);
    return () => clearInterval(interval);
  }, [refresh, pollIntervalMs]);

  const startCapture = useCallback(async () => {
    await ipc.startCapture();
    await refresh();
  }, [refresh]);

  const pauseCapture = useCallback(async () => {
    await ipc.pauseCapture();
    await refresh();
  }, [refresh]);

  return { status, startCapture, pauseCapture, refresh };
}
