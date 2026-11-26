// Each reader belongs to one backend and resource policy. Only the native
// default profile opts in, with private limits decoded from its policy payload.
import { sharedDecode, sharedEncode, sharedInvoke } from './css-shared-codec.mjs';

export function cssViewportAxis(value) {
  return value === undefined ? {status: 'unknown'} : {status: 'known', value};
}

function hostInput(facts, limits) {
  if (facts.hostPlatform !== 'macos' && facts.hostPlatform !== 'other')
    throw new TypeError('CSS default host platform unavailable');
  return {abiVersion: 1, hostPlatform: facts.hostPlatform, fontOverride: facts.fontOverride ?? null, limits};
}

function invoke(backend, name, input) {
  const method = backend[name];
  if (typeof method !== 'function') throw new TypeError('CSS default profile missing ' + name);
  const result = sharedDecode(sharedInvoke(method, backend, [sharedEncode(input)]));
  if (result.abiVersion !== 1) throw new TypeError('CSS default profile ' + name + ' ABI');
  return result;
}

// The default window producer creates plain records. An inherited or accessor
// override must take the existing full read path, even if it returns undefined.
const descriptor = Object.getOwnPropertyDescriptor;
const prototype = Object.getPrototypeOf;
const plain = Object.prototype;
const own = Object.hasOwn;
function noFontOverride(facts) {
  const field = descriptor(facts, 'fontOverride');
  if (field) return own(field, 'value') && field.value == null;
  return prototype(facts) === plain && !descriptor(plain, 'fontOverride');
}

function sameWindow(previous, facts) {
  return previous.hostPlatform === facts.hostPlatform && previous.width === facts.width &&
    previous.height === facts.height && previous.devicePixelRatio === facts.devicePixelRatio &&
    previous.mediaType === facts.mediaType;
}

export function createCssWindowEnvironmentReader(backend, policy, reuse) {
  let preferenceKey, preferences, backendKey, environment, previous;
  function resolve(facts) {
    const input = hostInput(facts, policy.preferences(facts));
    const key = sharedEncode(input);
    if (preferenceKey !== key) {
      preferences = invoke(backend, 'cssComputedPreferencesV1', input);
      preferenceKey = key;
    }
    const viewport = {width: cssViewportAxis(facts.width), height: cssViewportAxis(facts.height)};
    const request = {...hostInput(facts, policy.environment(facts)), viewport};
    const nextKey = sharedEncode([request, preferences.preferenceIdentity]);
    if (backendKey !== nextKey) {
      const next = invoke(backend, 'cssComputedEnvironmentV1', request);
      if (sharedEncode(next.preferenceIdentity) !== sharedEncode(preferences.preferenceIdentity))
        throw new Error('CSS default preference identity changed');
      environment = next;
      backendKey = nextKey;
    }
    const environmentKey = sharedEncode({
      identity: preferences.preferenceIdentity, viewport: [viewport.width, viewport.height],
      devicePixelRatio: facts.devicePixelRatio, mediaType: facts.mediaType,
    });
    return {preferences, environment, environmentKey};
  }
  return function read(facts) {
    const eligible = reuse && noFontOverride(facts);
    if (eligible && previous && sameWindow(previous, facts)) return previous.result;
    previous = undefined;
    const result = resolve(facts);
    if (eligible) {
      const {hostPlatform, width, height, devicePixelRatio, mediaType} = facts;
      previous = {hostPlatform, width, height, devicePixelRatio, mediaType, result};
    }
    return result;
  };
}
