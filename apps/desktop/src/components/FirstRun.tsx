import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Shield, FolderOpen } from "lucide-react";
import { ipc, toAppError, type AppError, type Settings } from "../lib/ipc";
import { ErrorBanner } from "../components/ErrorBanner";

/**
 * First-run flow (docs/design-principles.md section 5):
 * welcome -> local-first explanation + data location -> pick a model or skip.
 */
export function FirstRun({
  settings,
  onComplete,
}: {
  settings: Settings;
  onComplete: () => void;
}) {
  const [step, setStep] = useState(0);
  const [dataDir, setDataDir] = useState("...");
  const [error, setError] = useState<AppError | null>(null);
  const [addedModel, setAddedModel] = useState<string | null>(null);

  useEffect(() => {
    void ipc.diagnosticsSnapshot().then((s) => setDataDir(s.data_dir));
  }, []);

  const finish = async () => {
    await ipc.saveSettings({ ...settings, onboarding_complete: true });
    onComplete();
  };

  const pickModel = async () => {
    setError(null);
    const selected = await open({
      multiple: false,
      title: "Choose a GGUF model file",
      filters: [{ name: "GGUF models", extensions: ["gguf"] }],
    });
    if (typeof selected !== "string") return;
    try {
      const model = await ipc.addModel(selected);
      setAddedModel(model.name);
    } catch (e) {
      setError(toAppError(e));
    }
  };

  return (
    <div className="flex h-full items-center justify-center bg-brand-deep">
      <div className="w-full max-w-lg rounded-2xl border border-brand-border bg-brand-card p-8">
        {step === 0 && (
          <div className="flex flex-col items-center gap-4 text-center">
            <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-accent-primary/20 text-2xl font-bold text-accent-primary">
              O
            </div>
            <h1 className="text-2xl font-semibold">Welcome to Omnira</h1>
            <p className="text-sm text-brand-textMuted">
              A simple, private way to chat with AI models on your own computer.
            </p>
            <button
              onClick={() => setStep(1)}
              className="mt-2 w-full rounded-xl bg-accent-primary py-2.5 text-sm font-medium text-white hover:bg-accent-primary/90"
            >
              Get started
            </button>
          </div>
        )}

        {step === 1 && (
          <div className="flex flex-col gap-4">
            <div className="flex items-center gap-3">
              <Shield size={20} className="text-accent-success" />
              <h1 className="text-lg font-semibold">Private by default</h1>
            </div>
            <p className="text-sm text-brand-textMuted">
              Everything Omnira does happens on this computer. Your
              conversations are stored locally, models run locally, and Omnira
              makes no network connections. It works fully offline.
            </p>
            <div className="rounded-xl border border-brand-border bg-brand-deep p-4">
              <p className="text-xs text-brand-textMuted">Your data will be stored at</p>
              <p className="mt-1 break-all font-mono text-xs">{dataDir}</p>
            </div>
            <button
              onClick={() => setStep(2)}
              className="w-full rounded-xl bg-accent-primary py-2.5 text-sm font-medium text-white hover:bg-accent-primary/90"
            >
              Sounds good
            </button>
          </div>
        )}

        {step === 2 && (
          <div className="flex flex-col gap-4">
            <div className="flex items-center gap-3">
              <FolderOpen size={20} className="text-accent-primary" />
              <h1 className="text-lg font-semibold">Add a model</h1>
            </div>
            <p className="text-sm text-brand-textMuted">
              Omnira chats using GGUF model files you already have. Choose one
              now, or skip and add it later from the Models screen.
            </p>
            {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}
            {addedModel ? (
              <p className="rounded-xl border border-accent-success/30 bg-accent-success/10 px-4 py-3 text-sm">
                Added <span className="font-medium">{addedModel}</span>. You are
                ready to chat.
              </p>
            ) : (
              <button
                onClick={() => void pickModel()}
                className="w-full rounded-xl border border-brand-border py-2.5 text-sm hover:bg-brand-hover"
              >
                Choose a .gguf file
              </button>
            )}
            <button
              onClick={() => void finish()}
              className="w-full rounded-xl bg-accent-primary py-2.5 text-sm font-medium text-white hover:bg-accent-primary/90"
            >
              {addedModel ? "Start chatting" : "Skip for now"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
