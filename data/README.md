# data/

Placeholder directory. **Never commit runtime data, conversations, databases,
or logs to this repository.**

At runtime, Omnira stores all user data under `%LOCALAPPDATA%\Omnira\` (see
docs/data-ownership-and-storage.md), never in the repository. This directory
exists only to document that policy (it is gitignored except for this file).
