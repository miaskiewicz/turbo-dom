(async () => {
  const rows=[];
  for (const mode of ['open','closed']) {
    const host=document.createElement('div');document.body.appendChild(host);
    const root=host.attachShadow({mode});
    root.innerHTML='<style>@keyframes local {from {opacity:0} to {opacity:1}} div {animation:local .02s}</style><div></div>';
    const target=root.lastChild,animation=target.getAnimations()[0];
    const row={mode,count:target.getAnimations().length,name:animation.animationName,duration:animation.effect.getTiming().duration};
    await animation.finished;
    row.finished=animation.playState;row.remaining=target.getAnimations().length;
    rows.push(row);host.remove();
  }
  const host=document.createElement('div');document.body.appendChild(host);
  const root=host.attachShadow({mode:'closed'}),sheet=new CSSStyleSheet();
  sheet.replaceSync('@keyframes local {from {opacity:.5} to {opacity:1}} div {animation:local .02s !important}');
  root.adoptedStyleSheets=[sheet];root.innerHTML='<div style="animation:unknown 1s"></div>';
  const target=root.lastChild,animation=target.getAnimations()[0];
  const adopted={name:animation.animationName,firstOpacity:animation.effect.getKeyframes()[0].opacity,duration:animation.effect.getTiming().duration};
  await animation.finished;
  adopted.remaining=target.getAnimations().length;
  sheet.deleteRule(1);sheet.insertRule('div {animation:local .03s}',1);
  const restarted=target.getAnimations()[0];
  adopted.inlineWins=!!restarted; // The now-normal scoped declaration loses to inline.
  target.style.animation='';
  const replacement=target.getAnimations()[0];
  adopted.changedDuration=replacement.effect.getTiming().duration;
  await replacement.finished;
  adopted.completed=replacement.playState;
  host.remove();
  const outer=document.createElement('div');document.body.appendChild(outer);
  const outerRoot=outer.attachShadow({mode:'closed'});
  outerRoot.innerHTML='<style>@keyframes inheritedName {from {opacity:.25} to {opacity:1}}</style><section></section>';
  const innerRoot=outerRoot.lastChild.attachShadow({mode:'closed'});
  innerRoot.innerHTML='<style>div {animation:inheritedName .02s}</style><div></div>';
  const inherited=innerRoot.lastChild.getAnimations()[0];
  const ancestorScope={count:innerRoot.lastChild.getAnimations().length,firstOpacity:inherited&&inherited.effect.getKeyframes()[0].opacity};
  if(inherited)await inherited.finished;
  ancestorScope.remaining=innerRoot.lastChild.getAnimations().length;
  outer.remove();
  return {rows,adopted,ancestorScope};
})();
