# Local foundation runtime provenance

The current native application's backend starts the `eltor` Rust library inside
an abortable Tokio task. See `backend/src/eltor.rs` (`start_eltor_process`) and its call to
`eltor::run_with_args`. Its coordinator and embedded Tor therefore come from the
sources linked into the desktop executable, not from replacing a daemon file
while the desktop keeps running.

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
