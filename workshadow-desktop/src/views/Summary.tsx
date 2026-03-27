import { useState, useEffect } from "react";
import { ipc, StorageUsage } from "../lib/ipc";

export function Summary() {
  const [storage, setStorage] = useState<StorageUsage | null>(null);

  useEffect(() => {
    ipc.getStorageUsage().then(setStorage).catch(console.error);
  }, []);

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
  };

  return (
    <div>
      <h2 className="text-lg font-semibold mb-4">Activity Summary</h2>

      {/* Storage Stats */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <StatCard
          label="Total Frames"
          value={storage?.total_frames.toLocaleString() ?? "-"}
        />
        <StatCard
          label="Sessions"
          value={storage?.total_sessions.toLocaleString() ?? "-"}
        />
        <StatCard
          label="Disk Usage"
          value={storage ? formatBytes(storage.disk_usage_bytes) : "-"}
        />
      </div>

      {/* Placeholder for charts */}
      <div className="p-8 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded-lg text-center">
        <p className="text-[var(--ws-text-muted)] text-lg mb-2">Activity charts coming soon</p>
        <p className="text-[var(--ws-text-muted)] text-sm">
          Daily breakdown by application, top URLs, and most-used windows will appear here.
        </p>
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-4 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded-lg">
      <p className="text-xs text-[var(--ws-text-muted)] uppercase tracking-wider mb-1">{label}</p>
      <p className="text-2xl font-semibold">{value}</p>
    </div>
  );
}
