import { useEffect, useState } from "react";
import {
  ipc,
  toAppError,
  type AppError,
  type DiagnosticsSnapshot,
  type Settings as SettingsType,
} from "../lib/ipc";
import { ErrorBanner } from "../components/ErrorBanner";

export function Settings() {
  const [settings, setSettings] = useState<SettingsType | null>(null);
  const [snapshot, setSnapshot] = useState<DiagnosticsSnapshot | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [cleared, setCleared] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  useEffect(() => {
    void ipc.getSettings().then(setSettings);
    void ipc.diagnosticsSnapshot().then(setSnapshot);
  }, []);

  const update = async (patch: Partial<SettingsType>) => {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    try {
      await ipc.saveSettings(next);
    } catch (e) {
      setError(toAppError(e));
    }
  };

  const clearAll = async () => {
    if (!confirmClear) {
      setConfirmClear(true);
      return;
    }
    try {
      await ipc.clearConversations();
      setCleared(true);
      setConfirmClear(false);
      setTimeout(() => setCleared(false), 3000);
    } catch (e) {
      setError(toAppError(e));
    }
  };

  if (!settings) return null;

  return (
    <div className="mx-auto flex h-full max-w-3xl flex-col gap-6 overflow-y-auto px-8 py-6">
      <header>
        <h1 className="text-xl font-semibold">Settings</h1>
      </header>

      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}

      <Section title="Privacy">
        <p className="text-sm text-brand-textMuted">
          Omnira is local-first. Your conversations, settings, and model list
          stay on this computer. There is no telemetry, no account, no cloud
          sync, and no network access by default. Omnira works fully offline.
        </p>
      </Section>

      <Section title="Appearance">
        <label className="flex items-center justify-between text-sm">
          Theme
          <select
            value={settings.theme}
            onChange={(e) => void update({ theme: e.target.value })}
            className="rounded-lg border border-brand-border bg-brand-card px-3 py-1.5 text-sm outline-none focus:border-accent-primary/50"
          >
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </label>
      </Section>

      <Section title="Where your data lives">
        <dl className="space-y-2 text-sm">
          <PathRow label="All Omnira data" value={snapshot?.data_dir} />
          <PathRow label="Conversations database" value={snapshot?.db_path} />
          <PathRow label="Settings file" value={snapshot?.config_path} />
          <PathRow label="Logs" value={snapshot?.log_dir} />
        </dl>
        <p className="mt-2 text-xs text-zinc-600">
          Model files are never stored here -- Omnira references them where they
          already are.
        </p>
      </Section>

      <Section title="Advanced runtime">
        <label className="block text-sm">
          <span className="text-brand-textMuted">
            Custom runtime path (optional). Leave empty to use the engine that
            ships with Omnira.
          </span>
          <input
            type="text"
            value={settings.runtime_path_override ?? ""}
            onChange={(e) =>
              void update({ runtime_path_override: e.target.value.trim() || null })
            }
            placeholder="C:\path\to\llama-server.exe"
            className="mt-2 w-full rounded-lg border border-brand-border bg-brand-card px-3 py-2 text-sm outline-none placeholder:text-zinc-700 focus:border-accent-primary/50"
          />
        </label>
      </Section>

      <Section title="Your data">
        <div className="flex items-center justify-between">
          <p className="text-sm text-brand-textMuted">
            Delete all conversations stored on this computer.
          </p>
          <button
            onClick={() => void clearAll()}
            className={`rounded-lg px-4 py-2 text-sm font-medium ${
              confirmClear
                ? "bg-accent-danger text-white hover:bg-accent-danger/90"
                : "border border-accent-danger/40 text-accent-danger hover:bg-accent-danger/10"
            }`}
          >
            {cleared
              ? "Cleared"
              : confirmClear
                ? "Click again to confirm"
                : "Clear all conversations"}
          </button>
        </div>
      </Section>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="rounded-xl border border-brand-border bg-brand-card p-5">
      <h2 className="mb-3 text-sm font-semibold">{title}</h2>
      {children}
    </section>
  );
}

function PathRow({ label, value }: { label: string; value?: string }) {
  return (
    <div className="flex items-baseline justify-between gap-4">
      <dt className="shrink-0 text-brand-textMuted">{label}</dt>
      <dd className="truncate font-mono text-xs text-zinc-500" title={value}>
        {value ?? "..."}
      </dd>
    </div>
  );
}
