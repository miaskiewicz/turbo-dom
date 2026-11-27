# CSS payload deduplication review (source-only proposal)

No runtime/default switch, publication, registry access or Rust/WASM compilation was performed. The private follow-up implements build-time shared assembly; TypeScript checks use cached TypeScript5.9.3. Review base: `329347f2e514cb488c8a7ef5b22c5f0246e1faa7`. This is a measured host prototype, not a validated ten-native-artifact release.

## Measured prototype

Co-locate the existing generated Node loader as `css-wasm/turbo_dom_css.node.cjs`, existing generated web loader as `css-wasm/turbo_dom_css.web.mjs`, and ONE `css-wasm/turbo_dom_css_bg.wasm`. The initial prototype retained exact loader bytes. The validated follow-up changes precisely one leading `@ts-self-types` comment per generated loader, as explicitly authorized; all executable body bytes remain identical. Original/output and runtime-body SHA256 receipts are stored in the generated shared manifest. Both already resolve the same sibling WASM basename; explicit extensions settle CommonJS/ESM interpretation without rewriting engine code. Separate wrapper instances/memories remain separate; sharing a file does not permit cross-backend pointers or class objects.

Actual input CSS native SHA256: `c808daf506c0ac1a380dded2d8a20aa9eac2ab73c9f7ecb735d79a90120d9787` (19,941,376 bytes). Both input WASM files have SHA256 `5c4af4da0bd4c14315fc82d98b6422e2b1355a6a1f035404fcc66a33640fa33b` (15,867,677 bytes each). Generated Node/web loader files are 14,026/16,930 bytes.

| Actual host npm tarball | gzip bytes | unpacked bytes |
| --- | ---: | ---: |
| Original 329 packaging | 23,895,431 | 55,812,021 |
| Shared WASM prototype | 16,863,028 | 39,941,697 |
| Reduction | 7,032,403 | 15,870,324 |

These include the same actual Darwin-arm64 parser/native payloads, parser WASM, source, licenses and package metadata. No other-platform binary was invented. Tarballs were built with `npm pack --ignore-scripts`, not publishing hooks or a compiler. This is packaging size attribution, not a cold-start measurement.

Both concrete generated loaders pass ABI1/version0.8.7 plus real compile/prepare/resolve assertions: var fallback blue, 2em→32px, background longhands, declared text decoration, and dispose/retained-byte behavior. The Node loader runs with no native `.node` imports. The web default initializer resolves the expected shared sibling URL; an in-memory test fetch serves actual bytes with application/wasm. This verifies URL semantics without network/browser equivalence claims.

Evidence and reproducible scripts: `.evidence/dedup-review/{prototype.py,probe.mjs,PACKS.json,node-GREEN.json,web-GREEN.json}` in the isolated review worktree. Before/after tarballs are there too. Copies, not original artifacts, were changed.

## Small production follow-up, after approval by the runtime owner

1. Keep parser `pkg`, `pkg-web`, `index.js`, parser injection, and native/wasm/auto flag semantics unchanged.
2. Assemble shared CSS directory only after Node/web WASM byte equality and generated relative-import checks pass. Preserve immutable source hashes and code/font notices.
3. Keep tiny CommonJS/web forwarding modules at current private loader locations, or update their local paths together. Public `./css-wasm` retains its export name and web initializer API; point it at the `.mjs` and exact web declaration file. No automatic network download or installation is added.
4. Update CSS payload guards, npm files, CI assembly and release receipts together; require exactly one shared WASM. A failed assembly must preserve the preceding complete package.
5. Test actual native-only loading with WebAssembly unavailable, forced WASM without native loads, injected minimal parser behavior, and both backend suites before integrating. The existing native-only proof is unchanged by this prototype; it was not repeated here.

### Explicit integration blockers

Current generated artifacts have no snippets or module imports except Node's built-in fs. Future wasm-bindgen snippet output can contain CJS/ESM variants at the same path. Fail closed on any unreviewed snippet/module collision; do not pick one silently, rewrite generated engine code, or assume equal WASM guarantees compatible glue. Shared assembly should initially reject snippets until separately certified.

Generated Node/web declarations remain exact and separate beside the supported wrapper paths. Shared private Node/web glue comments now reference exact `.node.d.cts`/`.web.d.mts` copies. Assembly requires precisely one expected leading generated comment and verifies executable-body equality. Drift, missing/duplicate comments, arbitrary generated imports, snippets, unequal WASM bytes, unequal low-level declarations and version mismatches fail closed. TypeScript5.9.3 strict NodeNext checks pass for both supported wrappers; a negative assertion confirms the Node namespace cannot be called as a web initializer. Shared `.cjs`/`.mjs` filename inference resolves their matching `.d.cts`/`.d.mts` as well. Both public wrapper paths remain unchanged; no misleading common self-types alias is shipped.

Final implementation repeats the actual Node/web ABI/cascade/disposal probes against the generated forwarding paths. `build:css` and release WASM assembly now invoke the assembler after both raw generated targets; parser build commands remain unchanged. Release check requires shared assembly, exactly one shared WASM and certified wrapper/declaration hashes. Directory replacement rolls back all previously replaced directories on failure. Individual raw CSS WASM build commands produce intermediate packages and must be followed by assembly before distribution.

Private historical `pkg-css*/turbo_dom_css_bg.wasm` URLs cannot remain as physical duplicate files while claiming this saving. They are outside package exports; audit callers before removing them. Web default initialization remains local to the shared glue URL, while user-supplied byte/module injection remains unchanged. Browser bundler/CDN handling needs its own artifact test.

## Optional native packages (design only)

Keep all five existing parser native payloads and parser resolution unchanged. Move ONLY CSS native binaries into optional exact-version packages:

- `@miaskiewicz/turbo-dom-css-linux-x64-gnu`
- `@miaskiewicz/turbo-dom-css-linux-x64-musl`
- `@miaskiewicz/turbo-dom-css-linux-arm64-gnu`
- `@miaskiewicz/turbo-dom-css-darwin-arm64`
- `@miaskiewicz/turbo-dom-css-win32-x64-msvc`

Each contains the corresponding existing binary filename, version0.8.7 metadata, and its required licenses/notices; package names are proposals, not created packages. Root optionalDependencies pin exact0.8.7. Use os/cpu and glibc/musl libc metadata where package managers honor it; older manager behavior must be measured, not assumed to download exactly one Linux flavor. Preserve existing target matrix and GNU glibc2.35 controls.

The loader resolves an already-installed exact local platform package only. Forced native reports genuine absence clearly; existing auto may fall back to bundled WASM on true absence. Init/ABI mismatch and nested dependency errors must propagate instead of becoming false absence. Forced WASM never tries native. No fetch/npm invocation or new public runtime flag.

Release CI builds/checks all existing targets, certifies ABI/version and source identity, then publishes immutable platform packages BEFORE main package. Already-published exact-version bytes must be checked for identity; conflicts stop release. All targets must exist before main package publication. This introduces release ordering and partial-publication recovery work, so it is a separate change from shared-WASM assembly.

The main package still carries one WASM fallback to preserve offline auto behavior. Removing host CSS native from this measured package saves its 19,941,376 unpacked bytes, but users install a selected native sidecar separately; no compressed total or five-platform total is asserted. Current full ten-native-payload guard remains RED because only real host binaries were available. No default switch or package creation is recommended before complete guards and package-manager installation tests pass.

## Validated follow-up receipt

Final source-owned host tarball: **16,864,004 compressed / 39,948,638 unpacked bytes**, versus original **23,895,431 / 55,812,021**. Includes audited type-reference comment changes and generated provenance metadata; differences from the initial prototype are metadata/types, not another WASM. Both actual backend probes and strict cached TypeScript5.9.3 NodeNext checks passed. `python3 -m unittest discover -s scripts -p 'test*css*.py'` passes23 tests, including comment drift, snippet/import refusal, unequal WASM/declarations, artifact changes, duplicate payload rejection and multi-directory replacement rollback. Full platform release matrix remains unmeasured/RED.
