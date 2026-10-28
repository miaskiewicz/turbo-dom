(async () => {
  // A <details> whose open attribute is set by the parser (every fragment parser, template
  // contents) or copied by a clone queues a toggle task from "closed"
  // to "open", like an attribute change made by script (miaskiewicz/turbo-dom#131).
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  let log = [];
  const record = event => log.push([event.target.id, event.oldState, event.newState, event.target.isConnected]);
  document.addEventListener('toggle', record, true);
  // Disconnected details are observed through their own listeners.
  const watch = root => { for (const details of root.querySelectorAll('details')) details.addEventListener('toggle', event => { if (!event.target.isConnected) record(event); }); };
  const out = {};
  const run = async (name, body) => { log = []; body(); await wait(20); out[name] = log.slice(); };
  await run('innerHTML', () => { document.body.innerHTML = '<details id=a open></details><div><details id=b open><details id=c open></details></details></div><details id=z></details><p open id=p></p>'; });
  await run('insertAdjacentHTML', () => { document.body.insertAdjacentHTML('beforeend', '<details id=ia open></details>'); });
  await run('outerHTML', () => { document.body.innerHTML = '<p id=p></p>'; log = []; document.getElementById('p').outerHTML = '<details id=o open></details>'; });
  await run('detachedInnerHTML', () => { const div = document.createElement('div'); div.innerHTML = '<details id=di open></details>'; watch(div); });
  await run('contextualFragment', () => { const range = document.createRange(); range.selectNodeContents(document.body); const fragment = range.createContextualFragment('<details id=cf open></details>'); watch(fragment); });
  await run('setHTMLUnsafe', () => { document.body.setHTMLUnsafe('<details id=su open></details>'); });
  await run('shadowInnerHTML', () => { document.body.innerHTML = '<div id=h></div>'; log = []; const root = document.getElementById('h').attachShadow({ mode: 'open' }); root.innerHTML = '<details id=sh open></details>'; root.firstChild.addEventListener('toggle', event => log.push(['own:' + event.target.id, event.oldState, event.newState])); });
  await run('closedBeforeTask', () => { document.body.innerHTML = '<details id=pc open></details>'; document.getElementById('pc').open = false; });
  await run('reopenedBeforeTask', () => { document.body.innerHTML = '<details id=pr open></details>'; const details = document.getElementById('pr'); details.open = false; details.open = true; });
  await run('template', () => { document.body.innerHTML = '<template id=t><details id=td open></details></template>'; watch(document.getElementById('t').content); });
  await run('cloneNode', () => { document.body.innerHTML = '<details id=cl open></details>'; log = []; const clone = document.getElementById('cl').cloneNode(true); clone.id = 'cl2'; document.body.append(clone); });
  document.body.innerHTML = '';
  return out;
})()
