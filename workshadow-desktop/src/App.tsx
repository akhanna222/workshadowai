import { useState, useEffect } from "react";
import { Timeline } from "./views/Timeline";
import { Search } from "./views/Search";
import { Settings } from "./views/Settings";
import { Summary } from "./views/Summary";
import { Onboarding } from "./views/Onboarding";
import { StatusIndicator } from "./components/StatusIndicator";
import { useCaptureStatus } from "./hooks/useCaptureStatus";
import "./index.css";

type View = "timeline" | "search" | "summary" | "settings";

const ONBOARDING_KEY = "workshadow_onboarded";

function App() {
  const [activeView, setActiveView] = useState<View>("timeline");
  const [showOnboarding, setShowOnboarding] = useState(false);
  const { status, startCapture, pauseCapture } = useCaptureStatus();

  useEffect(() => {
    if (!localStorage.getItem(ONBOARDING_KEY)) {
      setShowOnboarding(true);
    }
  }, []);

  const handleOnboardingComplete = () => {
    localStorage.setItem(ONBOARDING_KEY, "true");
    setShowOnboarding(false);
  };

  if (showOnboarding) {
    return <Onboarding onComplete={handleOnboardingComplete} />;
  }

  const handleToggle = () => {
    if (status.state === "Recording") {
      pauseCapture();
    } else {
      startCapture();
    }
  };

  const navItems: { id: View; label: string }[] = [
    { id: "timeline", label: "Timeline" },
    { id: "search", label: "Search" },
    { id: "summary", label: "Summary" },
    { id: "settings", label: "Settings" },
  ];

  return (
    <div className="flex flex-col h-screen">
      {/* Header */}
      <header className="flex items-center justify-between px-4 py-3 border-b border-[var(--ws-border)] bg-[var(--ws-surface)]">
        <div className="flex items-center gap-6">
          <h1 className="text-base font-bold tracking-tight">
            WorkShadow<span className="text-[var(--ws-accent)]"> AI</span>
          </h1>
          <nav className="flex gap-1">
            {navItems.map((item) => (
              <button
                key={item.id}
                onClick={() => setActiveView(item.id)}
                className={`px-3 py-1.5 text-sm rounded transition-colors ${
                  activeView === item.id
                    ? "bg-[var(--ws-accent)] text-white"
                    : "text-[var(--ws-text-muted)] hover:text-[var(--ws-text)] hover:bg-[var(--ws-surface-hover)]"
                }`}
              >
                {item.label}
              </button>
            ))}
          </nav>
        </div>
        <StatusIndicator status={status} onToggle={handleToggle} />
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-auto p-6">
        {activeView === "timeline" && <Timeline />}
        {activeView === "search" && <Search />}
        {activeView === "summary" && <Summary />}
        {activeView === "settings" && <Settings />}
      </main>
    </div>
  );
}

export default App;
