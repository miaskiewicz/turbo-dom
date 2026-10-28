(async () => {
  // Selection.toString's visible-text model edges: case transforms that change length, a
  // pre-line newline, a hidden <br>, floats and absolutely positioned blocks, a shadow host's
  // slotted children and slot fallbacks, text directly in a shadow root, and tables; plus
  // caret moves across a table's edges.
  const selection = getSelection();
  const out = {};
  const text = html => {
    document.body.innerHTML = html;
    selection.selectAllChildren(document.body);
    return JSON.stringify(selection.toString());
  };
  out.transforms = [
    text('<p style="text-transform:lowercase">ABC D</p>'),
    text('<p style="text-transform:uppercase">aßb ŉ</p>'),
    text('<p style="text-transform:capitalize">ab cd</p>'),
  ];
  out.whitespace = [
    text('<p style="white-space:pre-line">a  \n  b \nc</p>'),
    text('<p style="white-space:pre">a  \n b</p>'),
    text('<p>a<br style="visibility:hidden">b<br style="visibility:collapse">c</p>'),
  ];
  out.boxes = [
    text('<div>x<span style="float:left;display:block">f</span>y</div>z'),
    text('<div>x<div style="position:absolute">abs</div>y<div style="position:fixed">fix</div>w</div>'),
    text('<div>x<p style="float:right">fp</p>y</div>'),
  ];
  document.body.innerHTML = '<div id=h>L1<span slot=x>L2</span><b>L3</b></div><div id=e>E1</div>';
  const root = document.getElementById('h').attachShadow({ mode: 'open' });
  root.innerHTML = '<p>a<slot name=x>fx</slot>|<slot name=none>fallback</slot>|<slot>fd</slot>b</p>';
  selection.selectAllChildren(document.body);
  out.shadow = [JSON.stringify(selection.toString())];
  const host2 = document.getElementById('e').attachShadow({ mode: 'open' });
  host2.appendChild(document.createTextNode('root text'));
  selection.setBaseAndExtent(host2.firstChild, 1, host2.firstChild, 6);
  out.shadow.push(JSON.stringify(selection.toString()));
  out.tables = [
    text('<table><tr><td>a</td><td>b</td></tr><tr><td>c</td><td></td></tr></table>x'),
    text('x<table><tbody><tr><th>h</th></tr></tbody></table><table><tr><td>t2</td></tr></table>'),
    text('<div style="display:table"><div style="display:table-row"><div style="display:table-cell">c1</div><div style="display:table-cell">c2</div></div></div>y'),
  ];
  document.body.innerHTML = 'ab<table><tr><td>c</td><td>d</td></tr></table>ef';
  selection.collapse(document.body.firstChild, 0);
  const moves = [];
  for (let index = 0; index < 8; index++) {
    selection.modify('move', 'forward', 'character');
    moves.push([selection.focusNode.nodeName, selection.focusNode.nodeType === 3 ? selection.focusNode.data : null, selection.focusOffset]);
  }
  for (let index = 0; index < 8; index++) {
    selection.modify('move', 'backward', 'character');
    moves.push([selection.focusNode.nodeName, selection.focusNode.nodeType === 3 ? selection.focusNode.data : null, selection.focusOffset]);
  }
  out.moves = moves.map(move => move.join(':')).join(' ');
  selection.removeAllRanges();
  document.body.innerHTML = '';
  return out;
})()
