# StreamShougiBoard development guide

## Product boundary

StreamShougiBoard is a local-only application. One executable owns the board state, the loopback
HTTP/WebSocket service, and the two pages served from it.

Do not add a hosted web application, login, database, public listener, or cloud deployment unless a
new product decision explicitly changes this boundary. The listener binds `127.0.0.1` only; keep the
Host and Origin checks on every request.

This is a **board editor**, not a game engine. It does not validate legal moves, detect check, or
enforce turns. Adding move validation would break the main use (setting up arbitrary positions on
stream). The only rules enforced are the ones that make an edit meaningless: the king cannot be
captured or sent to a hand, and a piece cannot move onto a friendly piece.

Every dependency that ends up in the distributed executable must be permissively licensed. The
project moved off a JavaScript runtime specifically because it statically linked LGPL-2.1 code, and
a single-file executable cannot honour that licence's relinking provision comfortably. Do not
reintroduce copyleft into the shipped binary — `scripts/generate-third-party-notices.ts` fails the
build if you try.

## Source layout

- `app/`: the Rust binary — board model, SFEN, the command reducer, the loopback server, config,
  and the Windows tray. Almost all of the application's behaviour lives here, and so do most tests.
- `client/`: two pages — `/` (control) and `/board` (OBS browser source) — sharing one SVG renderer.
- `protocol-fixtures/`: a snapshot written by Rust and read by a client test, so the two sides
  cannot drift apart silently.

`app/src/session.rs` decides what a click means (select / move / drop). It is deliberately on the
server side of the wire so that two control pages cannot disagree, and so the interaction can be
tested without a browser.

State flows one way: the control page sends commands, the server applies them to the canonical
state, and every subscriber receives a full snapshot. There are no incremental events — the whole
state is a couple of KB, so full snapshots remove a whole class of resync bugs.

The client holds no board logic. `client/src/shogi.ts` has the display-side types and a handful of
pure helpers; `client/src/protocol.ts` mirrors `app/src/protocol.rs`.

## Commands

```bash
bun install --frozen-lockfile
cargo install cargo-about --version 0.9.1 --locked --features cli

bun run check          # version match → prepare:assets → type → lint → client tests
bun run watch:run      # client bundle watch + cargo run
bun run generate:licenses

cd app
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo clippy --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings
cargo test --locked
```

`bun run prepare:assets` must run before building the Rust binary in release mode: `rust-embed`
pulls `client/static` into the executable. It first runs `generate:licenses`, because the client
build treats the generated third-party licence page as a required input — a build that silently
dropped it would ship an executable with no licence notice. Debug builds read `client/static` from
disk at runtime, so `watch:run` picks up page edits without a restart.

`app/build.rs` embeds the icon and version resources on Windows. `app/assets/*.ico` is committed;
regenerate it only when the icon should change.

## Tests

- `app`: SFEN round-trips, every board edit, click semantics, Host/Origin checks, hub broadcast,
  config parsing, tray tooltip and clipboard encoding.
- `client`: layout geometry, saved-position storage, and the protocol fixture.

When changing the protocol, update `app/src/protocol.rs`, `client/src/protocol.ts`, and
`docs/protocol.md` together, then regenerate the fixture with `UPDATE_PROTOCOL_FIXTURES=1 cargo test`.

There is no separate protocol version to bump: the pages are served from the same executable, so
the snapshot carries the app version and a page whose build differs reloads itself once. The
fixture writes a placeholder version so releases do not invalidate it.

## Versions

`app/Cargo.toml` holds the version that is embedded in the executable and matched against the
release tag. `package.json` carries the same number for convention; `bun run check:version` fails if
they drift.
