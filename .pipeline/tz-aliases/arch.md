# arch — tz-aliases (Phase 1.1)

Small, additive change layered on the shipped Phase 1 crate. No change to the read-only contract or any
command behavior — only the connect path gains a one-time alias registration.

## Component
New lib module **`src/tz.rs`**:
- `pub fn builtin_aliases() -> &'static [(&'static str, &'static str)]` — the curated table of
  **unambiguous** gateway-abbreviation → IANA mappings:
  `("HKT","Asia/Hong_Kong"), ("JST","Asia/Tokyo"), ("KST","Asia/Seoul"), ("SGT","Asia/Singapore")`.
- `pub fn register_builtin_aliases()` — iterates the table calling `ibapi::register_timezone_alias`,
  guarded by `std::sync::Once` so it runs at most once per process.

`pub mod tz;` added to `src/lib.rs`.

## Integration
`src/ib/mod.rs::connect` calls `crate::tz::register_builtin_aliases()` as its first line, before
`Client::connect`. (register must precede connect — confirmed by the crate docs.) Idempotent via `Once`.

## ibapi API (confirmed against vendored 3.1 source)
`ibapi::register_timezone_alias(name: impl Into<String>, iana: impl Into<String>)` — crate-root fn,
returns `()` (no Result), writes a global alias map. Safe to call repeatedly; `Once` avoids redundant work.
The `IBAPI_TIMEZONE_ALIASES` env is read natively by ibapi and remains additive/overriding.

## Freeze coverage
- **Frozen** (`tests/tz_aliases.rs`, offline-deterministic): `tz::builtin_aliases()` contains
  `HKT→Asia/Hong_Kong`, table non-empty, no empty entries, keys unique.
- **NOT frozen** (live acceptance): the gateway handshake actually succeeding without the env var —
  needs the live Tiger gateway (`unset IBAPI_TIMEZONE_ALIASES; omi --live health`).

No ADR: the change is small, additive, and reversible (one module + one call site).
