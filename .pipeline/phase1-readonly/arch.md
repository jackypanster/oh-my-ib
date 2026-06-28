# arch — phase1-readonly

Architecture for `omi`, the read-only IBKR CLI. Grounds PRD decisions D1–D7 in a concrete
crate shape. Verified against the `ibapi` 3.1 docs (sync client lives at
`ibapi::client::blocking::Client`; `Client::connect("host:port", client_id)`).

## Chosen shape: lib + bin split (testability-driven)

A binary-only crate cannot freeze internal logic (the pipeline freeze gate can only protect a
testable surface — CONTRACT §Freeze coverage). So `oh-my-ib` is a **library crate + a thin `omi`
binary**. Pure, deterministic logic is unit-testable; the only non-freezable part is the live IB
socket call, which needs a running gateway.

```
oh-my-ib/
  Cargo.toml            # package oh-my-ib; [lib] name = oh_my_ib; [[bin]] name = omi, path = src/main.rs
  src/
    lib.rs              # pub mod cli, config, output, error, model, ib
    cli.rs              # clap (derive): global opts + Command enum {health,account,positions,orders,quote,contract,history}
    config.rs           # Config{host,port,client_id,account,md_type}; load(path)+merge(flags); precedence flag>toml>default
    error.rs            # AppError enum + ErrorEnvelope{code,message,context}; exit-code mapping
    output.rs           # Format{Json,Table}; emit_success(Value); emit_error(&AppError); table renderers
    model.rs            # domain structs: AccountSummary, Position, OpenOrder, Quote, ContractInfo, Bar (+ serde)
    ib/
      mod.rs            # pub fn connect(&Config)->Result<Client>; dispatch helpers
      client.rs         # connect/disconnect wrapper over ibapi blocking Client
      account.rs positions.rs orders.rs quote.rs contract.rs history.rs   # each: fetch(client,..) -> model struct
    main.rs             # parse -> load+merge config -> connect -> op -> model -> output::emit -> exit code
  tests/
    cli_contract.rs     # FROZEN (card 01 spec) — black-box subprocess assertions
    data_commands.rs    # FROZEN (card 02 spec) — black-box subprocess assertions
```

## Data flow (linear, no cycles)

```
chat -> Claude Code -> `omi <cmd> [flags]`
  main.rs:
    1. cli::parse(args)                      -> Command + global opts
    2. config::load(~/.config/oh-my-ib/config.toml).merge(opts)   -> Config
    3. ib::connect(&Config)                  -> Client   (127.0.0.1:port, client_id)
    4. ib::<op>(client, args)                -> model::<Struct>     [the only gateway-dependent step]
    5. output::emit_success(struct, format)  -> stdout (JSON default | table)
    on Err(AppError) at any step -> output::emit_error -> stderr {"error":{..}} + process::exit(code)
    on success -> exit 0
```

## Component boundaries (write-sets map to cards)

| component | files | gateway needed? | frozen? |
|---|---|---|---|
| CLI surface | cli.rs | no | YES (black-box --help/parse contract) |
| config | config.rs | no | reviewed-by-reading + impl unit tests |
| error/exit | error.rs | no | YES (connection-error envelope, offline-deterministic) |
| output shaping | output.rs, model.rs | no | reviewed-by-reading + impl unit tests |
| IB socket ops | ib/** | YES | NOT frozen — reviewed-by-reading + manual paper acceptance |

## Error model

`AppError` variants → stable JSON `code` + process exit code:

| variant | code | exit |
|---|---|---|
| Connection (refused/timeout/handshake) | `connection` | 2 |
| NotFound (unknown contract/account) | `not_found` | 3 |
| Data (gateway returned an error/notice) | `data` | 4 |
| Config (bad toml / bad flag combo) | `config` | 5 |
| (catch-all) | `error` | 1 |

Serialized to stderr as `{"error":{"code":<above>,"message":<human>,"context":<where/input>}}`.
No swallowed errors — every `Err` carries location + input + concrete cause (global fast-fail rule).

## Output contract

- `--format json` (default): one JSON value to **stdout** on success. Stable top-level keys per command
  (e.g. account → `{"account":..,"net_liquidation":..,"buying_power":..,..,"currency":..}`).
- Quotes carry `"delayed": true|false` reflecting the effective market-data type.
- `--format table`: human-readable, same data, to stdout.
- Errors: stderr envelope above + non-zero exit. stdout stays empty on error (clean for piping/parsing).

## Config & flags

`Config` fields and precedence **flag > config.toml > default**:
`host`(127.0.0.1) `port`(4002 paper; `--live`→4001) `client_id`(100) `account`(none→first returned)
`md_type`(delayed | live | frozen). Config file: `~/.config/oh-my-ib/config.toml`. Repo is public →
the file lives outside the repo; risk-cap fields are reserved for Phase 2.

## Testability / freeze coverage (read by pipeline-review)

- **Frozen (spec-paths, run offline, deterministic):** the black-box CLI contract + the connection-error
  envelope. The error test targets a deliberately-dead port (`--port 65000`) so it is gateway-independent.
- **NOT frozen (impl-paths):** every `ib/**` live call (needs a gateway), config-precedence and
  output-shaping correctness. impl adds `#[cfg(test)]` white-box unit tests for config/output; review
  reads `ib/**` for correctness; the operator runs manual paper-account acceptance (PRD criteria 1–8).

## ibapi integration notes (confirm exact signatures in impl)

- Sync client: `ibapi = { version = "3.1", default-features = false, features = ["sync"] }`,
  type `ibapi::client::blocking::Client`.
- `Client::connect("127.0.0.1:4002", client_id) -> Result<Client, ibapi::Error>`.
- Ops to bind: `account_summary`, `positions`, `open_orders`/`all_open_orders`, market-data snapshot
  (`market_data(&contract).subscribe()` builder; take first snapshot then drop for a one-shot quote),
  `contract_details`, `historical_data(&contract, BarSize).what_to_show(..).duration(..)`.
- Market-data type set via the client's market-data-type call before a quote request
  (delayed = type 3, frozen = 4, live = 1) — map `--md-type`.
- Map every `ibapi::Error` into `AppError` (connection vs data vs not_found) — never bubble a raw error.
