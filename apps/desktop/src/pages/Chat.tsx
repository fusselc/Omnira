import { useCallback, useEffect, useRef, useState } from "react";
import { Plus, Send, Square, Trash2, Boxes } from "lucide-react";
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
}: {
  runtime: RuntimeStatus;
  refreshRuntime: () => Promise<void>;
  onGoToModels: () => void;
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

  const generating = streamingText !== null;
  const selectedModelId = runtime.model_id;

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

  const selectModel = async (modelId: string) => {
    setError(null);
    try {
      await ipc.startRuntime(modelId);
    } catch (e) {
      setError(toAppError(e));
    } finally {
      await refreshRuntime();
    }
  };

  const newConversation = async () => {
    const convo = await ipc.createConversation("New conversation", selectedModelId);
    await loadConversations();
    setActiveId(convo.id);
  };

  const removeConversation = async (id: string) => {
    await ipc.deleteConversation(id);
    if (activeId === id) setActiveId(null);
    await loadConversations();
  };

  const stopGeneration = () => {
    streamRef.current?.cancel();
  };

  const send = async () => {
    const content = draft.trim();
    if (!content || generating || runtime.state !== "ready") return;
    setError(null);
    setDraft("");

    // Ensure a conversation exists.
    let convoId = activeId;
    if (!convoId) {
      const title = content.length > 48 ? `${content.slice(0, 48)}...` : content;
      const convo = await ipc.createConversation(title, selectedModelId);
      convoId = convo.id;
      setActiveId(convoId);
      await loadConversations();
    } else if (messages.length === 0) {
      const title = content.length > 48 ? `${content.slice(0, 48)}...` : content;
      await ipc.renameConversation(convoId, title);
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

  const activeConvo = conversations.find((c) => c.id === activeId);
  const readyModels = models.filter((m) => m.status === "ok");

  return (
    <div className="flex h-full">
      {/* Conversation list */}
      <aside className="flex w-64 shrink-0 flex-col border-r border-brand-border">
        <div className="flex items-center justify-between px-4 py-3">
          <h2 className="text-sm font-medium text-brand-textMuted">Conversations</h2>
          <button
            onClick={() => void newConversation()}
            className="rounded-lg p-1.5 text-brand-textMuted hover:bg-brand-hover hover:text-zinc-100"
            title="New conversation"
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
              className={`group flex items-center gap-2 rounded-lg px-3 py-2 text-sm cursor-pointer ${
                c.id === activeId
                  ? "bg-brand-hover text-zinc-100"
                  : "text-brand-textMuted hover:bg-brand-hover/60"
              }`}
              onClick={() => setActiveId(c.id)}
            >
              <span className="flex-1 truncate">{c.title}</span>
              <span className="hidden text-[10px] text-zinc-600 group-hover:hidden">
                {formatWhen(c.updated_at)}
              </span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  void removeConversation(c.id);
                }}
                className="hidden shrink-0 rounded p-1 text-zinc-600 hover:text-accent-danger group-hover:block"
                title="Delete conversation"
              >
                <Trash2 size={13} />
              </button>
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
              value={selectedModelId ?? ""}
              onChange={(e) => e.target.value && void selectModel(e.target.value)}
              disabled={runtime.state === "starting" || generating}
              className="max-w-56 rounded-lg border border-brand-border bg-brand-card px-3 py-1.5 text-xs text-zinc-100 outline-none focus:border-accent-primary/50"
            >
              <option value="" disabled>
                {readyModels.length ? "Choose a model" : "No models added"}
              </option>
              {readyModels.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
            <StatusPill status={runtime} />
          </div>
        </header>

        {error && (
          <div className="px-5 pt-3">
            <ErrorBanner error={error} onDismiss={() => setError(null)} />
          </div>
        )}

        <div ref={scrollRef} className="flex-1 overflow-y-auto px-5 py-4">
          {/* Empty states are first-class (docs/design-principles.md) */}
          {runtime.state === "stopped" && messages.length === 0 && !generating ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
              <div className="flex h-14 w-14 items-center justify-center rounded-2xl bg-brand-card text-accent-primary">
                <Boxes size={26} />
              </div>
              <h2 className="text-lg font-medium">Choose a model to start chatting</h2>
              <p className="max-w-sm text-sm text-brand-textMuted">
                Omnira runs AI models from your own computer. Pick a model file
                you already have, and everything stays private.
              </p>
              <button
                onClick={onGoToModels}
                className="mt-2 rounded-lg bg-accent-primary px-4 py-2 text-sm font-medium text-white hover:bg-accent-primary/90"
              >
                Go to Models
              </button>
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
                runtime.state === "ready"
                  ? "Message your local model..."
                  : runtime.state === "starting"
                    ? "Starting model..."
                    : "Choose a model to start"
              }
              disabled={runtime.state !== "ready" || generating}
              className="flex-1 resize-none rounded-xl border border-brand-border bg-brand-card px-4 py-3 text-sm outline-none placeholder:text-zinc-600 focus:border-accent-primary/50 disabled:opacity-60"
            />
            {generating ? (
              <button
                onClick={stopGeneration}
                className="flex h-11 w-11 items-center justify-center rounded-xl bg-accent-danger/90 text-white hover:bg-accent-danger"
                title="Stop generating"
              >
                <Square size={16} />
              </button>
            ) : (
              <button
                onClick={() => void send()}
                disabled={runtime.state !== "ready" || !draft.trim()}
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
