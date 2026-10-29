# Local Tor 0.5.0 desktop integration

Use this branch with the sibling `merge-upstream-0.5.0` branches and the
workspace's opt-in Cargo source overrides. The full build and validation
handoff is in `eltord/docs/upstream-0.5.0.md` in the sibling repository.
Unpublished sibling commits require local source overrides; remote release
pins must be updated separately when publication is explicitly authorized.

LNI is pinned to the merged Phoenixd polling fix
`ac6e0eeb5b9103fdc2610c313535baf639145191`. Invoice creation keeps the explicit
BOLT11 choice through its current optional field. Native and HTTP wallet
transaction commands pass no date filters, preserving their existing behavior.
The native application's Cargo lockfile is committed so the current LNI graph,
including already-locked yanked transitive dependencies, remains reproducible.
Do not replace that lockfile with a fresh unconstrained resolution.

Rebuild the sibling Eltord daemon before rebuilding the app. Deploy
`target/debug/eltor` as `backend/bin/eltord` from the workspace root; native
resources are copied into the build output by Tauri. Verify that the resource
actually launched by the app matches the new daemon. Existing wallet binaries
and real wallet configuration must be preserved. Both development profiles
optimize the embedded Tor C code at level 2.

On 2026-10-28 the Linux native build started, its Activate button reached a green
Client indicator and Connected dialog, and its embedded Tor reported
0.5.0.0-alpha-dev at 100% bootstrap. Firefox used SOCKS5 `127.0.0.1:18048` with
remote DNS and rendered the Tor Project HTTPS website. Six repeated HTTPS page
requests and a 337736-byte download passed; the download's SHA-256 matched the
direct reference. The running daemon matched the deployed optimized binary.
The test used the free local nine-node Chutney network; it did not exercise real
Lightning settlement. The standalone test client selects DNS-capable exit
`test004r`, because synthetic Chutney exits can have DNS disabled.

The upgraded fork retains its 2048-byte cells and whole-cell TOR1 encryption.
Upstream's 509-byte CGO mode is disabled until a compatible profile is designed
and independently tested. The source snapshot is a development release, not a
stable Tor release. Proposed paid-onion-service and renewal features remain
outside this implementation change.
