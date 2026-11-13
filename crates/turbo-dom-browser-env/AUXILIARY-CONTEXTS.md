# Auxiliary browsing contexts (v0.8.4 candidate)

This is the producer boundary for turbo-dom #504 and turbo-surf #326. The
binding owns browser identities, target selection, opener relations and native
WindowProxy access checks. The host owns Page objects, network admission,
scheduling, document loading, session history and resource lifetime.

The candidate is not a published or independently accepted popup implementation
until its native controls and the paired Surf Page integration pass.

## Actor and identity

Synchronous same-origin `window.open()` requires the opener and initial popup
context to share one V8 isolate/actor. Creating another Node-facing Page through
an asynchronous `liveOpen` call cannot satisfy this contract. A Page facade must
address an auxiliary context inside its existing actor; closing that Page must
not close the shared actor while another Page still lives.

`AuxiliaryContextId` is unique within that actor and survives document navigation.
`OpenRequest.source_id`, `target_id` and `new_context` are assigned before the
host admission callback. They are usable even while an opener is still inside
its initial load and has not entered a process-wide live-Page registry.

`auxiliary_context_identity(context).creation_source` is immutable. It remains
available when implicit noopener, noreferrer, `window.opener = null`, or COOP
separates the public opener relation. Use this trusted initiating identity for
the context `page` and initiating Page `popup` events. Emit those events once per
new admitted Page, not on named reuse. Resource/request/frame attribution must
carry the target context ID; an empty iframe path is not enough to distinguish
an auxiliary root from its opener's main frame.

## Synchronous admission

1. Initialize the root's DOM and origin policy. Install the native
   failed-access callback and call `install_auxiliary_context` before guest code.
   `None` as the host means admission is denied, and `open()` returns null.
   Use `initialize_root_auxiliary_utf16_source_positions` or
   `initialize_auxiliary_utf16_source_positions` when CSP authorization needs
   original UTF-16 token/attribute provenance; these staged variants retain the
   native parser's positions and nonce records without executing guest init.
2. `OpenHost` receives the source context, an optional selected existing target,
   and the fully converted `OpenRequest`. Enforce activation, sandbox, CSP,
   navigation, isolation and popup policy here. Denial returns `None`.
3. For a new target, allocate a **real** context through
   `create_auxiliary_window_context(scope, None)`. Set its initial about:blank
   origin and policy; call `initialize_auxiliary_document` for its independent
   DOM and install the runtime's native hooks. This staged helper deliberately
   defers registered guest init scripts. The root equivalent is
   `initialize_root_auxiliary_document` when root services use the primary tree.
   Return the context synchronously. An arbitrary JS object or uninitialized
   context is rejected. For an existing target, return that same real proxy.
4. The producer installs canonical name/opener/closed/document bindings and
   commits its identity. Do not execute child guest scripts or drain guest tasks
   inside the initial `OpenHost` admission callback.
5. `set_auxiliary_ready_host(context, Some(callback))` registers a private native
   post-commit, pre-return hook, inherited by new auxiliaries. It runs in the
   target context/document with no native borrows held. Publish the Page before
   executing its init scripts, so init-time close has a truthful creation/close
   order. Clone configuration and release host/OpState borrows before running JS:
   init scripts may synchronously open another popup or close their own target.
   Return `AuxiliaryReadyAction::InitializedByHost` after running the correct
   target's context-wide scripts. Do not copy opener Page-only init scripts to
   the new Page. `RunRegisteredInitScripts` explicitly selects the binding's
   registered-script fallback; it is also used when no ready hook is configured.
   Initial scripts finish before `open()` returns, so opener writes cannot race
   ahead of initialization.
6. `Abort` and uncaught ready-hook exceptions never select fallback scripts.
   The producer calls `CloseHost` for only the newly owned target, retires it,
   and propagates the original exception (or returns null for a plain abort).
   A surface-install failure removes the uncommitted identity/name registration
   and also calls `CloseHost`; the host must release its pending allocation even
   if that target never reached ready/Page publication.
7. Queue authored fetch/navigation according to host policy after initialization.
   `request.navigate == false`
   means an omitted/empty URL; an existing target must retain its current
   document. `request.noopener` may produce a new Page while the JS return value
   is null. A null guest result therefore cannot be used as the Page-creation
   signal.

The pinned Chrome 156 controls also distinguish named and reserved targets:
a fresh named noopener popup retains its requested name; an existing same-named
target can be reused without changing its opener while the call returns null.
`_self`/`_parent`/`_top` preserve opener relationships and return the selected
window even with noopener. The host must honor the selected target and
`new_context`, not treat every noopener request as new Page creation.

The host must make a newly allocated context owned by its actor before returning
it from admission, but defer publication/guest callbacks until the ready phase.
Ready hooks may catch/report ordinary init-script errors as target Page errors;
if an exception escapes to the producer, it is a failed ready operation with
owned-target cleanup, not successful initialization.
The producer registry is weak and never roots a Page from an isolate-global
persistent context handle.

Use `install_nested_auxiliary_source` for real iframe contexts. Their parent/top
selection and sandbox policy are separate from auxiliary opener relationships.
For hyperlink defaults, call `open_auxiliary_hyperlink` only after the host's
uncancelled click/default-action policy admits navigation. It reads native
attributes and applies the implicit `_blank` noopener rule; it does not fake a
click, ignore download/CSP, or navigate the opener in place of a popup.

## Navigation and isolation

For ordinary committed navigation, create a new backing context with
`create_auxiliary_window_context(scope, Some(previous))`. This detaches and
reattaches the real native proxy. Install the new DOM/origin/runtime capabilities,
then call `commit_auxiliary_navigation`. It preserves the context ID and refuses
different proxy identities. The native origin synchronizer revokes access from
retained lexical contexts of a different origin, including retained functions.

A COOP/group separation is different: allocate a new genuine proxy and use
`commit_isolated_auxiliary_navigation`. The old proxy becomes closed to its
retaining opener; the new active context keeps the host Page's ID and immutable
creation source but has a fresh group, no opener and an empty name. Do not emit
a Page-close event or tear down the actor for this document transition.

`sever_auxiliary_opener` clears the native opener relationship and starts a new
group, but does not by itself perform a navigation/proxy backing swap. The host
must choose the matching navigation operation for its actual isolation policy.

## Window capabilities and close

`AuxiliaryWindowHooks` supplies canonical native Location access/navigation,
structured-clone postMessage delivery, focus/blur and live child contexts. A
cross-origin Location callback must return a genuinely protected Location, never
an ordinary dictionary that leaks its href. Message and navigation callbacks
receive the actual incumbent source context and enforce their corresponding
resource and navigation policy. Missing capabilities throw explicitly.

When adapting an existing WindowProxy host, delegate canonical document-guard
installation to `install_auxiliary_window_document_guard`. Installing two
different nonconfigurable `window.document` getters is refused. Existing native
cross-origin handlers must read `auxiliary_context_opener` and
`auxiliary_context_closed` for registered contexts rather than returning a fixed
null opener or relying only on an unrelated frame-retirement flag.

`CloseHost` closes only the addressed owned auxiliary Page, cancels its resources
and returns acceptance. Native `window.close()` is idempotent; a retained proxy
then reports `closed`. Host-side Page closure, crash or cleanup calls
`retire_auxiliary_context(scope, context)` for either a root or auxiliary. It
retires the native document and releases its private opener edge without closing
another Page or the shared runtime.

## Acceptance matrix

- Native same-origin document access, genuine proxy/window/self/parent/top and
  opener identities; no object shim or separate-isolate substitute.
- `_blank`, reserved targets, case-sensitive names, live name changes, named
  reuse, empty-URL non-navigation and feature boolean/alias conversion.
- Implicit/explicit noopener and noreferrer preserve the measured target
  creation/reuse rules, initiating event attribution and referrer policy.
- Cross-origin access restrictions, accessing-realm function constructors,
  incumbent postMessage source and retained old lexical/native accessors.
- Sandbox and popup denial before allocation, and host navigation/COOP admission.
- Stable normal navigation, isolated navigation with old-proxy closure, correct
  request/response/Page attribution and once-only Page/popup events.
- Target close, opener close with live child, actor reclamation and no strong
  isolate registry retaining obsolete contexts.

The shared JS probe and localhost Chrome collector independently capture real
browser behavior and actual created targets. Rust host fixtures validate the
producer seam; the Surf integration must additionally validate its actor/Page
facades and network events.

### Root template capability

The official/default V8 150.4.2 archive supports this feature. Engine flags alone
are not capability evidence. The initial root must be constructed with
`auxiliary_window_template(scope)` (for Deno, delegate the extension's
`global_template_middleware` to it); an ordinary Deno global or an older SDK's
private Window template does not qualify. `is_auxiliary_window_context(scope,
context)` checks active native ownership and the producer-reserved two-internal-
field global layout. These fields are a trusted embedder ABI and must remain
reserved; guest prototype copying cannot manufacture them. Root installation
returns false without assigning an auxiliary identity on an unsupported profile,
so the host can retain its ordinary Page behavior. Nested-frame source admission
still requires the host's actual frame ownership/policy integration.
