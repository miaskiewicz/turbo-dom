(async () => {
  // The driver supplies fixture URLs, never a replacement browser API.
  const entry = globalThis.__popupProbeCase;
  const pause = () => new Promise(resolve => setTimeout(resolve, 5));
  async function until(predicate) {
    for (let attempt = 0; attempt < 400; attempt++) {
      const result = predicate();
      if (result) return result;
      await pause();
    }
    throw new Error('Popup fixture did not reach the required state');
  }
  function open(url = '', name = '_blank', features = '') {
    const popup = window.open(url, name, features);
    return popup;
  }
  function identity(popup) {
    return {
      admitted: popup !== null,
      window: popup.window === popup,
      self: popup.self === popup,
      top: popup.top === popup,
      parent: popup.parent === popup,
      frames: popup.frames === popup,
      opener: popup.opener === window,
      closed: popup.closed,
      frameElement: popup.frameElement === null,
      documentView: popup.document.defaultView === popup,
      distinctDocument: popup.document !== document,
      length: popup.length,
    };
  }
  {
    if (entry.scenario === 'identities') {
      const popup = open();
      return { ...identity(popup), href: popup.location.href };
    }
    if (entry.scenario === 'same-origin') {
      const popup = open();
      const child = popup.document.createElement('span');
      child.textContent = 'native popup';
      popup.document.body.appendChild(child);
      popup.__popupSentinel = { value: 7 };
      return {
        ...identity(popup),
        text: popup.document.body.textContent,
        nodeOwner: child.ownerDocument === popup.document,
        sharedObject: popup.__popupSentinel.value === 7,
        openerDocument: popup.opener.document === document,
      };
    }
    if (entry.scenario === 'named') {
      const first = open('', 'Report');
      const same = open('', 'Report');
      const other = open('', 'report');
      return {
        reused: first === same,
        caseSensitive: first !== other,
        name: first.name,
        otherName: other.name,
        opener: first.opener === window,
      };
    }
    if (entry.scenario === 'blank-keyword') {
      const first = open('', '_blank');
      const second = open('', '_BLANK');
      return { distinct: first !== second, firstName: first.name, secondName: second.name };
    }
    if (entry.scenario === 'existing-keywords') {
      const self = open('', '_self', entry.features || '');
      const parent = open('', '_parent', entry.features || '');
      const top = open('', '_top', entry.features || '');
      return {
        self: self === window,
        parent: parent === window,
        top: top === window,
        selfNull: self === null,
        parentNull: parent === null,
        topNull: top === null,
        openerIsSelf: window.opener === window,
      };
    }
    if (entry.scenario === 'close') {
      const popup = open('', 'Closable');
      popup.close();
      await until(() => popup.closed);
      const replacement = open('', 'Closable');
      return {
        retainedClosed: popup.closed,
        replacementDistinct: replacement !== popup,
        replacementOpen: !replacement.closed,
        openerOpen: !window.closed,
      };
    }
    if (entry.scenario === 'features') {
      const popup = open(entry.sameOriginUrl, '_blank', entry.features);
      return { returnedNull: popup === null, opener: popup ? popup.opener === window : null };
    }
    if (entry.scenario === 'named-noopener') {
      const previous = entry.existing ? open('', entry.target) : null;
      const popup = open(entry.sameOriginUrl, entry.target, 'noopener');
      return {
        returnedNull: popup === null,
        reused: previous && popup ? previous === popup : null,
        previousName: previous ? previous.name : null,
        previousClosed: previous ? previous.closed : null,
      };
    }
    if (entry.scenario === 'hyperlink') {
      const anchor = document.createElement('a');
      anchor.href = entry.sameOriginUrl;
      anchor.target = '_blank';
      anchor.rel = entry.rel;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      return { defaultPrevented: false };
    }
    if (entry.scenario === 'cross-origin') {
      const popup = open(entry.crossOriginUrl);
      await until(() => {
        try { void popup.document; return false; }
        catch (error) { return error.name === 'SecurityError'; }
      });
      const denied = name => {
        try { void popup[name]; return null; }
        catch (error) { return error.name; }
      };
      return {
        document: denied('document'),
        name: denied('name'),
        closed: popup.closed,
        self: popup.self === popup,
        window: popup.window === popup,
        opener: popup.opener === window,
      };
    }
    if (entry.scenario === 'sandbox') {
      const frame = document.createElement('iframe');
      frame.setAttribute('sandbox', entry.sandbox);
      const token = `popup-control-${entry.id}`;
      const result = new Promise((resolve, reject) => {
        const timeout = setTimeout(() => { window.removeEventListener('message', receive); reject(new Error('Sandbox fixture timed out')); }, 2000);
        function receive(event) {
          if (event.source !== frame.contentWindow || event.data?.token !== token) return;
          clearTimeout(timeout);
          window.removeEventListener('message', receive);
          resolve(event.data.result);
        }
        window.addEventListener('message', receive);
      });
      frame.srcdoc = `<script>const popup=window.open('about:blank','_blank');parent.postMessage({token:${JSON.stringify(token)},result:{returnedNull:popup===null,opener:popup?popup.opener===window:null}},'*');</script>`;
      document.body.appendChild(frame);
      try { return await result; }
      finally { frame.remove(); }
    }
    throw new Error(`Unknown popup control scenario: ${entry.scenario}`);
  }
})()
