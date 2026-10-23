/**
 * Pure presentation decisions for the Chat screen, kept out of the component
 * so they can be unit tested without mounting React.
 */

export interface ComposerLockInput {
  /** `sendBlocker(...) === null` for the latest runtime snapshot. */
  canSend: boolean;
  generating: boolean;
  /** A send is between the click and the stream starting. */
  sending: boolean;
  /** The user picked a model and its engine is being started. */
  startingModel: boolean;
}

/** Whether the composer textarea must refuse input right now. */
export function composerDisabled({
  canSend,
  generating,
  sending,
  startingModel,
}: ComposerLockInput): boolean {
  return !canSend || generating || sending || startingModel;
}

/** Whether the Send button may submit `draft`. */
export function canSubmitDraft(input: ComposerLockInput, draft: string): boolean {
  return !composerDisabled(input) && draft.trim().length > 0;
}
