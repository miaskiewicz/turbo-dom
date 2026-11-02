(() => {
  'use strict';
  // Chrome 156 CSS property registry facts for Typed OM: which css_properties.json5 names the
  // build enables; each computable property's computed value on an element no UA rule styles
  // (an autonomous custom element: the initial value, or the root's inherited value) in
  // computedStyleMap() iteration order; and the UA `display`/`unicode-bidi` of HTML elements.
  const candidates = JSON.parse(document.getElementById('candidates').textContent);
  const enabled = candidates.filter((name) => CSS.supports(name, 'inherit'));
  const probe = document.getElementById('probe');
  const computed = [];
  for (const [name, values] of probe.computedStyleMap()) computed.push([name, values.map(String)]);
  const tags = ['html','head','title','meta','link','style','script','base','noembed','noframes','area','param','a','abbr','address','article','aside','audio','b','bdi','bdo','blockquote','body','br','button','canvas','caption','center','cite','code','col','colgroup','data','datalist','dd','del','details','dfn','dialog','div','dl','dt','em','embed','fieldset','figcaption','figure','footer','form','h1','h2','h3','h4','h5','h6','header','hgroup','hr','i','iframe','img','input','ins','kbd','label','legend','li','listing','main','map','mark','menu','meter','nav','noscript','object','ol','optgroup','option','output','p','picture','pre','progress','q','rp','rt','ruby','s','samp','search','section','select','slot','small','source','span','strong','sub','summary','sup','table','tbody','td','template','textarea','tfoot','th','thead','time','tr','track','u','ul','var','video','wbr','xmp','unknown-el','x'];
  const host = document.getElementById('host');
  const ua = (name) => tags.map((tag) => { const element = document.createElement(tag); host.append(element); const value = String(element.computedStyleMap().get(name)); element.remove(); return [tag, value]; });
  return { enabled, computed, display: ua('display'), unicodeBidi: ua('unicode-bidi') };
})()
