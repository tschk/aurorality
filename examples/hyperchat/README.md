# HyperChat

HyperChat is the Aurorality service-routing example. The visible app surface is authored in `views/main.crepus` and bundled as `views/main.json`; Swift owns the window, plugin bridge, and local state.

## Run

```bash
cargo build -p aurorality-core --features js
cd examples/hyperchat
swift build
brisk run
```

`brisk run` uses the bundled JSON IR so the app starts without runtime UniFFI calls from Brisk's direct `swiftc` backend.

## Services

Matrix uses the Matrix Client-Server API:

```bash
export MATRIX_HOMESERVER=https://matrix.example
export MATRIX_USER_ID=@user:matrix.example
export MATRIX_ACCESS_TOKEN=...
export MATRIX_ROOM_ID='!room:matrix.example'
```

Stalwart targets a local Stalwart/JMAP service, typically the sibling `../stalwart-lite` checkout:

```bash
export STALWART_BASE_URL=http://localhost:8080
export STALWART_USERNAME=...
export STALWART_PASSWORD=...
```

If a service is not configured, HyperChat reports it as unavailable instead of simulating success.

## Files

- `views/main.crepus` is the UI.
- `views/main.json` is the compiled IR loaded by the app.
- `Sources/App.swift` loads the bundled IR.
- `Sources/ChatStorePlugin.swift` stores local message state.
- `scripts/backend.js` scores route decisions.
