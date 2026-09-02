import { useCallback, useEffect, useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { FirstRun } from "./components/FirstRun";
import { Chat } from "./pages/Chat";
import { Models } from "./pages/Models";
import { Settings } from "./pages/Settings";
import { Diagnostics } from "./pages/Diagnostics";
import { ipc, type RuntimeStatus, type Settings as SettingsType } from "./lib/ipc";
import { isScreenMounted, type Screen } from "./lib/screens";
import { applyTheme } from "./lib/theme";

import { BrandMark } from "./components/BrandMark";

const stoppedStatus: RuntimeStatus = {
  state: "stopped",
  engine_label: null,
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

  // Settings is the single owner of the theme value; every screen that saves
  // settings reports back through setSettings so this applies immediately.
  useEffect(() => {
    applyTheme(document.documentElement, settings?.theme);
  }, [settings?.theme]);

  if (!settings) {
    return (
      <div className="flex h-full items-center justify-center bg-brand-deep">
        <div className="flex flex-col items-center gap-4 animate-pulse">
          <BrandMark size="lg" />
        </div>
      </div>
    );
  }

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

  const chatVisible = screen === "chat";

  return (
    <div className="flex h-full">
      <Sidebar active={screen} onSelect={setScreen} />
      <main className="min-w-0 flex-1">
        {/* Keep Chat mounted so an in-flight generation survives screen changes. */}
        {isScreenMounted("chat", screen) && (
          <div
            className={chatVisible ? "h-full" : "hidden"}
            aria-hidden={!chatVisible}
          >
            <Chat
              visible={chatVisible}
              runtime={runtime}
              refreshRuntime={refreshRuntime}
              onGoToModels={() => setScreen("models")}
              onGoToDiagnostics={() => setScreen("diagnostics")}
            />
          </div>
        )}
        {isScreenMounted("models", screen) && (
          <Models runtime={runtime} refreshRuntime={refreshRuntime} />
        )}
        {isScreenMounted("settings", screen) && (
          <Settings settings={settings} onSettingsSaved={setSettings} />
        )}
        {isScreenMounted("diagnostics", screen) && <Diagnostics />}
      </main>
    </div>
  );
}
