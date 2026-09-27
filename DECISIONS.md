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

## 2026-09-27 — The password generator draws its randomness in lokey-core

**Decided:** the generator dialog asks the Rust side for each password
(`generator.rs`, on the operating system's random number generator through
`getrandom`, already a dependency). The page shows the result and can copy it
through the same clipboard code as a saved value, so it stays out of Win+V
history and is cleared after 30 seconds. "Use as new key" fills the existing
add row; nothing new saves a value.

**Rejected:** generating in the page with `crypto.getRandomValues`. It is
equally random and could be tested locally without a Rust toolchain, but
`lokey-core` is the one place randomness and secrets are made, and a second
source would be one more thing to review.

**Also rejected:** keeping the password on the Rust side and never sending it
to the page. The dialog exists to show the password before it is used, and a
revealed saved value already reaches the page the same way.
