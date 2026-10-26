(async () => {
  const out = {};
  const tag = value => Object.prototype.toString.call(value);
  const shape = value => value === null ? null : value === undefined ? 'undefined' : typeof value === 'object' || typeof value === 'function' ? tag(value) : value;
  const failure = error => ({ name: error.name, message: error.message, type: tag(error), sameRealm: error instanceof Error });
  const run = body => { try { return { value: shape(body()) }; } catch (error) { return failure(error); } };
  const settle = async body => { let promise; try { promise = body(); } catch (error) { return { threw: failure(error) }; } if (tag(promise) !== '[object Promise]') return { notPromise: shape(promise) }; try { return { resolved: shape(await promise), samePromise: promise instanceof Promise }; } catch (error) { return { rejected: failure(error), samePromise: promise instanceof Promise }; } };
  const describe = (object, key) => {
    const descriptor = Object.getOwnPropertyDescriptor(object, key);
    if (!descriptor) return null;
    const result = {};
    for (const field of Object.keys(descriptor)) {
      const value = descriptor[field];
      result[field] = typeof value === 'function' ? { name: value.name, length: value.length } : value;
    }
    return result;
  };
  const accessor = (key, part) => Object.getOwnPropertyDescriptor(Document.prototype, key)[part];
  const pattern = /^(\d\d)\/(\d\d)\/(\d{4}) (\d\d):(\d\d):(\d\d)$/;
  // The local wall-clock string read back as an instant, so the comparison holds in any zone.
  const instant = text => { const match = pattern.exec(text); return match ? new Date(+match[3], match[1] - 1, +match[2], +match[4], +match[5], +match[6]).getTime() : NaN; };
  const lastModified = document => { const before = Math.floor(Date.now() / 1000) * 1000, text = document.lastModified, after = Date.now(), value = instant(text); return { format: pattern.test(text), now: value >= before && value <= after, fixed: value >= before && value <= after ? null : value }; };
  const members = ['domain', 'lastModified', 'fullscreen', 'fullscreenElement', 'exitFullscreen', 'webkitCancelFullScreen', 'webkitExitFullscreen', 'webkitIsFullScreen', 'webkitFullscreenElement', 'pointerLockElement', 'exitPointerLock', 'pictureInPictureElement', 'exitPictureInPicture', 'prerendering', 'wasDiscarded'];
  out.descriptors = Object.fromEntries(members.map(key => [key, describe(Document.prototype, key)]));
  out.own = members.filter(key => Object.getOwnPropertyNames(document).includes(key));
  const setDomain = (document, value) => run(() => { document.domain = value; return document.domain; });
  out.domain = { value: document.domain, same: setDomain(document, document.domain), upper: setDomain(document, document.domain.toUpperCase()) };
  const host = out.domain.value;
  if (host && !host.startsWith('[')) {
    const labels = host.split('.');
    out.domain.suffixes = labels.map((_, index) => labels.slice(index).join('.')).concat(['com', 'co.uk', 'github.io', 'localhost']).map(value => [value, setDomain(document, value)]);
  }
  out.domain.inputs = ['', '.' + host, host + '.', ' ' + host, host + ' ', 'example.com', '0x7f.0.0.1', '0.1', 'a.1', 'a.09', '1.2.3.4.5', '256.0.0.1', '[::1]', '[::1', '[::ffff:1.2.3.4]', 'x!y.example', 'x"y.example', 'x#y.example', 'x$y.example', 'x%y.example', 'x*y.example', 'x/y.example', 'x:y.example', 'x<y.example', 'x>y.example', 'x?y.example', 'x@y.example', 'x[y.example', 'x\\y.example', 'x^y.example', 'x_y.example', 'x`y.example', 'x{y.example', 'x|y.example', 'x}y.example', 'x~y.example', 'x\ty.example', 'x\u0000y.example', 'x\u007fy.example', '%41.example', '%2e', '%zz', '%', '%00', '%20', '%25', '%7f', '%c3%bc', '%ff', 'ß', '．', 'a­b', 'a‍b', '😀', 'AÜ', '。', 'İ', 'xn--zz', 'a..b', 'x'.repeat(64) + '.example'].map(value => [value, setDomain(document, value)]);
  // The original host stays assignable after relaxation; restore it so the child
  // browsing context below inherits the unrelaxed origin in every case.
  if (host && !host.startsWith('[')) { out.domain.regrow = setDomain(document, host); out.domain.after = document.domain; }
  out.domain.coercion = { undefined: setDomain(document, undefined), null: setDomain(document, null), number: setDomain(document, 1), symbol: run(() => { document.domain = Symbol('x'); }) };
  out.domain.brands = { get: run(() => accessor('domain', 'get').call({})), set: run(() => accessor('domain', 'set').call({}, 'x')), bodyGet: run(() => accessor('domain', 'get').call(document.body)), noArgument: run(() => accessor('domain', 'set').call(document)) };
  out.lastModified = { top: lastModified(document), brand: run(() => accessor('lastModified', 'get').call({})), assign: run(() => { document.lastModified = 'x'; return pattern.test(document.lastModified); }) };
  const fullscreen = {};
  fullscreen.values = { fullscreen: document.fullscreen, element: shape(document.fullscreenElement), webkitElement: shape(document.webkitFullscreenElement), webkitIs: document.webkitIsFullScreen, pointerLock: shape(document.pointerLockElement), pictureInPicture: shape(document.pictureInPictureElement), prerendering: document.prerendering, wasDiscarded: document.wasDiscarded };
  fullscreen.lenient = {
    fullscreen: run(() => { document.fullscreen = true; return document.fullscreen; }), element: run(() => { document.fullscreenElement = document.body; return shape(document.fullscreenElement); }),
    fullscreenOther: run(() => accessor('fullscreen', 'set').call({}, true)), elementOther: run(() => accessor('fullscreenElement', 'set').call({}, null)), fullscreenNoArgument: run(() => accessor('fullscreen', 'set').call(document)),
    readonly: run(() => { 'use strict'; document.prerendering = true; }),
  };
  fullscreen.brands = Object.fromEntries(['fullscreen', 'fullscreenElement', 'pointerLockElement', 'pictureInPictureElement', 'prerendering', 'wasDiscarded'].map(key => [key, run(() => accessor(key, 'get').call({}))]));
  fullscreen.methods = {
    exitFullscreen: await settle(() => document.exitFullscreen()), exitFullscreenOther: await settle(() => Document.prototype.exitFullscreen.call({})),
    exitPictureInPicture: await settle(() => document.exitPictureInPicture()), exitPictureInPictureOther: await settle(() => Document.prototype.exitPictureInPicture.call(document.body)),
    webkitExitFullscreen: run(() => document.webkitExitFullscreen()), webkitCancelFullScreen: run(() => document.webkitCancelFullScreen(1, 2)),
    webkitExitFullscreenOther: run(() => Document.prototype.webkitExitFullscreen.call({})), webkitCancelFullScreenOther: run(() => Document.prototype.webkitCancelFullScreen.call({})),
    exitPointerLock: run(() => document.exitPointerLock()), exitPointerLockOther: run(() => Document.prototype.exitPointerLock.call({})),
    exceptionPrototype: await (async () => { try { await document.exitPictureInPicture(); return null; } catch (error) { return [error instanceof DOMException, error.code]; } })(),
  };
  out.fullscreen = fullscreen;
  const inert = document.implementation.createHTMLDocument('inert');
  out.inert = {
    domain: inert.domain, setDomain: setDomain(inert, inert.domain), lastModified: lastModified(inert), fullscreen: inert.fullscreen, element: shape(inert.fullscreenElement),
    exitFullscreen: await settle(() => inert.exitFullscreen()), exitPictureInPicture: await settle(() => inert.exitPictureInPicture()), prerendering: inert.prerendering, wasDiscarded: inert.wasDiscarded,
  };
  // A same-origin child browsing context; its document belongs to the child realm.
  // An opaque-origin parent's child is a distinct opaque origin, so it is not probed.
  if (document.domain === '') return out;
  const frame = globalThis.__legacyChildFrame || (() => { const element = document.createElement('iframe'); document.body.append(element); return element; })();
  const child = frame.contentWindow, childDocument = child.document;
  out.child = {
    domain: childDocument.domain, setDomain: setDomain(childDocument, childDocument.domain), lastModified: lastModified(childDocument), fullscreen: childDocument.fullscreen, prerendering: childDocument.prerendering,
    exitFullscreen: await settle(() => childDocument.exitFullscreen()), crossDomain: accessor('domain', 'get').call(childDocument), crossExit: await settle(() => Document.prototype.exitFullscreen.call(childDocument)),
    childRealmError: await (async () => { try { await childDocument.exitFullscreen(); return null; } catch (error) { return [error instanceof child.TypeError, error instanceof TypeError]; } })(),
  };
  frame.remove();
  out.child.removed = { domain: childDocument.domain, setDomain: setDomain(childDocument, 'x'), lastModified: lastModified(childDocument).format, fullscreen: childDocument.fullscreen, exitFullscreen: await settle(() => childDocument.exitFullscreen()) };
  return out;
})();
