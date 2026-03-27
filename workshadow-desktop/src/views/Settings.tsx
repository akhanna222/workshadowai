import { useState, useEffect } from "react";
import { ipc, AppConfig } from "../lib/ipc";

export function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    ipc.getSettings().then(setConfig).catch(console.error);
  }, []);

  const save = async () => {
    if (!config) return;
    setSaving(true);
    try {
      await ipc.updateSettings(config);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      console.error("Failed to save settings:", err);
    } finally {
      setSaving(false);
    }
  };

  if (!config) return <p className="text-[var(--ws-text-muted)]">Loading settings...</p>;

  return (
    <div className="max-w-2xl">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-lg font-semibold">Settings</h2>
        <button
          onClick={save}
          disabled={saving}
          className="px-4 py-2 bg-[var(--ws-accent)] hover:bg-[var(--ws-accent-hover)] disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
        >
          {saved ? "Saved!" : saving ? "Saving..." : "Save"}
        </button>
      </div>

      {/* Capture */}
      <Section title="Capture">
        <Field label="Frame Rate (fps)">
          <input
            type="number"
            step="0.5"
            min="0.5"
            max="5"
            value={config.capture.fps}
            onChange={(e) =>
              setConfig({ ...config, capture: { ...config.capture, fps: Number(e.target.value) } })
            }
          />
        </Field>
        <Field label="Idle Frame Rate (fps)">
          <input
            type="number"
            step="0.1"
            min="0.1"
            max="2"
            value={config.capture.idle_fps}
            onChange={(e) =>
              setConfig({ ...config, capture: { ...config.capture, idle_fps: Number(e.target.value) } })
            }
          />
        </Field>
        <Field label="Idle Threshold (seconds)">
          <input
            type="number"
            min="5"
            max="300"
            value={config.capture.idle_threshold_secs}
            onChange={(e) =>
              setConfig({
                ...config,
                capture: { ...config.capture, idle_threshold_secs: Number(e.target.value) },
              })
            }
          />
        </Field>
      </Section>

      {/* Storage */}
      <Section title="Storage">
        <Field label="Retention (days)">
          <input
            type="number"
            min="1"
            max="365"
            value={config.storage.max_retention_days}
            onChange={(e) =>
              setConfig({
                ...config,
                storage: { ...config.storage, max_retention_days: Number(e.target.value) },
              })
            }
          />
        </Field>
        <Field label="Max Storage (GB)">
          <input
            type="number"
            min="1"
            max="500"
            value={config.storage.max_storage_gb}
            onChange={(e) =>
              setConfig({
                ...config,
                storage: { ...config.storage, max_storage_gb: Number(e.target.value) },
              })
            }
          />
        </Field>
      </Section>

      {/* Privacy */}
      <Section title="Privacy">
        <Field label="Excluded Apps (comma-separated)">
          <input
            type="text"
            value={config.privacy.excluded_apps.join(", ")}
            onChange={(e) =>
              setConfig({
                ...config,
                privacy: {
                  ...config.privacy,
                  excluded_apps: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
                },
              })
            }
          />
        </Field>
        <Field label="Excluded URL Patterns">
          <input
            type="text"
            value={config.privacy.excluded_url_patterns.join(", ")}
            onChange={(e) =>
              setConfig({
                ...config,
                privacy: {
                  ...config.privacy,
                  excluded_url_patterns: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
                },
              })
            }
          />
        </Field>
        <div className="flex items-center gap-2 mt-2">
          <input
            type="checkbox"
            checked={config.ocr.pii_detection}
            onChange={(e) =>
              setConfig({ ...config, ocr: { ...config.ocr, pii_detection: e.target.checked } })
            }
            className="rounded"
          />
          <label className="text-sm">Enable PII Detection</label>
        </div>
      </Section>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mb-6 p-4 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded-lg">
      <h3 className="text-sm font-medium mb-3 text-[var(--ws-text-muted)] uppercase tracking-wider">
        {title}
      </h3>
      <div className="space-y-3">{children}</div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <label className="text-sm flex-shrink-0">{label}</label>
      <div className="[&>input]:px-3 [&>input]:py-1.5 [&>input]:bg-[var(--ws-bg)] [&>input]:border [&>input]:border-[var(--ws-border)] [&>input]:rounded [&>input]:text-sm [&>input]:text-[var(--ws-text)] [&>input]:focus:outline-none [&>input]:focus:border-[var(--ws-accent)] [&>input]:w-48">
        {children}
      </div>
    </div>
  );
}
