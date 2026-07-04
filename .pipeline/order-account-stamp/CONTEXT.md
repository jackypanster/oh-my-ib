# CONTEXT — order-account-stamp

Delta on `.pipeline/close-pending-guard/CONTEXT.md`. New/changed terms:

- **Resolved account** — the single account authority: `cfg.account`/`--account` if set,
  else the first managed account (`resolve_account`, mod.rs). Reads have always used it;
  after this feature every placed order carries it in `Order.account`.
- **Choke-point stamping** — the stamp lives inside `place_with_client` behind a REQUIRED
  parameter; no placement path (current or future) can skip it (ADR 0024 §1).
- **Overwrite semantics** — the resolved account always replaces any pre-set
  `Order.account`; there is no second authority (ADR 0024 §2).
- **Explicit-account assumption** — Tiger must accept `Order.account` set explicitly;
  same-day paper probe (ack only); rejection ⇒ journaled observation + operator-decided
  fallback (stamp only when configured), never auto-applied (ADR 0024 §5).

## Conventions (feature-specific)

- The pure builders still emit `account: ""` — frozen suites unchanged; only the gateway
  path mutates the clone it sends.
- `cancel` stays account-agnostic (order-id domain).
- CLAUDE.md untouched; AGENTS.md gains one phrase.
