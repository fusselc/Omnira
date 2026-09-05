# Release QA Checklist

Use this checklist to perform final manual QA sweeps before tagging and publishing any public alpha release. Testing must be completed on a standard, target Windows OS environment.

---

## 1. Clean Install
- [ ] Run the generated installer `Omnira_0.1.0_x64-setup.exe` on a machine or VM where Omnira was not previously installed (or after running a full clean uninstall).
- [ ] Verify the installation process completes without UAC errors, warnings, or dependency prompts.
- [ ] Confirm that all application files and runtimes are successfully placed under `C:\Program Files\Omnira`.
- [ ] Confirm that no runtime user files are placed in Program Files (they must belong to `%LOCALAPPDATA%`).

## 2. Offline First Run with a Local GGUF
- [ ] Turn off all network interfaces (disconnect Wi-Fi, unplug Ethernet, or enable Airplane Mode).
- [ ] Launch Omnira for the first time.
- [ ] Complete the welcome and privacy onboarding steps.
- [ ] Verify that a local `.gguf` model can be imported from your local disk.
- [ ] Confirm that the model is referenced in-place without triggering network calls or copy operations.
- [ ] Confirm that onboarding successfully concludes and transitions you to the Chat screen.

## 3. Model Startup and Chat
- [ ] From the main Chat interface (with network interfaces still disabled), click to start your active model.
- [ ] Verify that the visual loading state displaying "Starting local model..." appears.
- [ ] Confirm that the model starts up and transitions to "Ready when you are" or opens the chat inputs.
- [ ] While the model loads, confirm that **no console or Windows Terminal window appears** at any point, and that none is left open once the model is ready (Task Manager shows a single `llama-server.exe` under `omnira.exe`).
- [ ] Confirm the status pill reads "Running locally · llama.cpp" and its tooltip names the mode (CPU or GPU (Vulkan)).
- [ ] With Discord/Steam/RTSS running, load a model — Vulkan must succeed with no ErrorDeviceLost.
- [ ] Send a message (e.g. "Say hello in one short sentence").
- [ ] Verify that token generation streams back in real-time.
- [ ] Test the unmistakable "Stop" generation control: press the Stop button during active generation and verify the stream immediately halts while preserving the partial response.
- [ ] Start a long generation, switch to Models, then back to Chat: the response is still streaming (or finished) -- it must not be lost.
- [ ] Click-drag across an assistant reply and a user message and press Ctrl+C: the text is selectable and copies.
- [ ] Press **Unload model** in the Chat header: `llama-server.exe` exits, the pill shows "No model running", and the composer shows "Choose a model to start". Repeat via the Models screen button.
- [ ] Load a large model and press **Cancel loading** while it is still starting: loading stops, no error is shown, and `llama-server.exe` is gone.
- [ ] With a model loaded and a conversation open, remove that model on the Models screen and return to Chat: the engine is unloaded, a "no longer available" notice appears, and sending is disabled (no "Thinking..." without a model).

## 4. CPU Fallback Notice
- [ ] Force a CPU fallback (e.g. by selecting a model that exceeds GPU memory capacity or running in an environment without Vulkan support).
- [ ] Verify that a performance notice is displayed in the main Chat experience stating:
  *"Running on CPU. Responses may take longer on this computer."*
- [ ] Confirm that the notice is dismissible for the current session.
- [ ] Click "View details" on the notice and verify it redirects you to the Advanced Diagnostics panel, where the fallback detail includes the engine's startup output (the reason Vulkan failed).
- [ ] Confirm that the notice does not show up when Vulkan/GPU acceleration starts successfully.
- [ ] Restart Omnira and load the model again: it starts on CPU (remembered), the notice still appears, and Advanced Diagnostics offers "Try GPU acceleration again". Press it, reload the model, and confirm Vulkan is attempted first.

## 4b. Theme
- [ ] In Settings, switch Theme to Light: the whole app (sidebar, chat, cards, inputs) switches immediately without restart, and text stays readable.
- [ ] Restart Omnira: the light theme is still applied. Switch back to Dark and confirm the same.

## 4c. Branding
- [ ] The window title bar, taskbar, Start menu, and installer show the Omnira ring icon, not the stock Tauri icon.

## 5. Conversation Rename, Model Rename, and Delete Confirmation
- [ ] Create a new conversation or send a first message.
- [ ] Verify that double-clicking the conversation title in the sidebar or clicking the Pencil icon triggers rename mode.
- [ ] Select the conversation and press `F2` to verify it triggers rename mode.
- [ ] Type a new name and press `Enter` (or click away to trigger Blur) to save.
- [ ] Verify that pressing `Escape` correctly cancels the edit without changing the title.
- [ ] Attempt to save a blank/whitespace-only title and verify that it cancels/reverts to the original title rather than saving a blank name.
- [ ] On the Models screen, rename a model via Pencil, double-click, or `F2`; confirm the display name changes and the file path is unchanged.
- [ ] Hover over a conversation item, click the Trash icon, and verify that a warning dialog pops up confirming the deletion.
- [ ] Cancel the deletion and confirm the data is preserved; approve the deletion and verify it is permanently wiped.

## 6. Restart/Persistence
- [ ] Close the application.
- [ ] Launch the application again (offline).
- [ ] Confirm that your settings, onboarding state, previously active conversation, and full message history are loaded and accessible.
- [ ] Verify the conversation that was open before quitting is selected again on launch, and that deleting that conversation (or clearing all conversations) leaves Chat on the empty state after the next launch instead of a stale thread.

## 7. Normal Close and Forced-Close Orphan-Process Check
- [ ] Load a model so `llama-server.exe` is running (verify it via Task Manager).
- [ ] Close Omnira normally. Verify in Task Manager that both `omnira.exe` and `llama-server.exe` terminate completely.
- [ ] Load the model again. While the model is active, forcefully terminate `omnira.exe` (via Task Manager "End Task" or PowerShell `Stop-Process`).
- [ ] Confirm that the child process `llama-server.exe` is immediately reaped and does not leak as an orphan process.

## 8. Uninstall and Reinstall Behavior
- [ ] Run the uninstaller (`C:\Program Files\Omnira\uninstall.exe`).
- [ ] Confirm that the application folder in `Program Files\Omnira` is completely removed.
- [ ] Verify that all user-generated data under `%LOCALAPPDATA%\Omnira\` (including `omnira.db` database and configs) remains **intact** and is not deleted.
- [ ] Verify that all GGUF models on disk are **intact** and were not deleted.
- [ ] Reinstall the application and verify that your historical conversations and settings are immediately recognized.
