import { useEffect, useState } from "react";
import { Shield, FolderOpen } from "lucide-react";
import { ipc, toAppError, type AppError, type Settings } from "../lib/ipc";
import { ErrorBanner } from "../components/ErrorBanner";
import { pickGgufFile } from "../lib/dialog";
import { BrandMark } from "./BrandMark";

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
    const selected = await pickGgufFile();
    if (!selected) return;
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
            <BrandMark size="lg" />
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
              <p className="text-xs text-brand-textMuted">Your conversations and settings will be saved here:</p>
              <p className="mt-1 break-all font-mono text-xs">{dataDir}</p>
            </div>
            <div className="flex gap-3">
              <button
                onClick={() => setStep(0)}
                className="w-1/3 rounded-xl border border-brand-border py-2.5 text-sm font-medium hover:bg-brand-hover"
              >
                Back
              </button>
              <button
                onClick={() => setStep(2)}
                className="w-2/3 rounded-xl bg-accent-primary py-2.5 text-sm font-medium text-white hover:bg-accent-primary/90"
              >
                Continue
              </button>
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="flex flex-col gap-4">
            <div className="flex items-center gap-3">
              <FolderOpen size={20} className="text-accent-primary" />
              <h1 className="text-lg font-semibold">Add a model</h1>
            </div>
            <div className="space-y-2">
              <p className="text-sm text-brand-textMuted">
                Omnira runs local AI models in the <span className="font-semibold text-zinc-300">.gguf</span> format. 
                If you don't have one, you can download models like Llama 3 or Mistral from Hugging Face.
              </p>
              <a 
                href="https://huggingface.co/models?search=gguf" 
                target="_blank" 
                rel="noreferrer"
                className="inline-block text-xs text-accent-primary hover:underline"
              >
                Browse GGUF models on Hugging Face &rarr;
              </a>
              <p className="text-xs text-zinc-500">
                Tip: A 4GB to 8GB file (e.g. 7B to 8B parameter models) is a good starting point for most computers.
              </p>
            </div>
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
                Choose a .gguf file...
              </button>
            )}
            <div className="flex gap-3">
              <button
                onClick={() => setStep(1)}
                className="w-1/3 rounded-xl border border-brand-border py-2.5 text-sm font-medium hover:bg-brand-hover"
              >
                Back
              </button>
              <button
                onClick={() => void finish()}
                className="w-2/3 rounded-xl bg-accent-primary py-2.5 text-sm font-medium text-white hover:bg-accent-primary/90"
              >
                {addedModel ? "Start chatting" : "Skip for now"}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
