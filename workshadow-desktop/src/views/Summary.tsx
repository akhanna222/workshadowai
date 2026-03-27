import { useState, useEffect } from "react";
import { ipc, StorageUsage } from "../lib/ipc";

export function Summary() {
  const [storage, setStorage] = useState<StorageUsage | null>(null);
  const [auditLog, setAuditLog] = useState<{ timestamp: string; event: string }[]>([]);
  const [deleteFrom, setDeleteFrom] = useState("");
  const [deleteTo, setDeleteTo] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [deleteResult, setDeleteResult] = useState<string | null>(null);
  const [privacyStatus, setPrivacyStatus] = useState<{
    encryption_active: boolean;
    excluded_apps_count: number;
    excluded_url_patterns_count: number;
    audit_log_entries: number;
  } | null>(null);

  useEffect(() => {
    ipc.getStorageUsage().then(setStorage).catch(console.error);
    ipc.getAuditLog().then(setAuditLog).catch(console.error);
    ipc.getPrivacyStatus().then(setPrivacyStatus).catch(console.error);
  }, []);

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
  };

  const handleDelete = async () => {
    if (!deleteFrom || !deleteTo) return;
    setDeleting(true);
    setDeleteResult(null);
    try {
      const startMs = new Date(deleteFrom).getTime();
      const endMs = new Date(deleteTo).getTime() + 86400000; // end of day
      const deleted = await ipc.deleteTimeRange(startMs, endMs);
      setDeleteResult(`Deleted ${deleted} frames.`);
      // Refresh stats
      const s = await ipc.getStorageUsage();
      setStorage(s);
      const a = await ipc.getAuditLog();
      setAuditLog(a);
    } catch (err) {
      setDeleteResult(`Error: ${err}`);
    } finally {
      setDeleting(false);
    }
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

      {/* Privacy Status */}
      {privacyStatus && (
        <div className="grid grid-cols-3 gap-4 mb-6">
          <StatCard
            label="Encryption"
            value={privacyStatus.encryption_active ? "Active (AES-256)" : "Inactive"}
          />
          <StatCard
            label="Excluded Apps"
            value={privacyStatus.excluded_apps_count.toString()}
          />
          <StatCard
            label="Audit Events"
            value={privacyStatus.audit_log_entries.toString()}
          />
        </div>
      )}

      {/* Data Deletion */}
      <div className="mb-6 p-4 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded-lg">
        <h3 className="text-sm font-medium mb-3 text-[var(--ws-text-muted)] uppercase tracking-wider">
          Delete Data
        </h3>
        <div className="flex gap-3 items-end flex-wrap">
          <div>
            <label className="block text-xs text-[var(--ws-text-muted)] mb-1">From</label>
            <input
              type="date"
              value={deleteFrom}
              onChange={(e) => setDeleteFrom(e.target.value)}
              className="px-3 py-1.5 bg-[var(--ws-bg)] border border-[var(--ws-border)] rounded text-sm text-[var(--ws-text)]"
            />
          </div>
          <div>
            <label className="block text-xs text-[var(--ws-text-muted)] mb-1">To</label>
            <input
              type="date"
              value={deleteTo}
              onChange={(e) => setDeleteTo(e.target.value)}
              className="px-3 py-1.5 bg-[var(--ws-bg)] border border-[var(--ws-border)] rounded text-sm text-[var(--ws-text)]"
            />
          </div>
          <button
            onClick={handleDelete}
            disabled={deleting || !deleteFrom || !deleteTo}
            className="px-4 py-1.5 bg-red-600 hover:bg-red-700 disabled:opacity-50 text-white rounded text-sm transition-colors"
          >
            {deleting ? "Deleting..." : "Delete Range"}
          </button>
        </div>
        {deleteResult && (
          <p className="mt-2 text-sm text-[var(--ws-text-muted)]">{deleteResult}</p>
        )}
      </div>

      {/* Audit Log */}
      <div className="mb-6 p-4 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded-lg">
        <h3 className="text-sm font-medium mb-3 text-[var(--ws-text-muted)] uppercase tracking-wider">
          Audit Log
        </h3>
        {auditLog.length === 0 ? (
          <p className="text-sm text-[var(--ws-text-muted)]">No audit events yet.</p>
        ) : (
          <div className="space-y-1 max-h-48 overflow-y-auto">
            {auditLog.map((entry, i) => (
              <div key={i} className="flex gap-3 text-xs">
                <span className="text-[var(--ws-text-muted)] flex-shrink-0 w-40">
                  {new Date(entry.timestamp).toLocaleString()}
                </span>
                <span className="text-[var(--ws-text)] font-mono truncate">
                  {entry.event}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Placeholder for charts */}
      <div className="p-8 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded-lg text-center">
        <p className="text-[var(--ws-text-muted)] text-lg mb-2">Activity charts coming soon</p>
        <p className="text-[var(--ws-text-muted)] text-sm">
          Daily breakdown by application, top URLs, and most-used windows.
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
