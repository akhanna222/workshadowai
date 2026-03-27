import { useState } from "react";

interface Props {
  onComplete: () => void;
}

const steps = [
  {
    title: "Welcome to WorkShadow AI",
    description:
      "WorkShadow captures your screen activity, extracts text via OCR, and gives you a searchable timeline of everything you've seen — all 100% local on your device.",
    detail: "No data ever leaves your machine. No cloud. No telemetry.",
  },
  {
    title: "Screen Recording Permission",
    description:
      "WorkShadow needs screen recording access to capture your display. This is the core of how it works.",
    detail:
      "On macOS: System Settings → Privacy & Security → Screen Recording → Enable WorkShadow.\nOn Windows: No extra permissions needed.\nOn Linux: PipeWire portal or X11 access required.",
  },
  {
    title: "Privacy Controls",
    description:
      "You're in full control of what gets captured. Certain apps are excluded by default.",
    detail:
      "Default exclusions: 1Password, Bitwarden, LastPass, KeePass, banking sites, Signal, WhatsApp.\n\nYou can add or remove exclusions anytime in Settings.\n\nPress Ctrl+Shift+P (Cmd+Shift+P on Mac) to instantly pause recording.",
  },
  {
    title: "Storage & Encryption",
    description:
      "All captured data is encrypted at rest using AES-256-GCM. Your encryption key is stored in the OS keychain.",
    detail:
      "Default retention: 90 days or 50 GB (whichever comes first).\nStorage: ~350 MB per hour of recording at 1 fps.\nYou can delete any time range at any point from the Summary view.",
  },
  {
    title: "Ready to Go",
    description:
      "WorkShadow is ready. Click 'Start Recording' in the header to begin capturing your activity.",
    detail:
      "Tips:\n• Use the Search view to find anything you've seen\n• Check the Timeline for a visual history of your day\n• The Summary view shows app usage analytics\n• Settings let you tune capture rate, OCR, and privacy",
  },
];

export function Onboarding({ onComplete }: Props) {
  const [step, setStep] = useState(0);
  const current = steps[step];
  const isLast = step === steps.length - 1;

  return (
    <div className="flex items-center justify-center h-screen bg-[var(--ws-bg)]">
      <div className="max-w-lg w-full p-8">
        {/* Progress dots */}
        <div className="flex gap-2 justify-center mb-8">
          {steps.map((_, i) => (
            <span
              key={i}
              className="w-2 h-2 rounded-full transition-colors"
              style={{
                backgroundColor:
                  i === step ? "var(--ws-accent)" : i < step ? "var(--ws-ide)" : "var(--ws-border)",
              }}
            />
          ))}
        </div>

        {/* Content */}
        <h2 className="text-2xl font-bold mb-4 text-center">{current.title}</h2>
        <p className="text-[var(--ws-text)] text-center mb-4">{current.description}</p>
        <pre className="text-sm text-[var(--ws-text-muted)] bg-[var(--ws-surface)] rounded-lg p-4 mb-8 whitespace-pre-wrap font-sans">
          {current.detail}
        </pre>

        {/* Navigation */}
        <div className="flex justify-between">
          <button
            onClick={() => setStep(Math.max(0, step - 1))}
            disabled={step === 0}
            className="px-4 py-2 text-sm text-[var(--ws-text-muted)] hover:text-[var(--ws-text)] disabled:opacity-30 transition-colors"
          >
            Back
          </button>
          <button
            onClick={() => {
              if (isLast) {
                onComplete();
              } else {
                setStep(step + 1);
              }
            }}
            className="px-6 py-2 bg-[var(--ws-accent)] hover:bg-[var(--ws-accent-hover)] text-white rounded-lg text-sm font-medium transition-colors"
          >
            {isLast ? "Get Started" : "Next"}
          </button>
        </div>
      </div>
    </div>
  );
}
