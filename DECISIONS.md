# DECISIONS.md — lokey

Append-only. Decisions before 2026-09-19 are on the Notion page named in
`.claude/CLAUDE.md`.

## 2026-09-27 — Recovery: a recovery key or none, not four security tiers

**Decided:** setup offers two choices when the master password is forgotten: a
recovery key (the default), or no recovery. Vault format v3 seals a random data
key under the password's key and, optionally, the recovery key's; changing
either replaces the data key.

**Rejected:** four tiers chosen at setup: password with a hint, password with
security questions, password with a recovery key, password only. A vault is as
strong as its weakest way in. A hint has to be readable without the password,
so it sits in the clear in the file; security-question answers are guessable
and would unlock the vault on their own, making the "moderate" tier weaker than
the password it protects. Most people take the easiest option, so those tiers
would set the strength of most vaults. NIST SP 800-63B also rules out hints
readable before authentication.

**Also rejected:** Windows Hello or DPAPI unlock as recovery. It is bound to one
Windows account on one PC, so it does not survive a new PC, and anyone in that
account gets the vault.

**Maintainer's words:** "decide for yourself. Just make it as safe and secure
as possible."
