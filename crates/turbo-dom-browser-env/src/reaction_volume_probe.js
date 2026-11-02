(async () => {
  // One [CEReactions] call that queues thousands of custom element reactions (upgrades,
  // connected / disconnected / attributeChanged callbacks) and mutation records runs to
  // completion: Chrome has no per-call reaction cap (miaskiewicz/turbo-dom#140; the binding
  // stopped at 4096).
  const counts = { constructed: 0, connected: 0, disconnected: 0, attributes: 0 };
  customElements.define('x-volume', class extends HTMLElement {
    static observedAttributes = ['data-v'];
    constructor() { super(); counts.constructed++; }
    connectedCallback() { counts.connected++; }
    disconnectedCallback() { counts.disconnected++; }
    attributeChangedCallback() { counts.attributes++; }
  });
  const snapshot = () => ({ ...counts });
  const out = {};
  document.body.innerHTML = '<x-volume data-v=1></x-volume>'.repeat(4100) + '<p>tail</p>';
  out.innerHTML = [snapshot(), document.body.childNodes.length, document.body.lastChild.textContent];
  const observer = new MutationObserver(() => {});
  observer.observe(document.body, { subtree: true, childList: true });
  document.body.textContent = '';
  out.cleared = snapshot();
  out.records = observer.takeRecords().map(record => [record.addedNodes.length, record.removedNodes.length]);
  const div = document.createElement('div');
  for (let index = 0; index < 4100; index++) div.append(document.createElement('x-volume'));
  document.body.replaceChildren(div);
  out.replaceChildren = snapshot();
  document.body.textContent = '';
  return out;
})()
