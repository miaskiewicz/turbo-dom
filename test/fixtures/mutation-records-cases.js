var MCASES = {
  before2: function(r,x,y){ r.lastChild.before(x,y); },
  after2: function(r,x,y){ r.firstChild.after(x,y); },
  replaceWith2: function(r,x,y){ r.firstChild.replaceWith(x,y); },
  replaceWithSelf: function(r,x){ r.firstChild.replaceWith(x, r.firstChild); },
  replaceChildren2: function(r,x,y){ r.replaceChildren(x,y); },
  replaceChildrenEmpty: function(r){ r.replaceChildren(); },
  adjacentHtml: function(r){ r.insertAdjacentHTML('beforeend','<x id=x></x><y id=y></y>'); },
  adjacentHtmlAfterbegin: function(r){ r.insertAdjacentHTML('afterbegin','<x id=x></x><y id=y></y>'); },
  appendFragment: function(r,x,y,d){ var f=d.createDocumentFragment(); f.append(x,y); r.appendChild(f); },
  appendEmptyFragment: function(r,x,y,d){ r.appendChild(d.createDocumentFragment()); },
  append2: function(r,x,y){ r.append(x,'t',y); },
  prepend2: function(r,x,y){ r.prepend(x,y); },
  moveWithin: function(r){ r.append(r.firstChild); },
  innerHTML: function(r){ r.innerHTML='<x id=x></x>'; },
  innerHTMLEmpty: function(r){ r.innerHTML=''; r.innerHTML=''; },
  textContent: function(r){ r.textContent='t'; },
  textContentEmpty: function(r){ r.textContent=''; },
  removeB: function(r){ r.lastChild.remove(); },
  appendX: function(r,x){ r.appendChild(x); },
  insertBeforeB: function(r,x){ r.insertBefore(x, r.lastChild); },
  normalize: function(r){ r.append('p','q'); r.normalize(); },
  normalizeEmpty: function(r,x,y,d){ r.insertBefore(d.createTextNode(''), r.lastChild); r.normalize(); },
  splitText: function(r){ r.append('pq'); r.lastChild.splitText(1); },
  outerHTML: function(r){ r.firstChild.outerHTML='<x id=x></x><y id=y></y>'; },
  outerHTMLDetached: function(r,x){ x.outerHTML='<y></y>'; },
};
function mrun(d, MO, name){ var r=d.createElement('div'); r.id='r'; r.innerHTML='<a id=a></a><b id=b></b>'; d.body.appendChild(r);
  var x=d.createElement('x'); x.id='x'; var y=d.createElement('y'); y.id='y';
  var o=new MO(function(){}); o.observe(r,{childList:true,subtree:true}); var err='';
  try { MCASES[name](r,x,y,d); } catch(e){ err=e.name; }
  var recs=o.takeRecords(); o.disconnect(); r.remove();
  var id=function(n){ return n ? (n.id || (n.nodeType===3 ? '#text' : n.nodeName)) : null; };
  return JSON.stringify([err, recs.map(function(m){ return [id(m.target), [].map.call(m.addedNodes,id), [].map.call(m.removedNodes,id), id(m.previousSibling), id(m.nextSibling)]; })]); }
