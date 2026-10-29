# Local foundation runtime provenance

The native Tauri `activate_eltord_invoke` entry point calls
`backend/src/eltor.rs::activate_eltord_process` and launches the bundled `eltord`
as a separate OS process. Its financial coordinator and Tor come from that actual
child executable. The managed backend has a different entry point which runs
`eltor::run_with_args` inside an abortable Tokio task. Check the entry point and
the running executable rather than inferring it from a library dependency.

For a local foundation acceptance run, rebuild the desktop after changes to
Eltord or the vendored Tor source. Record its actual executable digest, dependency
lockfile, local Cargo source overrides and source revisions. The bundled
`backend/bin/eltord` resource should still be rebuilt and kept consistent for
packaging and other entry points, but its digest alone cannot establish which
coordinator the native window is running. The standalone controller binaries
have their own executable provenance as well.

Deactivate the client before replacing runtime components. Retain the accepted
plan, private ledger, unresolved operation claims and previous test results.
A source checkout, successful bootstrap notification, process-running indicator
or successful request on an ordinary free circuit does not prove that the
approved paid path activated. Live foundation acceptance checks the exact native
stream binding against the accepted scope and authenticated all-hop progress.

Wallet lifecycle is independent of foundation settlement evidence. Starting a
configured Phoenixd and displaying its balance/history verifies the wallet UI;
it does not establish a paid-network transaction. The isolated foundation test
account uses a zero-price fixture which refuses payment APIs.

## Client status and cancellation

The local status response distinguishes the Tor process from the configured
foundation path. Its optional `foundation_client_status` contains only a phase:
starting, recovering, last-verified active, closed or stopped. The Connect screen
shows that phase separately. Bootstrap alone shows transport readiness while a
configured paid path is still being verified. Existing non-foundation clients
retain their ordinary connection display.

The managed backend's library status getter performs no control, wallet or ledger
queries. Last-verified active requires authenticated ACTIVE observations from
every pinned hop and expires at the earliest original monotonic round/enforcement
deadline. Deferred work, accepted payment and a running process cannot extend
that lease. An expired observation becomes recovering; closure requires durable
authoritative state. The frontend uses its existing five-second status poll and
labels the result as a last observation, not a guarantee of future delivery.
No circuit, scope, payment, wallet or relay identifiers are returned.

For the native sidecar, the parent creates one private launch directory and fresh
generation, passes these only in that child's environment, and retains the actual
`Child` handle. The existing UI poll checks that exact live child and matching PID
bookkeeping before reading its observation; PID files alone never adopt an
existing daemon. A desktop restart without an owned child shows recovering until
an explicitly managed launch exists. Deactivation/relaunch invalidates the old
observation without changing financial history or authorizing a replacement.

The bridge uses a 0700 directory, 0600 no-follow regular files, strict UID,
generation/PID, framing and 512-byte limits. Atomic replacement uses one fixed
temporary name, so repeated observations do not accumulate files. The active
deadline is converted conservatively from the original authenticated expiry to
Linux `CLOCK_MONOTONIC`; cached publication cannot reset it. Expired, malformed,
unknown-owner or unavailable observations show recovering. Other platforms remain
fail-closed until their shared monotonic clock and private-file profile is
verified. Write/cleanup failures affect observation only. Cleanup does not recurse
or delete an unvalidated directory, unrelated artifact or preexisting temporary.
A failed update may retain the last valid observation only until its already-saved
deadline; it never refreshes that deadline.

Privacy/timing: only an ephemeral local launch generation, process PID, phase and
original deadline cross this private channel. It contains no circuit, payment,
relay, account or path data and changes no network wire record. Writes occur on
observation changes; the parent reaps its child and uses the existing UI poll.
There are no extra control/wallet queries, payment permissions or refreshed
financial timers. This observation is separate from settlement and transport
acceptance; a financial-core run predating the bridge cannot validate its UI.

For the managed in-process entry point, aborting the outer application task cancels its owned controller tasks;
dropping their join handles alone would detach them. A private generation owner
clears the display observation on cancellation and rejects delayed publications
from an older runtime. Cancellation does not delete financial claims, resolve an
unknown wallet outcome, refund a payment or authorize a replacement path.
