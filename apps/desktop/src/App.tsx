import { useCallback, useEffect, useState } from "react";
import { Sidebar, type Screen } from "./components/Sidebar";
import { FirstRun } from "./components/FirstRun";
import { Chat } from "./pages/Chat";
import { Models } from "./pages/Models";
import { Settings } from "./pages/Settings";
import { Diagnostics } from "./pages/Diagnostics";
import { ipc, type RuntimeStatus, type Settings as SettingsType } from "./lib/ipc";

const stoppedStatus: RuntimeStatus = {
  state: "stopped",
  variant: null,
  accelerator_label: null,
  fallback_reason: null,
  model_id: null,
  port: null,
  context_size: null,
  last_error: null,
};

export default function App() {
  const [screen, setScreen] = useState<Screen>("chat");
  const [settings, setSettings] = useState<SettingsType | null>(null);
  const [runtime, setRuntime] = useState<RuntimeStatus>(stoppedStatus);

  const refreshRuntime = useCallback(async () => {
    try {
      setRuntime(await ipc.runtimeStatus());
    } catch {
      setRuntime(stoppedStatus);
    }
  }, []);

  useEffect(() => {
    void ipc.getSettings().then(setSettings);
    void refreshRuntime();
    const t = setInterval(() => void refreshRuntime(), 4000);
    return () => clearInterval(t);
  }, [refreshRuntime]);

  useEffect(() => {
    document.body.classList.toggle("light-theme", settings?.theme === "light");
  }, [settings?.theme]);

  if (!settings) return null;

  if (!settings.onboarding_complete) {
    return (
      <FirstRun
        settings={settings}
        onComplete={() =>
          setSettings({ ...settings, onboarding_complete: true })
        }
      />
    );
  }

  return (
    <div className="flex h-full">
      <Sidebar active={screen} onSelect={setScreen} />
      <main className="min-w-0 flex-1">
        {screen === "chat" && (
          <Chat
            runtime={runtime}
            refreshRuntime={refreshRuntime}
            onGoToModels={() => setScreen("models")}
          />
        )}
        {screen === "models" && (
          <Models runtime={runtime} refreshRuntime={refreshRuntime} />
        )}
        {screen === "settings" && <Settings />}
        {screen === "diagnostics" && <Diagnostics />}
      </main>
    </div>
  );
}
