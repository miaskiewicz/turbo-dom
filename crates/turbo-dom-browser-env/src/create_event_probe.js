(() => {
  'use strict';
  // turbo-dom#341: document.createEvent(interface): the DOM createEvent table (ASCII
  // case-insensitive) plus the legacy names Chrome 156 keeps; NotSupportedError otherwise.
  // Chrome 156's global *Event interfaces.
  const globals = ["AnimationEvent", "AnimationPlaybackEvent", "AudioProcessingEvent", "BeforeInstallPromptEvent", "BlobEvent", "CharacterBoundsUpdateEvent", "ClipboardChangeEvent", "ClipboardEvent", "CloseEvent", "CommandEvent", "CompositionEvent", "ContentVisibilityAutoStateChangeEvent", "CookieChangeEvent", "CustomEvent", "DocumentPictureInPictureEvent", "DragEvent", "ErrorEvent", "Event", "FocusEvent", "FontFaceSetLoadEvent", "FormDataEvent", "GPUUncapturedErrorEvent", "GamepadEvent", "HIDConnectionEvent", "HIDInputReportEvent", "HashChangeEvent", "IDBVersionChangeEvent", "InputEvent", "InterestEvent", "KeyboardEvent", "MIDIConnectionEvent", "MIDIMessageEvent", "MediaEncryptedEvent", "MediaKeyMessageEvent", "MediaQueryListEvent", "MediaStreamEvent", "MediaStreamTrackEvent", "MessageEvent", "MouseEvent", "NavigateEvent", "NavigationCurrentEntryChangeEvent", "OfflineAudioCompletionEvent", "PageRevealEvent", "PageSwapEvent", "PageTransitionEvent", "PaymentMethodChangeEvent", "PaymentRequestUpdateEvent", "PictureInPictureEvent", "PointerEvent", "PopStateEvent", "PresentationConnectionAvailableEvent", "PresentationConnectionCloseEvent", "ProgressEvent", "PromiseRejectionEvent", "RTCDTMFToneChangeEvent", "RTCDataChannelEvent", "RTCErrorEvent", "RTCPeerConnectionIceErrorEvent", "RTCPeerConnectionIceEvent", "RTCTrackEvent", "SecurityPolicyViolationEvent", "SensorErrorEvent", "SnapEvent", "SpeechRecognitionErrorEvent", "SpeechRecognitionEvent", "SpeechSynthesisErrorEvent", "SpeechSynthesisEvent", "SubmitEvent", "TaskPriorityChangeEvent", "TextFormatUpdateEvent", "TextUpdateEvent", "ToggleEvent", "TrackEvent", "TransitionEvent", "UIEvent", "USBConnectionEvent", "VirtualKeyboardGeometryChangeEvent", "WebGLContextEvent", "WheelEvent", "WindowControlsOverlayGeometryChangeEvent", "XRInputSourceEvent", "XRInputSourcesChangeEvent", "XRLayerEvent", "XRReferenceSpaceEvent", "XRSessionEvent", "XRVisibilityMaskChangeEvent", "webkitSpeechRecognitionEvent"];
  const extra = ['Events', 'HTMLEvents', 'MouseEvents', 'UIEvents', 'SVGEvents', 'KeyEvents', 'KeyboardEvents', 'MutationEvent', 'MutationEvents', 'TextEvent', 'BeforeUnloadEvent', 'WebKitTransitionEvent', 'WebKitAnimationEvent', 'OrientationEvent', 'SVGZoomEvent', 'SVGZoomEvents', 'StorageEvent', 'TouchEvent', 'DeviceMotionEvent', 'DeviceOrientationEvent', 'event', 'EVENT', 'mouseevent', 'MOUSEEVENTS', 'customevent', 'htmlevents', 'Event ', ' Event', 'Bogus', '', 'Eventſ'];
  const names = [...new Set([...extra, ...globals])];
  const out = { table: {}, defined: {} };
  for (const name of ['Event', 'UIEvent', 'MouseEvent', 'KeyboardEvent', 'FocusEvent', 'CompositionEvent', 'TextEvent', 'DragEvent', 'CustomEvent', 'MessageEvent', 'HashChangeEvent', 'StorageEvent', 'BeforeUnloadEvent', 'DeviceMotionEvent', 'DeviceOrientationEvent', 'TransitionEvent']) out.defined[name] = typeof globalThis[name] === 'function' && (globalThis[name] === Event || globalThis[name].prototype instanceof Event) ? 'Event interface' : 'none';
  for (const name of names) {
    let e;
    try { e = document.createEvent(name); } catch (error) { out.table[name] = ['throw', error.name, error.message]; continue; }
    const own = Object.getPrototypeOf(e).constructor.name;
    out.table[name] = [own, e.type, e.bubbles, e.cancelable, e.isTrusted, e.defaultPrevented, e.eventPhase, e.target, e.composed, typeof e.timeStamp];
  }
  const e = document.createEvent('Event');
  const target = document.createElement('div');
  let seen = 0;
  target.addEventListener('x', () => seen++);
  out.dispatch = [];
  try { target.dispatchEvent(e); out.dispatch.push('dispatched'); } catch (error) { out.dispatch.push(error.name); }
  e.initEvent('x', true, true);
  out.dispatch.push(e.type, e.bubbles, e.cancelable, target.dispatchEvent(e), seen);
  const c = document.createEvent('CustomEvent');
  c.initCustomEvent('y', false, true, 42);
  out.custom = [c.type, c.cancelable, c.detail];
  const m = document.createEvent('MouseEvents');
  m.initMouseEvent('click', true, true, window, 3, 1, 2, 3, 4, true, false, false, false, 0, null);
  out.mouse = [m.type, m.detail, m.screenX, m.clientY, m.ctrlKey, m.view === window];
  const desc = Object.getOwnPropertyDescriptor(Document.prototype, 'createEvent');
  out.shape = [typeof desc.value, desc.value.name, desc.value.length, desc.writable, desc.enumerable, desc.configurable, Object.hasOwn(document, 'createEvent')];
  for (const [label, f] of [['noArgs', () => document.createEvent()], ['symbol', () => document.createEvent(Symbol())], ['receiver', () => Document.prototype.createEvent.call(document.body, 'Event')], ['xml', () => document.implementation.createDocument(null, null).createEvent('Event').constructor.name]]) {
    try { out[label] = ['ok', f()]; } catch (error) { out[label] = ['throw', error.name, error.message]; }
  }
  return out;
})()
