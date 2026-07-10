import { useCallback, useEffect, useRef, useState } from "react";
import { Plus, Send, Square, Trash2, Boxes, Pencil } from "lucide-react";
import {
  ipc,
  toAppError,
  type AppError,
  type Conversation,
  type Message,
  type ModelEntry,
  type RuntimeStatus,
} from "../lib/ipc";
import { startStream, truncateToBudget, type StreamHandle } from "../lib/chat";
import { Markdown } from "../lib/markdown";
import { StatusPill } from "../components/StatusPill";
import { ErrorBanner } from "../components/ErrorBanner";
import { formatWhen } from "../lib/format";

export function Chat({
  runtime,
  refreshRuntime,
  onGoToModels,
  onGoToDiagnostics,
}: {
  runtime: RuntimeStatus;
  refreshRuntime: () => Promise<void>;
  onGoToModels: () => void;
  onGoToDiagnostics: () => void;
}) {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [draft, setDraft] = useState("");
  const [streamingText, setStreamingText] = useState<string | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [truncatedNotice, setTruncatedNotice] = useState(false);
  const streamRef = useRef<StreamHandle | null>(null);
  const streamBufferRef = useRef("");
  const scrollRef = useRef<HTMLDivElement>(null);

  // Renaming & Fallback states
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [dismissedFallback, setDismissedFallback] = useState(false);

  const generating = streamingText !== null;
  const activeConvo = conversations.find((c) => c.id === activeId);
  /** Model tied to the active conversation (persisted), not the live runtime. */
  const conversationModelId = activeConvo?.model_id ?? null;
  /** Dropdown reflects the conversation's model when one is selected. */
  const dropdownModelId = conversationModelId ?? runtime.model_id;
  const conversationModel = models.find((m) => m.id === conversationModelId);
  const runtimeModel = models.find((m) => m.id === runtime.model_id);
  /** Registry entry removed; conversation still references the old id. */
  const conversationModelUnregistered =
    conversationModelId != null && conversationModel === undefined;
  /** Registry entry exists but the on-disk .gguf is gone. */
  const conversationModelFileMissing = conversationModel?.status === "missing";
  const conversationModelInvalid = conversationModel?.status === "invalid";
  const needsRuntimeReload =
    conversationModelId != null &&
    conversationModel?.status === "ok" &&
    conversationModelId !== runtime.model_id &&
    !generating;
  const canSend =
    runtime.state === "ready" &&
    !needsRuntimeReload &&
    !conversationModelUnregistered &&
    !conversationModelFileMissing &&
    !conversationModelInvalid &&
    runtime.model_id === dropdownModelId;

  const loadConversations = useCallback(async () => {
    setConversations(await ipc.listConversations());
  }, []);

  const loadModels = useCallback(async () => {
    setModels(await ipc.listModels());
  }, []);

  useEffect(() => {
    void loadConversations();
    void loadModels();
  }, [loadConversations, loadModels]);

  useEffect(() => {
    if (!activeId) {
      setMessages([]);
      return;
    }
    void ipc.listMessages(activeId).then(setMessages);
  }, [activeId]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [messages, streamingText]);

  useEffect(() => {
    setDismissedFallback(false);
  }, [runtime.model_id]);

  const selectModel = async (modelId: string) => {
    setError(null);
    try {
      await ipc.startRuntime(modelId);
      if (activeId) {
        await ipc.setConversationModel(activeId, modelId);
        await loadConversations();
      }
    } catch (e) {
      setError(toAppError(e));
    } finally {
      await refreshRuntime();
    }
  };

  const loadConversationModel = async () => {
    if (!conversationModelId) return;
    await selectModel(conversationModelId);
  };

  const newConversation = async () => {
    // Defensive check: prevent programmatically creating empty conversations if not ready
    if (runtime.state !== "ready") return;
    const convo = await ipc.createConversation("New conversation", runtime.model_id);
    await loadConversations();
    setActiveId(convo.id);
  };

  const removeConversation = async (id: string) => {
    if (!window.confirm("Are you sure you want to delete this conversation? This cannot be undone.")) return;
    await ipc.deleteConversation(id);
    if (activeId === id) setActiveId(null);
    await loadConversations();
  };

  const startRename = (convo: Conversation, e: React.MouseEvent | React.KeyboardEvent) => {
    e.stopPropagation();
    if (generating) return; // cannot conflict with active streaming
    setEditingId(convo.id);
    setEditTitle(convo.title);
  };

  const cancelRename = () => {
    setEditingId(null);
    setEditTitle("");
  };

  const saveRename = async (id: string) => {
    const trimmed = editTitle.trim();
    if (!trimmed) {
      // Revert/cancel empty titles
      cancelRename();
      return;
    }
    const original = conversations.find((c) => c.id === id)?.title;
    if (trimmed === original) {
      cancelRename();
      return;
    }
    try {
      await ipc.renameConversation(id, trimmed);
      await loadConversations();
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setEditingId(null);
      setEditTitle("");
    }
  };

  const stopGeneration = () => {
    streamRef.current?.cancel();
  };

  const send = async () => {
    const content = draft.trim();
    if (!content || generating || !canSend) return;
    setError(null);
    setDraft("");

    // Ensure a conversation exists.
    let convoId = activeId;
    if (!convoId) {
      const title = content.length > 48 ? `${content.slice(0, 48)}...` : content;
      const convo = await ipc.createConversation(title, runtime.model_id);
      convoId = convo.id;
      setActiveId(convoId);
      await loadConversations();
    } else if (messages.length === 0) {
      const title = content.length > 48 ? `${content.slice(0, 48)}...` : content;
      await ipc.renameConversation(convoId, title);
      await loadConversations();
    }

    if (dropdownModelId && activeConvo?.model_id !== dropdownModelId) {
      await ipc.setConversationModel(convoId, dropdownModelId);
      await loadConversations();
    }

    // Persist the user message BEFORE streaming (stream-boundary contract).
    const userMsg = await ipc.addMessage(convoId, "user", content, "complete");
    const history = [...messages, userMsg];
    setMessages(history);

    // Truncate against the Rust-provided character budget.
    let budget = 24000;
    try {
      budget = (await ipc.chatEndpoint()).context_chars_budget;
    } catch {
      // endpoint errors surface below through the stream path
    }
    const { messages: wire, truncated } = truncateToBudget(history, budget);
    setTruncatedNotice(truncated);

    streamBufferRef.current = "";
    setStreamingText("");

    const persistAssistant = async (
      text: string,
      status: "complete" | "interrupted",
    ) => {
      if (text.length === 0 && status === "interrupted") {
        setStreamingText(null);
        return;
      }
      const saved = await ipc.addMessage(convoId, "assistant", text, status);
      setMessages((prev) => [...prev, saved]);
      setStreamingText(null);
      await loadConversations();
    };

    streamRef.current = startStream(wire, {
      onChunk: (delta) => {
        streamBufferRef.current += delta;
        setStreamingText(streamBufferRef.current);
      },
      onDone: (reason) => {
        void persistAssistant(
          streamBufferRef.current,
          reason === "cancelled" ? "interrupted" : "complete",
        );
      },
      onError: (err) => {
        // Persist whatever partial content arrived, then surface the error.
        void persistAssistant(streamBufferRef.current, "interrupted");
        if (err.code !== "GenerationCancelled") setError(err);
      },
    });
  };

  const readyModels = models.filter((m) => m.status === "ok");

  return (
    <div className="flex h-full">
      {/* Conversation list */}
      <aside className="flex w-64 shrink-0 flex-col border-r border-brand-border bg-brand-card/30">
        <div className="flex items-center justify-between px-4 py-3">
          <h2 className="text-sm font-medium text-brand-textMuted">Conversations</h2>
          <button
            onClick={() => void newConversation()}
            disabled={runtime.state !== "ready"}
            className="rounded-lg p-1.5 text-brand-textMuted hover:bg-brand-hover hover:text-zinc-100 disabled:opacity-30 disabled:hover:bg-transparent disabled:hover:text-brand-textMuted transition-all"
            title={runtime.state === "ready" ? "New conversation" : "Please select and start a model first"}
            aria-label={runtime.state === "ready" ? "New conversation" : "New conversation disabled (select and start a model first)"}
          >
            <Plus size={16} />
          </button>
        </div>
        <div className="flex-1 overflow-y-auto px-2 pb-2">
          {conversations.length === 0 && (
            <p className="px-3 py-6 text-center text-xs text-zinc-600">
              No conversations yet. Your chats stay on this computer.
            </p>
          )}
          {conversations.map((c) => (
            <div
              key={c.id}
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === "F2" && !generating) {
                  startRename(c, e);
                }
              }}
              className={`group flex items-center gap-2 rounded-lg px-3 py-2 text-sm cursor-pointer outline-none focus-visible:ring-1 focus-visible:ring-accent-primary/50 ${
                c.id === activeId
                  ? "bg-brand-hover text-zinc-100 font-medium"
                  : "text-brand-textMuted hover:bg-brand-hover/60"
              }`}
              onClick={() => setActiveId(c.id)}
            >
              {editingId === c.id ? (
                <input
                  type="text"
                  value={editTitle}
                  onChange={(e) => setEditTitle(e.target.value)}
                  onBlur={() => void saveRename(c.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      void saveRename(c.id);
                    } else if (e.key === "Escape") {
                      e.preventDefault();
                      cancelRename();
                    }
                  }}
                  className="flex-1 rounded border border-accent-primary/60 bg-brand-card px-1 py-0.5 text-sm text-zinc-100 outline-none"
                  autoFocus
                  onFocus={(e) => e.target.select()}
                  onClick={(e) => e.stopPropagation()}
                />
              ) : (
                <>
                  <span
                    className="flex-1 truncate"
                    onDoubleClick={(e) => startRename(c, e)}
                  >
                    {c.title}
                  </span>
                  <span className="text-[10px] text-zinc-600 group-hover:hidden">
                    {formatWhen(c.updated_at)}
                  </span>
                  <div className="hidden shrink-0 items-center gap-1 group-hover:flex">
                    <button
                      onClick={(e) => startRename(c, e)}
                      disabled={generating}
                      className="rounded p-0.5 text-zinc-500 hover:text-zinc-200 disabled:opacity-30"
                      title="Rename conversation"
                      aria-label="Rename conversation"
                    >
                      <Pencil size={12} />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        void removeConversation(c.id);
                      }}
                      disabled={generating}
                      className="rounded p-0.5 text-zinc-500 hover:text-accent-danger disabled:opacity-30"
                      title="Delete conversation"
                      aria-label="Delete conversation"
                    >
                      <Trash2 size={12} />
                    </button>
                  </div>
                </>
              )}
            </div>
          ))}
        </div>
      </aside>

      {/* Thread */}
      <section className="flex min-w-0 flex-1 flex-col">
        <header className="flex items-center justify-between gap-3 border-b border-brand-border px-5 py-3">
          <div className="flex min-w-0 items-center gap-3">
            <h1 className="truncate text-sm font-medium">
              {activeConvo?.title ?? "Chat"}
            </h1>
          </div>
          <div className="flex items-center gap-3">
            <select
              value={dropdownModelId ?? ""}
              onChange={(e) => e.target.value && void selectModel(e.target.value)}
              disabled={runtime.state === "starting" || generating}
              className="max-w-56 rounded-lg border border-brand-border bg-brand-card px-3 py-1.5 text-xs text-zinc-100 outline-none focus:border-accent-primary/50"
            >
              <option value="" disabled>
                {readyModels.length ? "Choose a model" : "No models added"}
              </option>
              {conversationModelUnregistered && conversationModelId && (
                <option value={conversationModelId} disabled>
                  Model no longer available
                </option>
              )}
              {conversationModel &&
                conversationModel.status !== "ok" &&
                !readyModels.some((m) => m.id === conversationModel.id) && (
                  <option value={conversationModel.id} disabled>
                    {conversationModel.name} (unavailable)
                  </option>
                )}
              {readyModels.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
            <StatusPill status={runtime} />
          </div>
        </header>

        {conversationModelUnregistered && (
          <div className="mx-5 mt-3 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-accent-danger/30 bg-accent-danger/10 px-4 py-3 text-sm">
            <p className="text-brand-textMuted">
              This conversation used a model that&apos;s no longer available in
              Omnira. Re-add the model on the Models screen, or delete this
              conversation and start a new one.
            </p>
            <button
              onClick={onGoToModels}
              className="shrink-0 rounded-lg border border-brand-border px-3 py-1.5 text-xs font-medium hover:bg-brand-hover"
            >
              Go to Models
            </button>
          </div>
        )}

        {conversationModelFileMissing && (
          <div className="mx-5 mt-3 rounded-lg border border-accent-danger/30 bg-accent-danger/10 px-4 py-3 text-sm">
            <p className="text-brand-textMuted">
              This conversation&apos;s model file is missing:{" "}
              <span className="font-medium text-zinc-100">
                {conversationModel?.name ?? "Unknown model"}
              </span>
              . Restore the file on disk or remove the entry from Models.
            </p>
          </div>
        )}

        {conversationModelInvalid && (
          <div className="mx-5 mt-3 rounded-lg border border-accent-danger/30 bg-accent-danger/10 px-4 py-3 text-sm">
            <p className="text-brand-textMuted">
              This conversation&apos;s model file is invalid or corrupt. Choose
              a different model or re-add a valid .gguf file on the Models
              screen.
            </p>
          </div>
        )}

        {needsRuntimeReload && (
          <div className="mx-5 mt-3 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-accent-warning/30 bg-accent-warning/10 px-4 py-3 text-sm">
            <p className="text-brand-textMuted">
              This conversation uses{" "}
              <span className="font-medium text-zinc-100">
                {conversationModel?.name ?? "another model"}
              </span>
              {runtimeModel && runtime.state === "ready" ? (
                <>
                  , but the chat engine is running{" "}
                  <span className="font-medium text-zinc-100">{runtimeModel.name}</span>.
                </>
              ) : (
                <> — load it to continue chatting.</>
              )}
            </p>
            <button
              onClick={() => void loadConversationModel()}
              disabled={runtime.state === "starting" || conversationModel?.status !== "ok"}
              className="shrink-0 rounded-lg bg-accent-primary px-3 py-1.5 text-xs font-medium text-white hover:bg-accent-primary/90 disabled:opacity-40"
            >
              Load {conversationModel?.name ?? "model"}
            </button>
          </div>
        )}

        {error && (
          <div className="px-5 pt-3">
            <ErrorBanner error={error} onDismiss={() => setError(null)} />
          </div>
        )}

        {runtime.state === "ready" && runtime.fallback_reason && !dismissedFallback && (
          <div className="px-5 pt-3">
            <div className="flex items-center justify-between rounded-xl border border-accent-warning/30 bg-accent-warning/5 px-4 py-3 text-sm text-zinc-200">
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-accent-warning font-semibold">⚡ Performance Note:</span>
                <span>Running in CPU mode. Responses may be slower because GPU acceleration was unavailable.</span>
                <button
                  onClick={onGoToDiagnostics}
                  className="ml-1 text-xs font-semibold text-accent-primary hover:underline"
                >
                  View details
                </button>
              </div>
              <button
                onClick={() => setDismissedFallback(true)}
                className="ml-4 text-xs text-brand-textMuted hover:text-zinc-100"
                title="Dismiss performance notice"
                aria-label="Dismiss performance notice"
              >
                Dismiss
              </button>
            </div>
          </div>
        )}

        <div ref={scrollRef} className="flex-1 overflow-y-auto px-5 py-4">
          {/* Empty states are first-class (docs/design-principles.md) */}
          {runtime.state === "stopped" && messages.length === 0 && !generating ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
              <div className="flex h-14 w-14 items-center justify-center rounded-2xl bg-brand-card text-accent-primary">
                <Boxes size={26} />
              </div>
              <h2 className="text-lg font-medium">Omnira needs a local GGUF model before chat can begin</h2>
              <p className="max-w-sm text-sm text-brand-textMuted">
                All inference runs completely offline and privately on your computer.
              </p>
              <button
                onClick={onGoToModels}
                className="mt-2 rounded-lg bg-accent-primary px-4 py-2 text-sm font-medium text-white hover:bg-accent-primary/90"
              >
                Choose a model
              </button>
            </div>
          ) : runtime.state === "starting" && messages.length === 0 && !generating ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
              <div className="flex h-14 w-14 items-center justify-center rounded-2xl bg-brand-card text-accent-warning animate-pulse">
                <Boxes size={26} />
              </div>
              <h2 className="text-lg font-medium">Starting local model...</h2>
              <p className="max-w-sm text-sm text-brand-textMuted">
                This can take up to a minute depending on your computer's performance and the model's size.
              </p>
            </div>
          ) : messages.length === 0 && !generating ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
              <h2 className="text-lg font-medium">Ready when you are</h2>
              <p className="max-w-sm text-sm text-brand-textMuted">
                Type a message below. Responses are generated locally on this
                computer.
              </p>
            </div>
          ) : (
            <div className="mx-auto flex max-w-3xl flex-col gap-4">
              {truncatedNotice && (
                <p className="text-center text-[11px] text-zinc-600">
                  Earlier messages are not included -- this conversation is
                  longer than the model can read at once.
                </p>
              )}
              {messages.map((m) => (
                <MessageBubble key={m.id} message={m} />
              ))}
              {streamingText !== null && (
                <div className="max-w-[85%] self-start rounded-2xl rounded-bl-sm bg-brand-card px-4 py-3">
                  {streamingText === "" ? (
                    <span className="text-sm text-brand-textMuted animate-pulse">
                      Thinking...
                    </span>
                  ) : (
                    <Markdown text={streamingText} />
                  )}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Composer */}
        <footer className="border-t border-brand-border px-5 py-4">
          <div className="mx-auto flex max-w-3xl items-end gap-2">
            <textarea
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  void send();
                }
              }}
              rows={Math.min(6, Math.max(1, draft.split("\n").length))}
              placeholder={
                conversationModelUnregistered
                  ? "This conversation's model is no longer available..."
                  : conversationModelFileMissing
                    ? "Restore the model file to continue..."
                    : needsRuntimeReload
                      ? "Load this conversation's model to continue..."
                      : runtime.state === "ready"
                        ? "Message your local model..."
                        : runtime.state === "starting"
                          ? "Starting model..."
                          : "Choose a model to start"
              }
              disabled={!canSend || generating}
              className="flex-1 resize-none rounded-xl border border-brand-border bg-brand-card px-4 py-3 text-sm outline-none placeholder:text-zinc-600 focus:border-accent-primary/50 disabled:opacity-60"
            />
            {generating ? (
              <button
                onClick={stopGeneration}
                className="flex h-11 items-center gap-2 rounded-xl bg-accent-danger px-4 font-medium text-white hover:bg-accent-danger/90 transition-colors"
                title="Stop generating"
                aria-label="Stop generating"
              >
                <Square size={14} fill="currentColor" />
                <span className="text-sm">Stop</span>
              </button>
            ) : (
              <button
                onClick={() => void send()}
                disabled={!canSend || !draft.trim()}
                className="flex h-11 w-11 items-center justify-center rounded-xl bg-accent-primary text-white hover:bg-accent-primary/90 disabled:opacity-40"
                title="Send"
              >
                <Send size={16} />
              </button>
            )}
          </div>
        </footer>
      </section>
    </div>
  );
}

function MessageBubble({ message }: { message: Message }) {
  if (message.role === "user") {
    return (
      <div className="max-w-[85%] self-end whitespace-pre-wrap rounded-2xl rounded-br-sm bg-accent-primary/20 px-4 py-3 text-sm">
        {message.content}
      </div>
    );
  }
  return (
    <div className="max-w-[85%] self-start rounded-2xl rounded-bl-sm bg-brand-card px-4 py-3">
      <Markdown text={message.content} />
      {message.status === "interrupted" && (
        <p className="mt-1 text-[11px] italic text-zinc-600">
          Generation stopped -- partial response kept.
        </p>
      )}
    </div>
  );
}
