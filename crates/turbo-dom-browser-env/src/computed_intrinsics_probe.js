(() => {
  const target=document.getElementById("target"), style=document.querySelector("style");
  const read=getComputedStyle, result={values:[],retainedValues:[],matchesReads:0,ownerReads:0,sheetReads:0,listReads:0};
  const measure=()=>{const value=read(target);result.values.push([value.width,value.color]);};
  measure();
  Object.defineProperty(target,"matches",{configurable:true,get(){result.matchesReads++;throw Error("public matches getter");}});
  measure();delete target.matches;
  const saved=Object.getOwnPropertyDescriptor(Element.prototype,"matches");
  Object.defineProperty(Element.prototype,"matches",{...saved,value:function(){result.matchesReads++;return false;}});
  try {
    measure();
    Object.defineProperty(target,"ownerDocument",{configurable:true,get(){result.ownerReads++;throw Error("public owner getter");}});
    measure();style.sheet.cssRules[0].style.setProperty("width","23px");measure();
  } finally {Object.defineProperty(Element.prototype,"matches",saved);delete target.ownerDocument;}
  const retained=read(target), originalSheet=style.sheet;
  const retainedMeasure=()=>result.retainedValues.push([retained.width,retained.color]);
  const lengthDescriptor=Object.getOwnPropertyDescriptor(StyleSheetList.prototype,"length");
  Object.defineProperty(document,"styleSheets",{configurable:true,get(){result.sheetReads++;throw Error("public stylesheet getter");}});
  Object.defineProperty(StyleSheetList.prototype,"length",{...lengthDescriptor,get(){result.listReads++;throw Error("public stylesheet length getter");}});
  try {
    retainedMeasure();
    originalSheet.insertRule(".target {width:31px;color:green}",1);retainedMeasure();
    const extra=document.createElement("style");extra.textContent=".target {width:41px;color:blue}";document.head.appendChild(extra);retainedMeasure();
    extra.remove();retainedMeasure();
    originalSheet.deleteRule(1);retainedMeasure();
    style.textContent=".target {width:47px;color:red}";retainedMeasure();
    const replacement=style.sheet;
    const fallback=document.createElement("style");fallback.textContent=".target {width:13px;color:red}";document.head.prepend(fallback);
    replacement.disabled=true;retainedMeasure();
    replacement.disabled=false;retainedMeasure();
    fallback.remove();retainedMeasure();
    result.replacedSheet=replacement!==originalSheet;
    result.retainedOldRules=originalSheet.cssRules.length;
  } finally {
    delete document.styleSheets;
    Object.defineProperty(StyleSheetList.prototype,"length",lengthDescriptor);
  }
  const inherited=document.createElement("section");inherited.style.cssText="color:blue;width:256px";
  document.body.appendChild(inherited);
  try {
    const chain=[];let parent=inherited;
    for(let index=0;index<3;index++){const child=document.createElement("div");child.style.cssText="color:inherit;width:50%";parent.appendChild(child);chain.push(child);parent=child;}
    const live=read(parent);result.inheritedValues=[];
    const inheritedMeasure=()=>result.inheritedValues.push([live.getPropertyValue("color"),live.getPropertyValue("width")]);
    inheritedMeasure();
    inherited.style.cssText="color:red;width:512px";inheritedMeasure();
    chain[1].style.color="green";inheritedMeasure();
    chain[1].style.color="inherit";inheritedMeasure();
    chain[1].style.width="25%";inheritedMeasure();
    const colorDescriptor=Object.getOwnPropertyDescriptor(CSSStyleDeclaration.prototype,"color");
    result.colorReads=0;result.computedReads=0;
    Object.defineProperty(CSSStyleDeclaration.prototype,"color",{configurable:true,get(){result.colorReads++;return "magenta";}});
    globalThis.getComputedStyle=function(){result.computedReads++;throw Error("public computed-style hook");};
    try {inheritedMeasure();}finally{
      globalThis.getComputedStyle=read;
      if(colorDescriptor)Object.defineProperty(CSSStyleDeclaration.prototype,"color",colorDescriptor);else delete CSSStyleDeclaration.prototype.color;
    }
  } finally {inherited.remove();}
  return result;
})();
