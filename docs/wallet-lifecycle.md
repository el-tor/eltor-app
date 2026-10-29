# Wallet startup and refresh

Desktop auto-start requires embedded Phoenix support enabled, an existing default
Phoenix configuration at the local HTTP endpoint on port 9740, and nonempty
credentials in `$HOME/.phoenix/phoenix.conf`. The desktop retains its existing
`true` default for `APP_ELTOR_USE_PHOENIXD_EMBEDDED`; an explicit `false` disables
auto-start. The standalone web backend retains its existing `false` default.
Missing credentials require explicit manual setup. A remote/default CLN or LND
wallet, nondefault Phoenix entry, or remote Phoenix endpoint does not start a
local wallet. Startup never changes the selected wallet or its configuration.
The local free-testnet launcher continues to supply `false`.

Boot, manual Start, status and Stop share one AppState/process owner. Concurrent
Start/Stop operations serialize, exited children are reaped, and readiness requires
an authenticated `/getinfo` response. A config file or a spawned PID alone does
not report startup success. A slow process remains owned and stoppable after the
bounded readiness wait. Wallet shutdown only targets the child owned by the app;
Tor port cleanup does not kill another wallet listening on the Phoenix port.

An app-level refresh hook runs above page routing. It refreshes on startup,
Phoenix/Lightning ready and stopped events, and bounded polling (two seconds while
unavailable, fifteen seconds when ready). Balance Refresh and configuration changes
reload status, balances, offer and transactions together. Manual Start refreshes
that same snapshot. Responses from an older request cannot overwrite newer ready
data. Timers, event subscriptions and obsolete request completions are cleaned up
on unmount, including React StrictMode's development remount.

From `frontend/`, `pnpm test:wallet` runs a mocked startup-to-ready and stale-response
regression with no wallet process or network access. `pnpm run build` produces the
frontend bundle. Backend/Tauri compilation is separate; none of these checks needs
a funded wallet. The configured free fixture must remain disabled during testing.
