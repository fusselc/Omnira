import { ipc } from "./ipc";

/** Persist a thread's model only after that exact activation succeeds. */
export async function activateConversationModel(
  modelId: string,
  conversationId: string | null,
  api: Pick<typeof ipc, "startRuntime" | "setConversationModel"> = ipc,
): Promise<boolean> {
  const status = await api.startRuntime(modelId);
  if (status.state !== "ready" || status.model_id !== modelId || !conversationId) {
    return false;
  }
  await api.setConversationModel(conversationId, modelId);
  return true;
}
