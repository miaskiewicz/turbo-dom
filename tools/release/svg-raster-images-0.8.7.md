# SVG embedded raster and MIME decoding, v0.8.7

Base: `a8832a861c1acc58a5eb3bfbab9a42b74ac0474e`. Two independent fixes: activate only pinned resvg0.45.1 `raster-images`, then reuse the shared raster signature classifier and upstream resolver for MIME aliases such as `img/png`. System-font/text/memmap features remain disabled. SVG CSS-variable paint is a separate defect and is **not resolved by these two changes**.

## Public regression proof

Synthetic fixtures under `crates/turbo-dom-layout/tests/fixtures/svg-raster` originate in the independently captured Chromium159.0.8154.6 control. `oracle.json` records input hashes, probe/capture hashes, version and expected RGBA(238,34,68,255). Canonical href/xlink/pattern/use-mask and MIME-alias cases must paint all6400 pixels exactly red.

Controlled local feature-off mutation fails the canonical embedded-raster test (exit101). Restoring the feature passes. Independently forwarding original MIME instead of canonical signature resolution fails the alias test (exit101); restoring signature resolution passes. Raw logs and source/capture hashes are in the isolated SVG worktree's `.evidence/{feature-RED.log,mime-RED.log,RED-RESULT.json,BASELINE-HASHES.json}`.

The public `svg::rasterize` actual-asset replay uses the exact workspace dependency versions (no package-version drift). Accounting/banks/fintech/EOR/ISO raw pixels match the independent feature-only controls byte-for-byte; SOC2/GDPR/vector/literal FAQ remain unchanged. Both actual img/png href variants now match canonical6400 red pixels. The real navbar alias matches canonical-MIME pixels byte-for-byte with1,727,891 nontransparent pixels. `.evidence/REPLAY-RESULT.json` contains raw-pixel hashes. Variable FAQ remains independently black; no consumer rebuild or site-success claim is made here.

## Resource and permission boundary

`resolve_string` remains explicitly disabled: existing valid local PNG absolute paths, file URIs and relative paths never load; HTTP/HTTPS stay denied. Existing nested-SVG XML/root/depth guards remain. Malformed RIFF/non-raster XML cannot bypass them. No filesystem or network resolver was introduced.

New checked `parse_checked` / `rasterize_checked` distinguish `SvgResourceError`. Legacy Option APIs retain None on resource failure. Existing raster/screenshot adapters still use those legacy APIs; checked error visibility in consumers requires separate adoption.

Per-axis embedded-raster admission uses existing `MAX_DIM`8192. The cumulative RGBA ceiling derives from the existing maximum canvas, `MAX_DIM * MAX_DIM *4`; it is not a raised or independently chosen budget. Actual GIF first-frame dimensions are checked independently of logical-screen size. Iterative final converted-tree accounting covers repeated use, fill/stroke patterns, masks/clips/filter images and nested SVG/text roots. Upstream resvg decodes on each render occurrence and drops the local decoded raster afterward; accounting does **not** deduplicate encoded bytes. The ceiling bounds conservative decode work, not peak RAM. Pure arithmetic and header-only fixtures exercise limits without allocating oversized pixel buffers.

## Validation

Rust1.98.0, isolated target, one compiler job:

- `cargo test -p turbo-dom-layout --all-features`:1307passed,0failed,0ignored.
- `cargo test -p turbo-dom-layout`:1255passed,0failed,0ignored.
- `cargo test -p turbo-dom-raster`:476passed,0failed,0ignored.
- Scoped strict Clippy for layout+raster, all targets/all features: pass,0warnings.
- Workspace rustfmt, changed-source CC≤9 and whitespace checks: pass.
- New public SVG tests:6passed; intermediate feature-only commit:5passed.

Coverage floors and combined workspace/binding/consumer integration remain the release owner's batch gates. No floors, limits, assertions or execution budgets were weakened.
