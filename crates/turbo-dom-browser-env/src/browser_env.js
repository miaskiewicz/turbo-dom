(function(){
  var g = globalThis;
  // Intrinsics captured at install time: the engine's load / insertion paths never call a
  // page-replaced builtin (miaskiewicz/turbo-dom#172).
  var $apply=Reflect.apply,$gopd=Object.getOwnPropertyDescriptor,$def=Object.defineProperty,$create=Object.create;
  var $wmGet=WeakMap.prototype.get,$wmSet=WeakMap.prototype.set,$lower=String.prototype.toLowerCase,$String=String;
  var $jsonStringifyNative=JSON.stringify,$JSON=JSON,$jsonStringify=function(value){return $apply($jsonStringifyNative,$JSON,[value]);};
  var $fromCodePoint=String.fromCodePoint,$imul=Math.imul,$Mround=Math.round,$Mmin=Math.min;
  var $call=function(f,receiver,...rest){return $apply(f,receiver,rest);};
  // Element attribute natives for the reflected members (miaskiewicz/turbo-dom#199).
  // Captured once the Element interface is defined (below), before any page script runs.
  // Node / Element / Attr natives the legacy members below call (miaskiewicz/turbo-dom#199), captured with the attribute natives.
  var $N={__proto__:null};
  var $getAttributeNative,$setAttributeNative,$removeAttributeNative,$hasAttributeNative;
  var $getAttr=function(el,name){return $apply($getAttributeNative,el,[name]);},$setAttr=function(el,name,value){return $apply($setAttributeNative,el,[name,value]);};
  var $removeAttr=function(el,name){return $apply($removeAttributeNative,el,[name]);},$hasAttr=function(el,name){return $apply($hasAttributeNative,el,[name]);};
  // The CSSOM's uncurried builtins (miaskiewicz/turbo-dom#175): $c.<method>(receiver, ...)
  // dispatches on the receiver's type to the members captured here, and grows arrays
  // through own definitions (no inherited index setters, no @@species). Any other receiver
  // (the engine's own objects) keeps its own method. $O / $R are Object / Reflect.
  var $O={__proto__:null},$R={__proto__:null},$own=Reflect.ownKeys;
  (function(){var keys=$own(Object);for(var i=0;i<keys.length;i++)$O[keys[i]]=Object[keys[i]];keys=$own(Reflect);for(i=0;i<keys.length;i++)$R[keys[i]]=Reflect[keys[i]];})();
  var $promiseResolveNative=Promise.resolve,$promiseRejectNative=Promise.reject,$PromiseType=Promise;
  var $promiseResolve=function(value){return $apply($promiseResolveNative,$PromiseType,[value]);},$promiseReject=function(reason){return $apply($promiseRejectNative,$PromiseType,[reason]);};
  var $mapSizeGetter=$gopd(Map.prototype,'size').get,$mapSize=function(map){return $apply($mapSizeGetter,map,[]);};
  var $c=(function(){
    var isArray=Array.isArray,hasOwn=Object.hasOwn,MapType=Map,WeakMapType=WeakMap,SetType=Set,WeakSetType=WeakSet,RegExpType=RegExp;
    var table=function(proto){var out={__proto__:null},keys=$own(proto);for(var i=0;i<keys.length;i++){var d=$gopd(proto,keys[i]);if(d&&typeof d.value==='function')out[keys[i]]=d.value;}return out;};
    var maps=table(Map.prototype),weakMaps=table(WeakMap.prototype),sets=table(Set.prototype),weakSets=table(WeakSet.prototype),strings=table(String.prototype),regexps=table(RegExp.prototype);
    var globalOf=$gopd(RegExp.prototype,'global').get,exec=regexps.exec;
    // One reused null-prototype row descriptor (no literal per push; defining a data
    // property never reenters page code). The value is cleared after each define.
    var row=$O.create(null);row.value=undefined;row.writable=true;row.enumerable=true;row.configurable=true;
    var add=function(list,value){row.value=value;$def(list,list.length,row);row.value=undefined;};
    var arrays={__proto__:null,
      push:function(list){for(var i=1;i<arguments.length;i++)add(list,arguments[i]);return list.length;},
      forEach:function(list,f,self){for(var i=0;i<list.length;i++)if(hasOwn(list,i))$apply(f,self,[list[i],i,list]);},
      map:function(list,f,self){var out=[];for(var i=0;i<list.length;i++)add(out,$apply(f,self,[list[i],i,list]));return out;},
      filter:function(list,f,self){var out=[];for(var i=0;i<list.length;i++)if($apply(f,self,[list[i],i,list]))add(out,list[i]);return out;},
      some:function(list,f,self){for(var i=0;i<list.length;i++)if($apply(f,self,[list[i],i,list]))return true;return false;},
      every:function(list,f,self){for(var i=0;i<list.length;i++)if(!$apply(f,self,[list[i],i,list]))return false;return true;},
      indexOf:function(list,value){for(var i=0;i<list.length;i++)if(list[i]===value)return i;return -1;},
      lastIndexOf:function(list,value){for(var i=list.length-1;i>=0;i--)if(list[i]===value)return i;return -1;},
      find:function(list,f,self){for(var i=0;i<list.length;i++)if($apply(f,self,[list[i],i,list]))return list[i];return undefined;},
      includes:function(list,value){for(var i=0;i<list.length;i++)if(list[i]===value||(list[i]!==list[i]&&value!==value))return true;return false;},
      join:function(list,separator){var out='';separator=separator===undefined?',':String(separator);for(var i=0;i<list.length;i++){if(i)out+=separator;if(list[i]!=null)out+=list[i];}return out;},
      slice:function(list,from,to){var length=list.length;from=from===undefined?0:from<0?Math.max(length+from,0):Math.min(from,length);to=to===undefined?length:to<0?Math.max(length+to,0):Math.min(to,length);var out=[];for(var i=from;i<to;i++)add(out,list[i]);return out;},
      concat:function(list){var out=[],i,k;for(i=0;i<list.length;i++)add(out,list[i]);for(k=1;k<arguments.length;k++){var item=arguments[k];if(isArray(item))for(i=0;i<item.length;i++)add(out,item[i]);else add(out,item);}return out;},
      splice:function(list,start,count){var length=list.length;start=start<0?Math.max(length+start,0):Math.min(start,length);count=arguments.length<3?length-start:Math.min(Math.max(count,0),length-start);var removed=[],rest=[],i;for(i=start;i<start+count;i++)add(removed,list[i]);for(i=start+count;i<length;i++)add(rest,list[i]);list.length=start;for(i=3;i<arguments.length;i++)add(list,arguments[i]);for(i=0;i<rest.length;i++)add(list,rest[i]);return removed;}
    };
    var Math={max:globalThis.Math.max,min:globalThis.Math.min};
    // String.prototype.replace without RegExp.prototype[@@replace] (which looks up exec).
    var replace=function(text,pattern,replacement){
      if(!(pattern instanceof RegExpType)||(typeof replacement==='string'&&$apply(strings.indexOf,replacement,['$'])>=0))return $apply(strings.replace,text,[pattern,replacement]);
      var global=$apply(globalOf,pattern,[]),out='',at=0,match;pattern.lastIndex=0;
      while((match=$apply(exec,pattern,[text]))!==null){
        var found=match[0],index=match.index,args=[found];for(var i=1;i<match.length;i++)add(args,match[i]);add(args,index);add(args,text);
        out+=$apply(strings.slice,text,[at,index])+(typeof replacement==='function'?String($apply(replacement,undefined,args)):replacement);at=index+found.length;
        if(!global)break;if(found==='')pattern.lastIndex=index+1;
      }
      pattern.lastIndex=0;return out+$apply(strings.slice,text,[at]);
    };
    // String.prototype.split by a RegExp without RegExp.prototype[@@split] (which looks up exec,
    // flags and the species constructor): a global clone of the pattern, built from the captured
    // flag getters, scans the whole string so anchors keep their meaning.
    var flagGetters=[['g','global'],['i','ignoreCase'],['m','multiline'],['s','dotAll'],['u','unicode'],['v','unicodeSets'],['d','hasIndices']],sourceOf=$gopd(RegExp.prototype,'source').get,clones=new WeakMapType(),cloneUnicode=new WeakMapType(),weakGet=weakMaps.get,weakSet=weakMaps.set;
    for(var f=0;f<flagGetters.length;f++){var flagDescriptor=$gopd(RegExp.prototype,flagGetters[f][1]);flagGetters[f][1]=flagDescriptor&&flagDescriptor.get;}
    var globalClone=function(pattern){
      var clone=$apply(weakGet,clones,[pattern]);if(clone)return clone;
      var flags='g';for(var k=1;k<flagGetters.length;k++)if(flagGetters[k][1]&&$apply(flagGetters[k][1],pattern,[]))flags+=flagGetters[k][0];
      clone=new RegExpType($apply(sourceOf,pattern,[]),flags);$apply(weakSet,clones,[pattern,clone]);$apply(weakSet,cloneUnicode,[clone,$apply(strings.indexOf,flags,['u'])>=0||$apply(strings.indexOf,flags,['v'])>=0]);return clone;
    };
    var isPair=function(text,index){var high=$apply(strings.charCodeAt,text,[index]),low=$apply(strings.charCodeAt,text,[index+1]);return high>=0xD800&&high<=0xDBFF&&low>=0xDC00&&low<=0xDFFF;};
    // After a zero-length match at `index`, advance past the current character — two code
    // units when the regex is unicode-flagged and we are on a surrogate pair.
    var splitAdvanceAfterZero = function(text, index, unicode){ return index + (unicode && isPair(text, index) ? 2 : 1); };
    var splitEmptyText=function(pattern,exec){var clone=globalClone(pattern);clone.lastIndex=0;var match=$apply(exec,clone,['']);var out=[];if(!match)add(out,'');return out;};
    var splitCaptureGroups=function(out,match,max){for(var i=1;i<match.length;i++){add(out,match[i]);if(out.length===max)return true;}return false;};
    var splitMatchStep=function(st,out,match,max,text,unicode){
      var index=match.index,end=index+match[0].length;
      if(end===st.at){st.from=splitAdvanceAfterZero(text,index,unicode);return 0;}
      add(out,$apply(strings.slice,text,[st.at,index]));if(out.length===max)return 1;
      if(splitCaptureGroups(out,match,max))return 1;
      st.at=end;st.from=end;return 0;
    };
    var split=function(text,pattern,limit){
      var out=[],max=limit===undefined?4294967295:limit>>>0;if(max===0)return out;
      if(text==='')return splitEmptyText(pattern,exec);
      var clone=globalClone(pattern),unicode=$apply(weakGet,cloneUnicode,[clone]),st={at:0,from:0},match;
      while(st.from<text.length){
        clone.lastIndex=st.from;match=$apply(exec,clone,[text]);
        if(!match||match.index>=text.length)break;
        if(splitMatchStep(st,out,match,max,text,unicode))return out;
      }
      add(out,$apply(strings.slice,text,[st.at]));return out;
    };
    // The arguments after the receiver. Array literals define their elements (no Array.prototype
    // index setter runs), so the common arities skip `add`'s per-element defineProperty, which
    // made every $c call cost a runtime call per argument.
    var restOf=function(args,a,b,c){var n=args.length;if(n<=1)return [];if(n===2)return [a];if(n===3)return [a,b];if(n===4)return [a,b,c];var rest=[];for(var i=1;i<n;i++)add(rest,args[i]);return rest;};
    var withReceiver=function(receiver,rest){var n=rest.length;if(n===0)return [receiver];if(n===1)return [receiver,rest[0]];if(n===2)return [receiver,rest[0],rest[1]];if(n===3)return [receiver,rest[0],rest[1],rest[2]];var args=[];add(args,receiver);for(var k=0;k<n;k++)add(args,rest[k]);return args;};
    var dispatchTableOf=function(receiver){
      if(receiver instanceof MapType)return maps;
      if(receiver instanceof WeakMapType)return weakMaps;
      if(receiver instanceof SetType)return sets;
      if(receiver instanceof WeakSetType)return weakSets;
      if(receiver instanceof RegExpType)return regexps;
      return null;
    };
    // The extras return their result, or NOT_HANDLED (one shared sentinel, nothing allocated
    // per call) to fall through to the receiver's own member.
    var NOT_HANDLED={__proto__:null};
    var dispatchStringExtra=function(name,receiver,rest){
      if(name==='replace')return replace(receiver,rest[0],rest[1]);
      if(name==='split'&&rest[0] instanceof RegExpType)return split(receiver,rest[0],rest[1]);
      return NOT_HANDLED;
    };
    var dispatchFunctionExtra=function(name,receiver,rest){
      if(name==='call')return $apply(receiver,rest[0],$apply(arrays.slice,null,[rest,1]));
      if(name==='apply')return $apply(receiver,rest[0],rest[1]==null?[]:rest[1]);
      return NOT_HANDLED;
    };
    var dispatchObjectResult=function(name,receiver,rest){var t=dispatchTableOf(receiver);if(t===regexps&&name==='test')return{v:$apply(exec,receiver,rest)!==null};return t;};
    var dispatchResolve=function(name,receiver,rest){
      if(typeof receiver==='string'){var sx=dispatchStringExtra(name,receiver,rest);return sx!==NOT_HANDLED?{v:sx}:{t:strings};}
      if(typeof receiver==='function'){var fx=dispatchFunctionExtra(name,receiver,rest);return fx!==NOT_HANDLED?{v:fx}:{t:null};}
      if(isArray(receiver))return arrays[name]?{v:$apply(arrays[name],null,withReceiver(receiver,rest))}:{t:null};
      var obj=dispatchObjectResult(name,receiver,rest);return obj&&obj.v!==undefined?obj:{t:obj};
    };
    var dispatch=function(name){return function(receiver,a,b,c){var rest=restOf(arguments,a,b,c);
      var r=dispatchResolve(name,receiver,rest);if(r.v!==undefined)return r.v;var t=r.t;
      if(t&&t[name])return $apply(t[name],receiver,rest);
      return $apply(receiver[name],receiver,rest);};};
    var out={__proto__:null},names=['get','set','has','delete','clear','add','slice','forEach','test','trim','push','indexOf','startsWith','splice','replace','toLowerCase','toUpperCase','map','concat','join','some','lastIndexOf','exec','filter','every','call','apply','endsWith','includes','split','charCodeAt','find'];
    for(var i=0;i<names.length;i++)out[names[i]]=dispatch(names[i]);
    return out;
  })();
  var computedStyleNativeDocumentQueryAll,compatibilityElementPrototype,computedStyleNativeMatches,computedStyleNativeGetAttribute,computedStyleNativeOwner,computedStyleNativeTag,computedStyleNativeNamespace,computedStyleNativeConnected,computedStyleNativeRoot,computedStyleNativeQueryAll;
  var normalizeComputedColor=g.__normalizeComputedColor;delete g.__normalizeComputedColor;
  var canvasColorNative=g.__canvasColorNative;delete g.__canvasColorNative;
  var canvasMeasureTextNative=g.__canvasMeasureTextNative;delete g.__canvasMeasureTextNative;
  var canvasLengthUnitNative=g.__canvasLengthUnitNative;delete g.__canvasLengthUnitNative;
  var computedFontContext=g.__computedFontContext;delete g.__computedFontContext;
  var computedStyleVersion=g.__computedStyleVersion;delete g.__computedStyleVersion;
  var computedStyleSelectorVersion=g.__computedStyleSelectorVersion;delete g.__computedStyleSelectorVersion;
  var computedStyleGeneration=g.__computedStyleGeneration;delete g.__computedStyleGeneration;
  var computedStyleFilter=g.__computedStyleFilter;delete g.__computedStyleFilter;
  var computedStyleTouch=g.__computedStyleTouch;delete g.__computedStyleTouch;
  // Test builds only: counts fresh computed-style calculations (turbo-test#125 reuse tests).
  var noteCascadeRun=g.__noteCascadeRun;delete g.__noteCascadeRun;
  var noteStyleLookup=g.__noteStyleLookup;delete g.__noteStyleLookup;
  var noteEntryBuild=g.__noteEntryBuild;delete g.__noteEntryBuild;
  var noteSheetRead=g.__noteSheetRead;delete g.__noteSheetRead;
  var noteRuleProbe=g.__noteRuleProbe;delete g.__noteRuleProbe;
  // Test builds only: counts shared computed styles; the differential test switches sharing off.
  var noteStyleShare=g.__noteStyleShare;delete g.__noteStyleShare;
  var styleShareOff=g.__styleShareOff;delete g.__styleShareOff;
  // Test builds only: counts CSS animation / transition update passes (animations.js cssUpdate).
  (function(){if(g.__noteCssUpdatePass){__tsNs.noteCssUpdatePass=g.__noteCssUpdatePass;delete g.__noteCssUpdatePass;}})();
  var cssomEpoch=g.__cssomEpoch,bumpCssomEpoch=g.__bumpCssomEpoch;delete g.__cssomEpoch;delete g.__bumpCssomEpoch;
  var cssomStructureEpoch=g.__cssomStructureEpoch,bumpCssomStructure=g.__bumpCssomStructure;delete g.__cssomStructureEpoch;delete g.__bumpCssomStructure;
  // The native live-state epoch (form / popover / dialog state the tree version misses); the
  // engine's JS state stores report their changes through __tsNs.bumpStyleState.
  var styleStateEpoch=g.__styleStateEpoch,bumpStyleState=g.__bumpStyleState;delete g.__styleStateEpoch;delete g.__bumpStyleState;__tsNs.bumpStyleState=bumpStyleState;
  var cssFontNumber=Number,cssFontString=String,cssFontFinite=Number.isFinite,cssFontPrecision=Number.prototype.toPrecision,cssFontLower=String.prototype.toLowerCase;
  var prepareInlineHandler=g.__prepareInlineHandler,noteInlineHandler=g.__noteInlineHandler;delete g.__prepareInlineHandler;delete g.__noteInlineHandler;
  var collectionGet=g.__getCollectionRecord,collectionSet=g.__setCollectionRecord;
  var appliedInlineRead=g.__readAppliedInlineStyle,appliedInlineWrite=g.__writeAppliedInlineStyle,inlineSheetAllowed=g.__isInlineSheetAllowed;
  delete g.__readAppliedInlineStyle;delete g.__writeAppliedInlineStyle;delete g.__isInlineSheetAllowed;
  function cssMap(family){return {get:function(object){var shared=collectionGet(object);return shared&&shared.css&&shared.css[family];},set:function(object,value){var shared=collectionGet(object);if(!shared){shared={kind:'CSS'};collectionSet(object,shared);}if(!shared.css)shared.css=$create(null);shared.css[family]=value;},delete:function(object){var shared=collectionGet(object);if(shared&&shared.css)delete shared.css[family];}};}
  // turbo-surf shares this binding; its internal cross-context state lives on the engine
  // namespace, a V8 private on the global (turbo-dom#222): page script cannot name it, and
  // each engine script receives it as its `__tsNs` parameter (run_engine_script).
  var __ns = __tsNs;
  // Moves a native hook the embedder put on the global onto the engine namespace.
  function adoptNsHook(globalName, key){ if (typeof g[globalName] === 'function') { __ns[key] = g[globalName]; delete g[globalName]; } }
  // The uncurried captured builtins, for the engine files that run after this bootstrap.
  __ns.intrinsics=$c;
  adoptNsHook('__parseCssNative', 'parseCssNative');
  adoptNsHook('__cssShorthandNative', 'cssShorthandNative');
  var cssShorthand = __ns.cssShorthandNative;
  var cssNativeParse=__ns.parseCssNative,cssSetPrototype=Object.setPrototypeOf,cssApply=Reflect.apply,cssMapGet=Function.prototype.call.bind(Map.prototype.get),cssTagUpper=Function.prototype.call.bind(String.prototype.toUpperCase);
  var computedSnapshotMap=WeakMap,computedSnapshotGet=Function.prototype.call.bind(WeakMap.prototype.get),computedSnapshotSet=Function.prototype.call.bind(WeakMap.prototype.set);
  adoptNsHook('__domAccessor', 'domAccessor');
  computedStyleNativeOwner=__ns.domAccessor('ownerDocument',false,'Node');
  computedStyleNativeTag=__ns.domAccessor('tagName',false,'Element');
  computedStyleNativeNamespace=__ns.domAccessor('namespaceURI',false,'Element');
  computedStyleNativeConnected=__ns.domAccessor('isConnected',false,'Node');
  var createdLocalName=__ns.domAccessor('localName',false,'Element');
  var computedStyleNativeAttribute=null;
  // An HTML <iframe>'s `width` / `height` attributes map to presentational hints (author CSS
  // wins); without them the replaced box is 300x150 (turbo-dom#288).
  function iframeSizeHints(el){
    try{if(cssApply(createdLocalName,el,[])!=='iframe'||cssApply(computedStyleNativeNamespace,el,[])!=='http://www.w3.org/1999/xhtml')return null;}catch(e){return null;}
    // Captured on first use: Element.prototype is complete once pages run.
    if(!computedStyleNativeAttribute)computedStyleNativeAttribute=g.Element.prototype.getAttribute;
    var hints={__proto__:null};
    $c.forEach([['width','300px'],['height','150px']],function(pair){var text=cssApply(computedStyleNativeAttribute,el,[pair[0]]),match=text==null?null:$c.exec(/^[\t\n\f\r ]*(\d+(?:\.\d+)?)(%?)/,text);hints[pair[0]]=match?(+match[1])+(match[2]?'%':'px'):pair[1];});
    return hints;
  }
  adoptNsHook('__nativeDOMFunction', 'makeNativeFunction');
  adoptNsHook('__nativeNamedObject', 'makeNamedObject');
  function installWindowSelfReferences(){
    if (typeof g.navigator === 'undefined') g.navigator = { userAgent: 'turbo-test', platform: 'rust', language: 'en-US', languages: ['en-US'], clipboard: {}, maxTouchPoints: 0 };
    // Top-level window self-references (browsers + jsdom + vitest): a non-framed window is its own
    // parent/top, and `self`/`frames` alias the window. `window === globalThis` here, so point them at
    // g. Configurable so a test can stub framing (Object.defineProperty(window,'parent',{value:...})).
    // Without these, `window.parent !== window` reads as "always inside an iframe" and silently breaks
    // top-level/iframe detection that works under vitest.
    $c.forEach(['self', 'parent', 'top', 'frames'], function(k){
      if (g[k] !== g) { try { Object.defineProperty(g, k, { value: g, writable: true, configurable: true, enumerable: false }); } catch(e){ try { g[k] = g; } catch(e2){} } }
    });
    if (typeof g.frameElement === 'undefined') { try { Object.defineProperty(g, 'frameElement', { value: null, writable: true, configurable: true, enumerable: false }); } catch(e){ try { g.frameElement = null; } catch(e2){} } }
  }
  installWindowSelfReferences();
  // ---- CSS shorthand expansion ----------------------------------------------------------------
  // Computed-style reads resolve longhands (marginTop, borderWidth, backgroundColor, rowGap,
  // flexBasis) of shorthands from the UA tables and Web Animations keyframes: rtdom's Chrome 156
  // shorthand grammars expand them (turbo-dom#90). `emit(prop, value)` writes one longhand; a
  // var() value reaches each longhand as written.
  function fontLower(value){return cssApply(cssFontLower,value,[]);}
  var expandShorthand = function(name, val, emit){
    var flat = cssShorthand ? cssShorthand('expand', name, String(val)) : null;
    if (!flat) return;
    for (var index = 0; index < flat.length; index += 4) emit(flat[index], flat[index + 2] === null ? flat[index + 1] : flat[index + 3]);
  };
  __ns.expandCssShorthand = expandShorthand;

  // Bounded stylesheet and inline computed-value resolution supports style/visibility reads.
  // Returned declarations refresh on public reads; internal ancestor values use private
  // snapshots from the current calculation.
  function inheritCustomProperties(decl,parent){var record=declarationRecords.get(parent);if(!record)return;$c.forEach(record.names,function(name){if($c.slice(name,0,2)==='--'&&decl[name]==null)decl[name]=cssMapGet(record.values,name).text;});}
  function computedSnapshotValue(style,name){var record=declarationRecords.get(style),value=record&&cssMapGet(record.values,name);return value?value.text:'';}
  // Keep inherited unitless line-height as a factor: its public computed px value
  // alone cannot describe inheritance after a descendant changes font-size.
  // computedFonts' static tables and fixed-shape records (no literal per computation).
  var fontUnitScale={__proto__:null,px:1,pt:96/72,pc:16,in:96,cm:96/2.54,mm:96/25.4,q:96/101.6};
  var fontSizeKeywords={__proto__:null,'xx-small':9,'x-small':10,small:13,medium:16,large:18,'x-large':24,'xx-large':32,'xxx-large':48};
  var fontStretchValues={__proto__:null,'ultra-condensed':'50%','extra-condensed':'62.5%',condensed:'75%','semi-condensed':'87.5%',normal:'100%','semi-expanded':'112.5%',expanded:'125%','extra-expanded':'150%','ultra-expanded':'200%'};
  function FontLine(kind,value,text){this.kind=kind;this.value=value;this.text=text;}
  FontLine.prototype=$O.create(null);
  function FontLength(number,dimension){this.number=number;this.dimension=dimension;}
  FontLength.prototype=$O.create(null);
  function FontState(active,family,size,sizeText,style,weight,stretch,caps,line,rootSize){this.active=active;this.family=family;this.size=size;this.sizeText=sizeText;this.style=style;this.weight=weight;this.stretch=stretch;this.caps=caps;this.line=line;this.rootSize=rootSize;}
  FontState.prototype=$O.create(null);
  var fontNormalLine=new FontLine('normal',0,undefined);
  // A child iframe document's root element is its own root (turbo-dom#299).
  function computedFontRoot(element,parent,root){return !parent&&computedFrameRoot(element)===element?element:root;}
  function fontRaw(decl,name,inheritedValue,initialValue){var value=decl[name],key=typeof value==='string'?fontLower(value):value;return !value||$c.test(/^(inherit|unset|revert|revert-layer)$/, key)?inheritedValue:key==='initial'?initialValue:value;}
  // Resolve a parsed `<number><unit>` against the surrounding `base` / `rem` lengths.
  // Hoisted out of fontLength.term so the inner closure stays a small dispatcher.
  function fontTermRelative(unit,number,base,rem){
    if(unit==='em')return base===null?null:number*base;
    if(unit==='rem')return rem===null?null:number*rem;
    return base===null?null:number*base/100;
  }
  function fontTermResolve(unit,number,base,rem,scalar){
    if(unit==='em'||unit==='rem'||unit==='%')return fontTermRelative(unit,number,base,rem);
    if(unit)return number*fontUnitScale[unit];
    return scalar||number===0?number:null;
  }
  function fontCalcProductInvalid(value,right,operator){return !value||!right||operator==='/'&&(right.number===0||right.dimension!==0)||operator==='*'&&value.dimension+right.dimension>1;}
  function fontCalcProductApply(value,right,operator){return new FontLength(operator==='*'?value.number*right.number:value.number/right.number,operator==='*'?value.dimension+right.dimension:value.dimension);}
  function fontLength(value,base,rem,typedLine){
    function term(text,scalar){
      var match=$c.exec(/^((?:\d*\.)?\d+)(px|em|rem|%|pt|pc|in|cm|mm|q)?$/i, text);if(!match)return null;
      var number=cssFontNumber(match[1]),unit=fontLower(match[2]||'');
      var resolved=fontTermResolve(unit,number,base,rem,scalar);
      return resolved!==null&&cssFontFinite(resolved)?new FontLength(resolved,unit?1:0):null;
    }
    function result(value){if(typedLine)return new FontLine(value&&value.dimension===0?'factor':'length',value?value.number:null,source);return value&&(value.dimension===1||value.number===0)?value.number:null;}
    var source=value;
    if(!$c.test(/^calc\(/i, value))return result(term(value,false));
    var expression=$c.replace($c.replace(value, /calc\(/gi,'('), /\s+/g,''),tokens=[],cursor=0,tokenPattern=/(?:\d*\.)?\d+(?:[a-z]+|%)?|[()+*/-]/gi,tokenMatch;
    tokenPattern.lastIndex=0;while((tokenMatch=$c.exec(tokenPattern,expression))!==null)$c.push(tokens,tokenMatch[0]);
    if($c.join(tokens, '')!==expression)return result(null);
    function atom(){var token=tokens[cursor++];if(token==='+'||token==='-'){var signed=atom();if(signed&&token==='-')signed.number=-signed.number;return signed;}if(token==='('){var grouped=sum();return tokens[cursor++]===')'?grouped:null;}return term(token||'',true);}
    function product(){var value=atom();while(tokens[cursor]==='*'||tokens[cursor]==='/'){var operator=tokens[cursor++],right=atom();if(fontCalcProductInvalid(value,right,operator))return null;value=fontCalcProductApply(value,right,operator);}return value;}
    function sum(){var value=product();while(tokens[cursor]==='+'||tokens[cursor]==='-'){var operator=tokens[cursor++],right=product();if(!value||!right||value.dimension!==right.dimension)return null;value=new FontLength(operator==='+'?value.number+right.number:value.number-right.number,value.dimension);}return value;}
    var calculated=sum();return result(cursor===tokens.length&&calculated&&cssFontFinite(calculated.number)?calculated:null);
  }
  function fontPixels(value){return cssFontString(cssFontNumber(cssApply(cssFontPrecision,value,[6])))+'px';}
  // font-size keywords (`smaller` / `larger` / absolute) and the negative clamp.
  function fontSizeKeywordResolve(sizeKey,inherited,initialSize){
    if(sizeKey==='smaller')return inherited.size===null?null:inherited.size/1.2;
    if(sizeKey==='larger')return inherited.size===null?null:inherited.size*1.2;
    if(fontSizeKeywords[sizeKey])return fontSizeKeywords[sizeKey]*initialSize/16;
    return null;
  }
  function computedFontSize(sizeValue,inherited,rootSize,initialSize){
    var size=typeof sizeValue==='number'?sizeValue:fontLength(sizeValue,inherited.size,rootSize);
    var sizeKey=typeof sizeValue==='string'?fontLower(sizeValue):sizeValue;
    if(size===null)size=fontSizeKeywordResolve(sizeKey,inherited,initialSize);
    if(size!==null&&size<0)size=0;
    return size;
  }
  // A value whose lower-cased form is one of `pattern`'s keywords reads lower-cased.
  function fontKeyword(value,pattern){var key=fontLower(value);return $c.test(pattern, key)?key:value;}
  function computedFontWeight(weight,inheritedWeight){
    if(weight==='normal')weight='400';else if(weight==='bold')weight='700';else if(weight==='bolder')weight=cssFontNumber(inheritedWeight)<350?'400':cssFontNumber(inheritedWeight)<550?'700':'900';else if(weight==='lighter')weight=cssFontNumber(inheritedWeight)<550?'100':cssFontNumber(inheritedWeight)<750?'400':'700';
    return weight;
  }
  function computedFontLine(decl,inherited,initial,size,rootSize){
    var lineValue=decl['line-height'],lineKey=typeof lineValue==='string'?fontLower(lineValue):lineValue,line;
    if(!lineValue||$c.test(/^(inherit|unset|revert|revert-layer)$/, lineKey))line=inherited.line;
    else if($c.test(/^(normal|initial)$/, lineKey))line=initial.line;
    else if($c.test(/^(?:\d*\.)?\d+$/, lineValue))line=new FontLine('factor',cssFontNumber(lineValue),undefined);
    else line=fontLength(lineValue,size,rootSize,true);
    if(line.value!==null&&line.value<0)line=new FontLine(line.kind,0,undefined);
    return line;
  }
  function computedFontLineText(line,size){return line.kind==='normal'?'normal':line.value===null?line.text:line.kind==='factor'&&size===null?cssFontString(line.value):fontPixels(line.kind==='factor'?line.value*size:line.value);}
  function computedFontAvailable(connected,context,parent,inherited,root,element){return !!(connected&&context&&context[4]&&(parent?inherited.active:root===element));}
  function computedFontShorthand(style,caps,weight,stretch,sizeText,lineText,family){
    var shorthand=[];
    if(style!=='normal')$c.push(shorthand, style);if(caps!=='normal')$c.push(shorthand, caps);if(weight!=='400')$c.push(shorthand, weight);
    if(stretch!=='100%'){var stretchName=$c.find($O.keys(fontStretchValues),function(key){return fontStretchValues[key]===stretch;});$c.push(shorthand, stretchName||stretch);}
    $c.push(shorthand, sizeText);if(lineText!=='normal')$c.push(shorthand, '/ '+lineText);$c.push(shorthand, family);return $c.join(shorthand, ' ');
  }
  // The default `initial` FontState for an element whose font context is `context`
  // (or a disconnected element, when `context` is null). Allocated once per
  // computedFonts call, same as the inline literal it replaces.
  function computedFontsInitial(context){
    var initialFamily=context?context[0]:'';
    var initialSize=context?context[1]:16;
    return new FontState(undefined,initialFamily,initialSize,undefined,'normal','400','100%','normal',fontNormalLine,initialSize);
  }
  // A named helper beats the per-call closure the $c.map used to allocate for every
  // computedFonts call (same body, hoisted so the family mapper is reused).
  var computedFontGenericFamily=/^(serif|sans-serif|monospace|cursive|fantasy|system-ui|ui-serif|ui-sans-serif|ui-monospace|ui-rounded|math|emoji|fangsong)$/;
  function computedFontsFamilyName(part){
    var name=$c.trim(part),key=fontLower(name);
    return $c.test(computedFontGenericFamily, key)?key:name;
  }
  // Resolve [parent, root] for `element`'s font context. A pseudo-element's parent
  // is its originating element (its root clears so inheritance restarts, Chrome).
  function computedFontsParentRoot(element,context,pseudo){
    var parent=context&&context[2],root=context&&context[3];
    if(pseudo&&context)return [element,null];
    return [parent,computedFontRoot(element,parent,root)];
  }
  function computedFonts(decl,element,parentStyle,connected,pseudo,parentSnapshot){
    var context=computedFontContext(element),pr=computedFontsParentRoot(element,context,pseudo),parent=pr[0],root=pr[1];
    var initial=computedFontsInitial(context),initialSize=initial.size;
    var inherited=parent?declarationRecords.get(parentSnapshot||parentStyle(parent)).fontState:null;
    inherited=inherited||initial;
    var rootSize=inherited.rootSize===undefined?initialSize:inherited.rootSize;
    var sizeValue=fontRaw(decl,'font-size',inherited.size===null?inherited.sizeText:inherited.size,initialSize),size=computedFontSize(sizeValue,inherited,rootSize,initialSize);
    var family=fontRaw(decl,'font-family',inherited.family,initial.family),style=fontRaw(decl,'font-style',inherited.style,initial.style),weight=cssFontString(fontRaw(decl,'font-weight',inherited.weight,initial.weight));
    style=fontKeyword(style,/^(normal|italic|oblique)$/);
    weight=fontKeyword(weight,/^(normal|bold|bolder|lighter)$/);
    family=$c.join($c.map(cssParts(family,','), computedFontsFamilyName), ', ');
    weight=computedFontWeight(weight,inherited.weight);
    var stretch=fontRaw(decl,'font-stretch',inherited.stretch,initial.stretch);
    stretch=fontStretchValues[fontLower(stretch)]||stretch;
    var caps=fontRaw(decl,'font-variant-caps',fontRaw(decl,'font-variant',inherited.caps,initial.caps),initial.caps);
    caps=fontKeyword(caps,/^(normal|small-caps|all-small-caps|petite-caps|all-petite-caps|unicase|titling-caps)$/);
    var line=computedFontLine(decl,inherited,initial,size,rootSize);
    var available=computedFontAvailable(connected,context,parent,inherited,root,element);
    var state=new FontState(available,family,size,cssFontString(sizeValue),style,weight,stretch,caps,line,root===element?size:rootSize);
    var lineText=computedFontLineText(line,size);
    var values={'font-family':family,'font-size':size===null?cssFontString(sizeValue):fontPixels(size),'font-style':style,'font-weight':weight,'font-stretch':stretch,'font-variant-caps':caps,'line-height':lineText};
    values.font=computedFontShorthand(style,caps,weight,stretch,values['font-size'],lineText,family);
    $c.forEach($O.keys(values), function(name){var value=available?values[name]:'';decl[name]=value;decl[$c.replace(name, /-([a-z])/g,function(_,letter){return $c.toUpperCase(letter);})]=value;});
    return state;
  }
  function computeStyleFrameRedirect(computeStyle,el,args){var view=computedFrameView(el);if(view!==mediaView)return __ns.withMediaView(view,function(){return cssApply(computeStyle,g,args);});return undefined;}
  function computeStyleCacheEntry(el,pseudo){return !pseudo&&el&&typeof el==='object'?computedSnapshotGet(computedStyleCache,el):null;}
  if (typeof g.getComputedStyle === 'undefined') g.getComputedStyle = function computeStyle(el){
    if(frameRootCount){var redirect=computeStyleFrameRedirect(computeStyle,el,arguments);if(redirect!==undefined)return redirect;}
    var pseudo=computedPseudoTarget(arguments[1]);
    if(pseudo===null)return __ns.makeComputedStyleDeclaration({},null,null);
    var snapshot=computeStyleSnapshot(el,new computedSnapshotMap(),undefined,pseudo||undefined);
    var entry=computeStyleCacheEntry(el,pseudo);
    return entry&&entry.snapshot===snapshot?__ns.cloneComputedStyleDeclaration(snapshot,el):snapshot;
  };
  // Chrome 156 pseudo-elements getComputedStyle computes, without arguments (turbo-dom#47).
  // ::cue and the unknown -webkit- / -internal- names read the element; ::scroll-marker (no
  // box without a scroll-marker-group) and the rest read an empty declaration.
  var computedPseudoNames={__proto__:null};
  'after backdrop before checkmark column details-content file-selector-button first-letter first-line grammar-error marker picker-icon placeholder scroll-marker-group search-text selection spelling-error target-text view-transition -webkit-scrollbar -webkit-scrollbar-thumb -webkit-scrollbar-track -webkit-scrollbar-button -webkit-scrollbar-corner -webkit-scrollbar-track-piece -webkit-resizer'.split(' ').forEach(function(name){computedPseudoNames[name]=true;});
  // Functional pseudo-elements and the argument each accepts: a name (not `*`), `select`, or
  // a scroll direction / `*`.
  var computedPseudoFunctions={__proto__:null,'highlight':'name','view-transition-group':'name','view-transition-image-pair':'name','view-transition-new':'name','view-transition-old':'name','picker':'select','scroll-button':'direction'};
  var scrollButtonDirections={__proto__:null,'*':1,'up':1,'down':1,'left':1,'right':1,'block-start':1,'block-end':1,'inline-start':1,'inline-end':1};
  function cssEscapeHexCode(code){return code===0||code>0x10FFFF||code>=0xD800&&code<=0xDFFF?'\uFFFD':$apply($fromCodePoint,String,[code]);}
  function cssEscapeHex(text,st){
    var hex=$c.exec(/^[0-9a-fA-F]{1,6}/,$c.slice(text,st.at+1,st.at+7));
    if(!hex)return null;
    var code=parseInt(hex[0],16);st.at+=1+hex[0].length;if($c.test(/[ \t\n\r\f]/,text[st.at]||''))st.at++;return cssEscapeHexCode(code);
  }
  function cssEscapeChar(text,st){
    var hex=cssEscapeHex(text,st);if(hex!==null)return hex;
    var next=text[st.at+1];if(next===undefined||next==='\n')return null;st.at+=2;return next;
  }
  function cssIdentIsStart(c,start){return $c.test(/[A-Za-z_\u0080-\uFFFF]/,c)||!start&&$c.test(/[0-9-]/,c);}
  function cssIdentifierPrefix(text){
    var st={at:0},name='',start=true;
    if(text[st.at]==='-'){name+='-';st.at++;if(text[st.at]==='-'){name+='-';st.at++;start=false;}}
    while(st.at<text.length){
      var c=text[st.at],part=null;
      if(c==='\\')part=cssEscapeChar(text,st);
      else if(cssIdentIsStart(c,start)){part=c;st.at++;}
      if(part===null)break;
      name+=part;start=false;
    }
    if(start)return null;
    return {name:$c.replace(name,/[A-Z]/g,function(c){return $c.toLowerCase(c);}),length:st.at};
  }
  // The pseudo-element a getComputedStyle(element, pseudoElt) read targets (turbo-dom#30, #47),
  // per Chrome 156: '' reads the element (no argument, a string without a leading colon that
  // is not a bare before / after, ::cue, unknown -webkit- / -internal- names), null is an
  // empty declaration (invalid, or a form Chrome computes no style for), else the
  // pseudo-element: its lowercase name, functional ones with their argument (`highlight(x)`,
  // `picker(select)`).
  // `scroll-button(direction)` — direction is `*` or a scroll keyword.
  function computedPseudoDirection(name,argument,word){
    var direction=argument==='*'?'*':word&&word.length===argument.length?word.name:'';
    return scrollButtonDirections[direction]?name+'('+direction+')':null;
  }
  // A functional pseudo-element argument: `name(` ... `)` per its kind.
  function computedPseudoFunctionalArg(name,kind,argument,word){
    if(kind==='direction')return computedPseudoDirection(name,argument,word);
    if(!word||word.length!==argument.length)return null;
    return kind==='select'?word.name==='select'?name+'(select)':null:name+'('+argument+')';
  }
  function computedPseudoFunctional(name,rest,ident,colons){
    var kind=computedPseudoFunctions[name];
    if(colons!==2||rest[ident.length]!=='('||rest[rest.length-1]!==')'||!kind)return null;
    var argument=$c.replace($c.slice(rest,ident.length+1,-1),/^[ \t\n\r\f]+|[ \t\n\r\f]+$/g,''),word=cssIdentifierPrefix(argument);
    return computedPseudoFunctionalArg(name,kind,argument,word);
  }
  // A non-functional name after one or two colons.
  function computedPseudoPlain(name,colons,bare){
    if(colons===1)return bare||name==='first-line'||name==='first-letter'?name:null;
    if(computedPseudoNames[name])return name;
    return name==='cue'||$c.slice(name,0,8)==='-webkit-'||$c.slice(name,0,10)==='-internal-'?'':null;
  }
  // No leading colon: only a bare before / after names a pseudo-element.
  function computedPseudoUncoloned(name,rest,ident){
    if(!ident)return '';if(ident.length<rest.length)return null;return name==='before'||name==='after'?name:'';
  }
  function computedPseudoTargetColoned(colons,name,rest,ident){
    if(colons>2||!ident)return null;
    if(ident.length<rest.length)return computedPseudoFunctional(name,rest,ident,colons);
    return computedPseudoPlain(name,colons,name==='before'||name==='after');
  }
  function computedPseudoTarget(value){
    if(value===undefined||value===null)return '';
    var text=$c.replace(''+value,/\/\*[\s\S]*?(?:\*\/|$)/g,'');
    var colons=0;while(text[colons]===':')colons++;
    var rest=$c.slice(text,colons),ident=cssIdentifierPrefix(rest),name=ident&&ident.name;
    if(colons===0)return computedPseudoUncoloned(name,rest,ident);
    return computedPseudoTargetColoned(colons,name,rest,ident);
  }
  // Page-replaceable numeric and string intrinsics never participate in this read.
  var cssZoomFround=Math.fround,cssZoomExec=RegExp.prototype.exec,cssZoomTrim=String.prototype.trim;
  function computedZoom(value, inherited){
    var text = typeof value === 'string' ? cssApply(cssFontLower,cssApply(cssZoomTrim,value,[]),[]) : '';
    if (text === 'inherit') return inherited() || '1';
    var calc = cssApply(cssZoomExec,/^calc\((.*)\)$/,[text]); if (calc) text = calc[1];
    var match = cssApply(cssZoomExec,/^([0-9.e+-]+)(%?)$/,[text]), factor = match ? cssZoomFround(cssFontNumber(match[1]) / (match[2] ? 100 : 1)) : 1;
    return !(factor > 0) || !cssFontFinite(factor) ? '1' : cssFontString(cssFontNumber(cssApply(cssFontPrecision,factor,[6])));
  }
  // Typed OM computes from the same cascade, read before used-value resolution.
  __ns.typedOmCascade={cascade:function(el){var sink={__proto__:null,values:null,order:{__proto__:null},count:0};computeStyleSnapshot(el,new computedSnapshotMap(),sink);return sink.values?{values:sink.values,order:sink.order}:null;},color:function(text){return normalizeComputedColor(text);},parent:function(el){var context=computedFontContext(el);return context?context[2]:null;}};
  // Chrome 156 html.css display / position for an element no author rule styles (turbo-surf#36;
  // computed_defaults_chrome156.json). The attribute-dependent rules are matched natively.
  var uaDisplayByTag = (function(){
    var table = { __proto__: null }, add = function(display, names){ $c.forEach($c.split(names, ' '), function(name){ table[name] = display; }); };
    add('none', 'area base basefont datalist head link meta noembed noframes param rp script style template title');
    add('block', 'html address blockquote body center dialog div figure figcaption footer form header hr legend listing main p plaintext pre search xmp article aside h1 h2 h3 h4 h5 h6 hgroup nav section dir dd dl dt menu ol ul fieldset details summary optgroup option frameset frame');
    add('list-item', 'li');
    add('inline-block', 'input select button textarea marquee meter progress');
    add('table', 'table'); add('table-caption', 'caption'); add('table-column-group', 'colgroup'); add('table-column', 'col');
    add('table-header-group', 'thead'); add('table-row-group', 'tbody'); add('table-footer-group', 'tfoot'); add('table-row', 'tr'); add('table-cell', 'td th');
    add('ruby', 'ruby'); add('ruby-text', 'rt'); add('contents', 'slot');
    return table;
  })();
  var uaDisplayRules = [
    ['[hidden]:not([hidden=until-found i]):not(embed)', 'none'],
    ['dialog:not([open])', 'none'],
    ['[popover]:not(:popover-open):not(dialog[open])', 'none'],
    ['input[type=hidden i]', 'none'],
    ['audio:not([controls])', 'none'],
    ['details > summary:first-of-type', 'list-item'],
  ];
  function computedNativeMatches(el, selector){ try { return !!cssApply(computedStyleNativeMatches, el, [selector]); } catch(e){ return false; } }
  function computedUserAgentMathBox(el, namespace){
    var parent = null; try { var context = computedFontContext(el); parent = context ? context[2] : null; } catch(e){}
    var parentNamespace = ''; try { if (parent) parentNamespace = cssApply(computedStyleNativeNamespace, parent, []); } catch(e){}
    var inMath = parentNamespace === namespace;
    return { display: inMath || computedNativeMatches(el, '[display=block i]') ? 'block math' : 'math', position: 'static' };
  }
  function computedUserAgentDisplayHtml(el, name){
    for (var i = 0; i < uaDisplayRules.length; i++) if (computedNativeMatches(el, uaDisplayRules[i][0])) return uaDisplayRules[i][1];
    return uaDisplayByTag[name] || 'inline';
  }
  function computedUserAgentPositionHtml(el, name, layer){
    if (computedNativeMatches(el, '[popover]') || layer === 'modal') return 'fixed';
    return name === 'dialog' ? 'absolute' : 'static';
  }
  function computedUserAgentBox(el, layer){
    var namespace = '', tag = '';
    try { namespace = cssApply(computedStyleNativeNamespace, el, []); tag = cssApply(computedStyleNativeTag, el, []); } catch(e){}
    if (namespace === 'http://www.w3.org/1998/Math/MathML') return computedUserAgentMathBox(el, namespace);
    if (namespace !== 'http://www.w3.org/1999/xhtml') return { display: 'inline', position: 'static' };
    var name = cssApply(cssFontLower, String(tag), []);
    return { display: computedUserAgentDisplayHtml(el, name), position: computedUserAgentPositionHtml(el, name, layer), topLayer: layer !== '' };
  }
  // turbo-dom#30, #58: why `el` is in the top layer — 'modal' (a dialog shown with showModal(),
  // which matches :modal), 'fullscreen' (in the fullscreen stack; the root element takes no
  // fullscreen UA values in Chrome 156), 'popover' (an open popover) — or '' when it is not.
  function computedTopLayer(el){
    var namespace = '', tag = '';
    try { namespace = cssApply(computedStyleNativeNamespace, el, []); tag = cssApply(computedStyleNativeTag, el, []); } catch(e){}
    if (computedNativeMatches(el, ':fullscreen:not(:root)')) return 'fullscreen';
    if (namespace !== 'http://www.w3.org/1999/xhtml') return '';
    if (tag === 'DIALOG' && computedNativeMatches(el, ':modal')) return 'modal';
    return computedNativeMatches(el, '[popover]') && computedNativeMatches(el, ':popover-open') ? 'popover' : '';
  }
  // ::before / ::after are children of `el`: flex / grid items when it is a flex / grid
  // container (or, as display: contents, when its own parent is).
  function computedPseudoInContainer(el, parentStyle){
    var display = computedSnapshotValue(parentStyle(el), 'display');
    if (display === 'contents') return computedFlexOrGridItem(el, parentStyle);
    return $c.test(/^(inline-)?(flex|grid)$/, display);
  }
  // html.css form controls (turbo-dom#61): `font: -webkit-small-control` (13.3333px Arial;
  // monospace for textarea and the date / time inputs), optgroup bolder, option normal; the measured button
  // box values. `tag` is the HTML element's tagName.
  // turbo-dom#318: buttons and the button-like inputs have ButtonFace, rgb(239, 239, 239).
  var monospaceInputTypes = { __proto__: null, 'date': 1, 'time': 1, 'datetime-local': 1, 'month': 1, 'week': 1 };
  var buttonInputTypes = { __proto__: null, 'button': 1, 'submit': 1, 'reset': 1 };
  var formControlTagUA = { __proto__: null,
    'BUTTON': function(){ return { 'display': 'inline-block', 'font': '13.3333px Arial', 'margin': '0px', 'padding': '1px 6px', 'border-width': '2px', 'vertical-align': 'baseline', 'background-color': 'rgb(239, 239, 239)' }; },
    'TEXTAREA': function(){ return { 'font': '13.3333px monospace' }; },
    'SELECT': function(){ return { 'font': '13.3333px Arial' }; },
    'OPTGROUP': function(){ return { 'font-weight': 'bolder' }; },
    'OPTION': function(){ return { 'font-weight': 'normal' }; }
  };
  function formControlInputUA(el){
    var type = ''; try { type = cssApply(cssFontLower, String(cssApply(computedStyleNativeGetAttribute, el, ['type']) || ''), []); } catch(e){}
    var ua = { 'font': '13.3333px ' + (monospaceInputTypes[type] ? 'monospace' : 'Arial') };
    if (buttonInputTypes[type]) ua['background-color'] = 'rgb(239, 239, 239)';
    return ua;
  }
  function formControlUserAgent(el, tag){
    var handler = formControlTagUA[tag];
    if (handler) return handler();
    return tag === 'INPUT' ? formControlInputUA(el) : null;
  }
  // html.css font sizes / weights of headings and b / strong (turbo-dom#305).
  var textUserAgent = (function(){
    var table = { __proto__: null }, sizes = ['2em', '1.5em', '1.17em', '1em', '0.83em', '0.67em'];
    for (var level = 1; level <= 6; level++) table['H' + level] = { 'font-size': sizes[level - 1], 'font-weight': 'bold' };
    table.B = table.STRONG = { 'font-weight': 'bolder' };
    return table;
  })();
  function htmlUserAgent(el, tag){ return formControlUserAgent(el, tag) || textUserAgent[tag] || null; }
  // Computed values of properties no declaration sets (turbo-dom#305; the unset-properties case
  // of computed_defaults_chrome156.json), read lazily per property so a snapshot stores only
  // its declared values: [initial value, inherited]. `currentcolor` resolves to the element's
  // color; a border side's width is 0px while its style is none / hidden.
  var computedInitialValues = (function(){
    var table = { __proto__: null }, add = function(inherited, pairs){ for (var i = 0; i < pairs.length; i += 2) table[pairs[i]] = [pairs[i + 1], inherited]; };
    add(false, ['background-color', 'rgba(0, 0, 0, 0)', 'background-image', 'none', 'background-repeat', 'repeat', 'background-position', '0% 0%',
      'outline-style', 'none', 'outline-width', '3px', 'outline-color', 'currentcolor', 'outline-offset', '0px',
      'text-decoration-line', 'none', 'text-decoration-style', 'solid', 'text-decoration-color', 'currentcolor',
      'float', 'none', 'clear', 'none', 'overflow-x', 'visible', 'overflow-y', 'visible', 'box-sizing', 'content-box', 'z-index', 'auto',
      'flex-grow', '0', 'flex-shrink', '1', 'flex-basis', 'auto', 'flex-direction', 'row', 'flex-wrap', 'nowrap', 'justify-content', 'normal',
      'align-items', 'normal', 'align-self', 'auto', 'align-content', 'normal', 'order', '0', 'row-gap', 'normal', 'column-gap', 'normal',
      'max-width', 'none', 'max-height', 'none', 'user-select', 'auto', 'box-shadow', 'none', 'vertical-align', 'baseline', 'filter', 'none',
      'mix-blend-mode', 'normal', 'isolation', 'auto', 'object-fit', 'fill', 'resize', 'none', 'table-layout', 'auto',
      'transition-duration', '0s', 'transition-property', 'all', 'animation-name', 'none', 'animation-duration', '0s', 'will-change', 'auto',
      'clip-path', 'none', 'content-visibility', 'visible', 'text-overflow', 'clip', 'touch-action', 'auto', 'scroll-behavior', 'auto', 'appearance', 'none',
      'opacity', '1', 'transform', 'none']);
    add(true, ['text-align', 'start', 'text-transform', 'none', 'text-indent', '0px', 'letter-spacing', 'normal', 'word-spacing', '0px',
      'white-space-collapse', 'collapse', 'text-wrap-mode', 'wrap', 'cursor', 'auto', 'direction', 'ltr', 'writing-mode', 'horizontal-tb',
      'pointer-events', 'auto', 'text-shadow', 'none', 'list-style-type', 'disc', 'list-style-position', 'outside', 'list-style-image', 'none',
      'border-collapse', 'separate', 'overflow-wrap', 'normal', 'word-break', 'normal', 'tab-size', '8', 'quotes', 'auto', 'caret-color', 'currentcolor',
      'accent-color', 'auto', 'empty-cells', 'show', 'caption-side', 'top']);
    $c.forEach(['top', 'right', 'bottom', 'left'], function(side){
      table['margin-' + side] = table['padding-' + side] = ['0px', false];
      table['border-' + side + '-style'] = ['none', false]; table['border-' + side + '-color'] = ['currentcolor', false];
      table['border-' + side + '-width'] = ['3px', false]; table[side] = ['auto', false];
    });
    $c.forEach(['top-left', 'top-right', 'bottom-right', 'bottom-left'], function(corner){ table['border-' + corner + '-radius'] = ['0px', false]; });
    return table;
  })();
  // html.css block margins (in em of the element's font size) and list paddings / styles.
  var blockMarginEm = { __proto__: null, P: 1, DL: 1, PRE: 1, UL: 1, OL: 1, MENU: 1, DIR: 1, BLOCKQUOTE: 1, FIGURE: 1, H1: 0.67, H2: 0.83, H3: 1, H4: 1.33, H5: 1.67, H6: 2.33 };
  var listTags = { __proto__: null, UL: 1, OL: 1, MENU: 1, DIR: 1 };
  var appearanceAuto = { __proto__: null, INPUT: 1, BUTTON: 1, SELECT: 1, TEXTAREA: 1, METER: 1, PROGRESS: 1 };
  function computedPixels(value){ return cssFontString(cssFontNumber(cssApply(cssFontPrecision, value, [6]))) + 'px'; }
  function computedNativeTag(el){ try { return el && cssApply(computedStyleNativeNamespace, el, []) === 'http://www.w3.org/1999/xhtml' ? cssApply(computedStyleNativeTag, el, []) : ''; } catch(e){ return ''; } }
  function computedParentElement(el){ try { var context = computedFontContext(el); return context ? context[2] : null; } catch(e){ return null; } }
  // ul / ol / menu / dir ancestors, up to 2 (html.css's disc / circle / square nesting).
  function computedListDepth(record){ var depth = 0; for (var node = record.defaultsParent; node && depth < 2; node = computedParentElement(node)) if (listTags[computedNativeTag(node)]) depth++; return depth; }
  function computedBlockMargin(record, tag){
    if (listTags[tag] && listTags[computedNativeTag(record.defaultsParent)]) return '0px';
    var size = record.fontState && typeof record.fontState.size === 'number' ? record.fontState.size : 16;
    return computedPixels(blockMarginEm[tag] * size);
  }
  // html.css margins: body 8px, block margins in em, blockquote / figure 40px sides.
  function computedUserAgentMargin(record, side, tag){
    if (tag === 'BODY') return '8px';
    if (blockMarginEm[tag] && (side === 'top' || side === 'bottom')) return computedBlockMargin(record, tag);
    return (tag === 'BLOCKQUOTE' || tag === 'FIGURE') && (side === 'left' || side === 'right') ? '40px' : undefined;
  }
  // html.css paddings: lists' 40px inline-start, table cells' 1px.
  function computedUserAgentPadding(record, side, tag){
    if (tag === 'TD' || tag === 'TH') return '1px';
    if (!listTags[tag] || side === 'top' || side === 'bottom') return undefined;
    return (side === 'left') === (propertyValue(record, 'direction') !== 'rtl') ? '40px' : undefined;
  }
  function computedUserAgentDirection(record){
    var dir = ''; try { dir = cssApply(cssFontLower, String(cssApply(computedStyleNativeGetAttribute, record.defaultsOf, ['dir']) || ''), []); } catch(e){}
    return dir === 'rtl' || dir === 'ltr' ? dir : undefined;
  }
  // html.css's value of `name` for this HTML element (`tag`), or undefined.
  function computedUserAgentDefault(record, name, tag){
    if ($c.slice(name, 0, 7) === 'margin-') return computedUserAgentMargin(record, $c.slice(name, 7), tag);
    if ($c.slice(name, 0, 8) === 'padding-') return computedUserAgentPadding(record, $c.slice(name, 8), tag);
    if (name === 'list-style-type' && listTags[tag]) return tag === 'OL' ? 'decimal' : ['disc', 'circle', 'square'][computedListDepth(record)];
    if (name === 'appearance' && appearanceAuto[tag]) return 'auto';
    return name === 'direction' ? computedUserAgentDirection(record) : undefined;
  }
  // An inherited property whose initial value is currentcolor: the nearest declared value of the
  // element's ancestors, else the element's color.
  function computedInheritedColor(record, name){
    for (var node = record.defaultsParent; node; node = computedParentElement(node)) {
      var declared = $c.get($c.get(declarationRecords, computeStyleSnapshot(node, new computedSnapshotMap())).values, name);
      if (declared) return declared.text;
    }
    return propertyValue(record, 'color');
  }
  // The parent a connected element's snapshot inherits unset properties from (null at the
  // root), or undefined for snapshots without defaults (pseudo-elements, disconnected).
  function defaultsParentOf(el, pseudo, connected, parent){ return connected && !pseudo && el ? parent : undefined; }
  // `border-<side>-width` reads 0px when the matching style is none/hidden (Chrome).
  function computedDefaultBorderWidth(record, name, entry){
    return $c.test(/^(none|hidden)$/, propertyValue(record, $c.replace(name, 'width', 'style'))) ? '0px' : entry[0];
  }
  // `auto` top/right/bottom/left reads '' when position is not `static` (used value).
  function computedDefaultSideAuto(record, name, entry){
    return entry[0] === 'auto' && $c.test(/^(top|right|bottom|left)$/, name) && propertyValue(record, 'position') !== 'static';
  }
  // The computed value of `name` for a connected element's snapshot no declaration set, or ''.
  function computedDefaultInherited(record, name, entry){
    if (entry[0] === 'currentcolor') return computedInheritedColor(record, name);
    if (record.defaultsParent) return propertyValue($c.get(declarationRecords, computeStyleSnapshot(record.defaultsParent, new computedSnapshotMap())), name);
    return entry[0];
  }
  function computedDefault(record, name){
    var entry = computedInitialValues[name];
    if (!entry) return '';
    var ua = computedUserAgentDefault(record, name, computedNativeTag(record.defaultsOf));
    if (ua !== undefined) return ua;
    if (entry[1]) return computedDefaultInherited(record, name, entry);
    if (entry[0] === 'currentcolor') return propertyValue(record, 'color');
    if ($c.test(/^border-[a-z]+-width$/, name)) return computedDefaultBorderWidth(record, name, entry);
    if (computedDefaultSideAuto(record, name, entry)) return '';
    return entry[0];
  }
  // CSS-wide keywords compute to the value they select (turbo-dom#317; computed_defaults case
  // css-wide-keywords) for the properties computedInitialValues knows: initial -> the
  // initial value (currentcolor -> the element's `color`), inherit -> the parent's computed
  // value (the root's: initial), unset -> inherit or initial by whether the property
  // inherits, revert -> html.css's value, i.e. the lazy default (setProp already applied a UA
  // table's). A border width resolved here is 0px while its side's style is none / hidden.
  var wideKeywordInitials = { __proto__: null, width: ['auto', false], height: ['auto', false] };
  // The value `inherit` / `initial` resolves to (currentcolor -> the element's `color`),
  // or undefined for any other key (which the caller skips).
  function wideKeywordValue(key, entry, parentRecord, decl, name){
    if (key === 'inherit' && parentRecord) return propertyValue(parentRecord, name);
    if (key === 'inherit' || key === 'initial') return entry[0] === 'currentcolor' ? decl.color : entry[0];
    return undefined;
  }
  function resolveWideKeywords(decl, parentRecord, setProp, drop){
    var widths = [];
    $c.forEach($O.keys(decl), function(name){
      var entry = computedInitialValues[name] || wideKeywordInitials[name], value = decl[name];
      if (!entry || typeof value !== 'string') return;
      var key = $c.toLowerCase($c.trim(value));
      if (key === 'revert') { drop(name); return; }
      if (key === 'unset') key = entry[1] ? 'inherit' : 'initial';
      var resolved = wideKeywordValue(key, entry, parentRecord, decl, name);
      if (resolved === undefined) return;
      setProp(name, resolved);
      if ($c.test(/^border-[a-z]+-width$/, name)) $c.push(widths, name);
    });
    $c.forEach(widths, function(name){
      var style = decl[$c.replace(name, 'width', 'style')];
      if ($c.test(/^(none|hidden)$/, $c.toLowerCase($c.trim(style == null ? 'none' : String(style))))) setProp(name, '0px');
    });
  }
  // The used value a rendered box's width / height reads (turbo-dom#316; computed_defaults
  // case used-size): the embedder's layout, `__ns.layoutUsedSize(element)`, answers
  // [border-box width, border-box height, content-box width, content-box height] in unzoomed
  // CSS px, or null for an element without a box; box-sizing picks the box (Blink's
  // UsedBoxSize). Elements Chrome reports the computed value for (computedUsedApplies), and
  // those the layout gives no box, read their declared value or `auto`; without a layout a
  // rendered box reads its declared value (or "": no used value to report, the null return).
  var usedReplacedTags = { __proto__: null, IMG: 1, CANVAS: 1, VIDEO: 1, AUDIO: 1, IFRAME: 1, EMBED: 1, OBJECT: 1, INPUT: 1, SELECT: 1, TEXTAREA: 1, BUTTON: 1 };
  var usedSvgBoxes = { __proto__: null, rect: 1, image: 1, foreignObject: 1 };
  // The index into the `[bw, bh, cw, ch]` tuple for `name` at the current `box-sizing`.
  function computedUsedBoxIndex(record, name){
    return (name === 'height' ? 1 : 0) + (propertyValue(record, 'box-sizing') === 'border-box' ? 0 : 2);
  }
  function computedDeclaredOrAuto(declared){ return declared ? declared.text : 'auto'; }
  function computedUsedSize(record, name){
    var el = record.defaultsOf;
    if ((name !== 'width' && name !== 'height') || !el) return null;
    var declared = cssMapGet(record.values, name);
    if (!computedUsedApplies(record, el)) return computedDeclaredOrAuto(declared);
    var provider = __ns.layoutUsedSize;
    if (typeof provider !== 'function') return null;
    var value = NaN;
    try { var box = provider(el); value = box ? cssFontNumber(box[computedUsedBoxIndex(record, name)]) : NaN; } catch(e){}
    return cssFontFinite(value) ? computedUsedPixels(value) : computedDeclaredOrAuto(declared);
  }
  // Blink's WidthOrHeightShouldReturnUsedValue: a box (not display none / contents) that is not
  // a non-atomic inline; of SVG children only rect / image / foreignObject (the outermost
  // <svg> is a replaced box). Whether an ancestor's display hides the box is the layout's.
  function computedUsedAppliesSvg(el, namespace, tag){
    var parent = computedParentElement(el), parentNamespace = '';
    try { parentNamespace = parent ? cssApply(computedStyleNativeNamespace, parent, []) : ''; } catch(e){}
    return parentNamespace !== namespace || usedSvgBoxes[tag] === 1;
  }
  function computedUsedApplies(record, el){
    var entry = cssMapGet(record.values, 'display'), display = entry ? entry.text : '';
    if (display === 'none' || display === 'contents') return false;
    var namespace = '', tag = '';
    try { namespace = cssApply(computedStyleNativeNamespace, el, []); tag = cssApply(computedStyleNativeTag, el, []); } catch(e){}
    if (namespace === 'http://www.w3.org/2000/svg') return computedUsedAppliesSvg(el, namespace, tag);
    if (namespace === 'http://www.w3.org/1998/Math/MathML') return true;
    return !$c.test(/^(inline|ruby|ruby-text)$/, display) || namespace === 'http://www.w3.org/1999/xhtml' && usedReplacedTags[tag] === 1;
  }
  // Blink serializes a used px value with printf's %g: six significant digits, an exact tie
  // rounding to even where toPrecision rounds it away from zero (5.328125 -> 5.32812).
  function computedUsedPixels(value){
    var text = cssApply(cssFontPrecision, value, [6]);
    var digits = $c.replace($c.replace(cssApply(cssFontPrecision, value, [21]), /[-.]|e.*$/g, ''), /^0+/, '');
    if ($c.test(/^\d{5}[02468]50*$/, digits)) text = cssApply(cssFontPrecision, 2 * value - cssFontNumber(text), [6]);
    return cssFontString(cssFontNumber(text)) + 'px';
  }
  // CSS Position 4: a top-layer box whose position is static / relative / sticky is absolute.
  function topLayerPosition(position){ return $c.test(/^(static|relative|sticky)$/, position) ? 'absolute' : position; }
  // Chrome 156 html.css values for a modal dialog (`dialog:modal`) and for ::backdrop; the
  // max sizes (calc(100% - 6px - 2em)) resolve against the dialog's own font size later.
  var modalDialogUserAgent = { 'overflow': 'auto', 'top': '0px', 'bottom': '0px', 'visibility': 'visible' };
  function backdropUserAgent(layer){
    var inset = layer ? '0px' : 'auto';
    return { 'top': inset, 'right': inset, 'bottom': inset, 'left': inset, 'background-color': layer === 'modal' ? 'rgba(0, 0, 0, 0.1)' : layer === 'fullscreen' ? 'rgb(0, 0, 0)' : 'rgba(0, 0, 0, 0)' };
  }
  // Chrome 156 fullscreen.css `:not(:root):fullscreen` (turbo-dom#58): object-fit is an
  // ordinary UA declaration, the rest are !important and beat every author declaration.
  var fullscreenUserAgent = { 'object-fit': 'contain' };
  var fullscreenImportant = { 'position': 'fixed', 'inset': '0px', 'margin': '0px', 'box-sizing': 'border-box', 'min-width': '0px', 'max-width': 'none', 'min-height': '0px', 'max-height': 'none', 'width': '100%', 'height': '100%', 'transform': 'none' };
  // +1 for a bracket open, -1 for a bracket close, 0 for anything else. Hoisted and
  // allocation-free — V8 inlines it into the character loop of selectorListItems.
  function selectorListItemsBracket(c){
    if (c === '(' || c === '[') return 1;
    if (c === ')' || c === ']') return -1;
    return 0;
  }
  // Split a selector list at its top-level commas (outside (), [] and strings).
  function selectorListIsQuoteChar(c){ return c === '"' || c === "'"; }
  function selectorListItems(text){
    var items = [], depth = 0, quote = '', start = 0;
    for (var i = 0; i < text.length; i++) {
      var c = text[i];
      if (c === '\\') { i++; continue; }
      if (quote) { if (c === quote) quote = ''; continue; }
      if (selectorListIsQuoteChar(c)) quote = c;
      else {
        var bracket = selectorListItemsBracket(c);
        if (bracket) depth += bracket;
        else if (c === ',' && depth === 0) { $c.push(items, $c.slice(text, start, i)); start = i + 1; }
      }
    }
    $c.push(items, $c.slice(text, start));
    return items;
  }
  // Selector aliases of pseudo-elements (`::-webkit-input-placeholder` is `::placeholder`).
  var pseudoSelectorAliases = { __proto__: null, '-webkit-input-placeholder': 'placeholder', '-webkit-file-upload-button': 'file-selector-button' };
  // Does a style rule's selector list select `pseudo` of `el`: some complex selector ends with
  // ::pseudo (the legacy single-colon form for before / after / first-line / first-letter; a
  // functional pseudo-element's argument must match, `*` matching every one) and its
  // originating part matches `el`. `pseudo` is a computedPseudoTarget name, or a predicate
  // over the selector's pseudo-element name.
  // Whether a selector item's pseudo-element (`name`, `argument`) is the `wanted` one.
  function pseudoNameWanted(name, argument, wanted){
    if (name !== wanted[1] || (argument === undefined) !== (wanted[2] === undefined)) return false;
    if (argument !== undefined && argument !== '*' && wanted[2] !== '*') {
      return wanted[1] === 'picker' || wanted[1] === 'scroll-button' ? $c.toLowerCase(argument) === wanted[2] : argument === wanted[2];
    }
    return true;
  }
  function pseudoItemMatchIsEscaped(match, item){ return match.index > 0 && item[match.index - 1] === '\\'; }
  function pseudoItemMatchesName(name, argument, wanted, pseudo){ return wanted ? pseudoNameWanted(name, argument, wanted) : pseudo(name); }
  function pseudoItemMatches(el, item, wanted, pseudo){
    var match = $c.exec(/(::?)([A-Za-z-]+)(?:\(\s*([^()]*?)\s*\))?$/, item);
    if (!match || pseudoItemMatchIsEscaped(match, item)) return false;
    var name = $c.toLowerCase(match[2]), argument = match[3];
    name = pseudoSelectorAliases[name] || name;
    if (match[1] === ':' && !$c.test(/^(before|after|first-line|first-letter)$/, name)) return false;
    if (!pseudoItemMatchesName(name, argument, wanted, pseudo)) return false;
    var origin = $c.slice(item, 0, match.index);
    if (origin === '' || $c.test(/[\s>+~]$/, origin)) origin += '*';
    return computedNativeMatches(el, origin);
  }
  function pseudoRuleMatches(el, selectorText, pseudo){
    var items = selectorListItems(String(selectorText));
    var wanted = typeof pseudo === 'string' ? $c.exec(/^([a-z-]+)(?:\((.*)\))?$/, pseudo) : null;
    for (var i = 0; i < items.length; i++) if (pseudoItemMatches(el, $c.trim(items[i]), wanted, pseudo)) return true;
    return false;
  }
  // turbo-dom#47: how a UA-styled pseudo-element computes in Chrome 156. Each kind lists the
  // author properties it accepts (null: all), its html.css declarations (`ua`, and
  // `important` ones authors cannot override), its display / position, whether a flex / grid
  // container blockifies it, and whether author rules reach it at all (element-backed
  // pseudo-elements exist only on their elements: ::placeholder on text inputs / textarea with a
  // placeholder, ::file-selector-button on file inputs, ::details-content on details,
  // ::picker(select) on select).
  function cssPropertySet(text){ var set = { __proto__: null }; $c.forEach($c.split(text, ' '), function(name){ set[name] = true; }); return set; }
  var pseudoFontProperties = 'font font-family font-size font-style font-variant font-variant-caps font-variant-numeric font-variant-ligatures font-variant-east-asian font-variant-alternates font-variant-position font-weight font-stretch font-feature-settings font-kerning font-optical-sizing font-size-adjust font-synthesis font-variation-settings font-palette line-height letter-spacing word-spacing';
  var pseudoAnimationProperties = 'animation animation-name animation-duration animation-timing-function animation-delay animation-iteration-count animation-direction animation-fill-mode animation-play-state transition transition-property transition-duration transition-timing-function transition-delay transition-behavior';
  var pseudoTextProperties = 'background background-color background-image background-position background-repeat background-size background-attachment background-clip background-origin text-decoration text-decoration-line text-decoration-style text-decoration-color text-decoration-thickness text-underline-offset text-underline-position text-emphasis text-emphasis-color text-emphasis-style text-emphasis-position text-shadow text-transform vertical-align -webkit-text-fill-color -webkit-text-stroke -webkit-text-stroke-color -webkit-text-stroke-width';
  var markerProperties = cssPropertySet(pseudoFontProperties + ' ' + pseudoAnimationProperties + ' color content direction unicode-bidi text-transform tab-size text-combine-upright text-emphasis text-emphasis-color text-emphasis-position text-emphasis-style text-shadow');
  var firstLineProperties = cssPropertySet(pseudoFontProperties + ' ' + pseudoTextProperties + ' color opacity visibility');
  var highlightProperties = cssPropertySet('color background-color text-decoration text-decoration-line text-decoration-style text-decoration-color text-decoration-thickness text-underline-offset text-shadow text-emphasis-color -webkit-text-fill-color -webkit-text-stroke -webkit-text-stroke-color -webkit-text-stroke-width stroke-color fill-color stroke-width');
  var highlightPseudoNames = cssPropertySet('selection highlight spelling-error grammar-error target-text search-text');
  // turbo-dom#374: computed `<color>` longhands Chrome 156 serializes as rgb() / rgba()
  // (`color` and pseudos' `background-color` are canonicalized earlier in computeStyle).
  var computedColorLonghands = ['background-color', 'border-top-color', 'border-right-color', 'border-bottom-color', 'border-left-color', 'border-block-start-color', 'border-block-end-color', 'border-inline-start-color', 'border-inline-end-color', 'outline-color', 'text-decoration-color', 'text-emphasis-color', 'column-rule-color', 'caret-color', 'accent-color', 'fill', 'stroke', 'flood-color', 'lighting-color', 'stop-color', '-webkit-text-fill-color', '-webkit-text-stroke-color'];
  function pseudoAccepts(kind){
    if (kind === 'marker') return function(name){ return markerProperties[name] === true; };
    if (kind === 'first-line') return function(name){ return firstLineProperties[name] === true; };
    if (kind === 'first-letter') return function(name){ return firstLineProperties[name] === true || name === 'float' || $c.test(/^(margin|padding|border)(-|$)/, name) || name === 'box-shadow'; };
    if (kind === 'highlight') return function(name){ return highlightProperties[name] === true; };
    return null;
  }
  var placeholderInputTypes = { __proto__: null, '': 1, 'text': 1, 'search': 1, 'url': 1, 'tel': 1, 'email': 1, 'password': 1, 'number': 1 };
  var knownInputTypes = cssPropertySet('hidden text search tel url email password date month week time datetime-local number range color checkbox radio file submit image reset button');
  function computedHtmlTag(el){
    try { if (cssApply(computedStyleNativeNamespace, el, []) === 'http://www.w3.org/1999/xhtml') return cssApply(computedStyleNativeTag, el, []); } catch(e){}
    return '';
  }
  function computedInputType(el){
    var type = ''; try { type = cssApply(cssFontLower, String(cssApply(computedStyleNativeGetAttribute, el, ['type']) || ''), []); } catch(e){}
    return knownInputTypes[type] ? type : '';
  }
  function computedHasAttribute(el, name){ try { return cssApply(computedStyleNativeGetAttribute, el, [name]) !== null; } catch(e){ return false; } }
  // An inherited property's value at `el`: the nearest authored value on it or a composed
  // ancestor (`fallback` at the root); `own(node)` may supply a UA value for a node.
  function inheritedAuthoredNodeValue(node, name, parentStyle, own){
    var value = '';
    try { value = computedSnapshotValue(parentStyle(node), name); } catch(e){}
    if (!value && own) value = own(node);
    return value;
  }
  function inheritedAuthoredIsTerminal(value){ return value && !$c.test(/^(inherit|unset)$/i, value); }
  function computedInheritedAuthored(el, name, parentStyle, fallback, own){
    var node = el;
    for (var depth = 0; node && depth < 256; depth++) {
      var value = inheritedAuthoredNodeValue(node, name, parentStyle, own);
      if (inheritedAuthoredIsTerminal(value)) return $c.test(/^initial$/i, value) ? fallback : value;
      var context = null; try { context = computedFontContext(node); } catch(e){}
      node = context ? context[2] : null;
    }
    return fallback;
  }
  function markerInside(el, parentStyle){
    var position = computedInheritedAuthored(el, 'list-style-position', parentStyle, 'outside', function(node){
      var shorthand = ''; try { shorthand = computedSnapshotValue(parentStyle(node), 'list-style'); } catch(e){}
      var word = $c.exec(/(?:^|\s)(inside|outside)(?:\s|$)/i, shorthand || '');
      if (word) return word[1];
      return computedHtmlTag(node) === 'SUMMARY' && computedNativeMatches(node, 'details > summary:first-of-type') ? 'inside' : '';
    });
    return $c.test(/^inside$/i, position);
  }
  // computedPseudoInfo's per-kind UA defaults, one writer per pseudo-element family.
  function pseudoInfoMarker(info, el, parentStyle){
    var inside = markerInside(el, parentStyle);
    info.display = inside ? 'inline' : 'inline-block';
    info.ua = { 'white-space': inside ? computedInheritedAuthored(el, 'white-space', parentStyle, 'normal') : 'pre', 'font-variant-numeric': 'tabular-nums', 'unicode-bidi': 'isolate', 'text-transform': 'none' };
  }
  function pseudoInfoHighlight(info){
    info.highlight = true; info.container = false;
    if (info.kind === 'spelling-error' || info.kind === 'grammar-error') info.ua = { 'text-decoration-line': info.kind };
  }
  function pseudoInfoPlaceholder(info, el, tag){
    var input = tag === 'INPUT' && placeholderInputTypes[computedInputType(el)];
    info.authored = (input || tag === 'TEXTAREA') && computedHasAttribute(el, 'placeholder');
    if (info.authored) {
      info.display = 'block';
      info.ua = { 'color': 'rgb(117, 117, 117)', 'unicode-bidi': 'isolate' };
      if (input) { info.ua['white-space'] = 'pre'; info.ua.overflow = 'hidden'; }
      info.important = { 'pointer-events': 'none' };
    }
  }
  function pseudoInfoFileButton(info, el, tag){
    info.authored = tag === 'INPUT' && computedInputType(el) === 'file';
    if (info.authored) {
      info.display = 'inline-block';
      info.ua = { 'background-color': 'rgb(239, 239, 239)', 'white-space': 'nowrap', 'padding': '1px 6px', 'margin': '0px 4px 0px 0px', 'border-width': '2px', 'border-style': 'outset', 'text-align': 'center', 'box-sizing': 'border-box', 'user-select': 'none' };
    }
  }
  function pseudoInfoDetailsContent(info, el, tag){
    info.authored = tag === 'DETAILS';
    if (info.authored) { info.display = 'block'; info.ua = { 'content-visibility': computedHasAttribute(el, 'open') ? 'visible' : 'hidden' }; }
  }
  function pseudoInfoPicker(info, tag){
    info.authored = tag === 'SELECT';
    if (info.authored) { info.display = 'none'; info.position = 'fixed'; info.ua = { 'overflow': 'auto', 'white-space': 'normal', 'unicode-bidi': 'isolate' }; }
  }
  function pseudoInfoGenerated(info, kind, tag){
    info.generated = true;
    if (kind === 'picker-icon' && tag === 'SELECT') info.display = 'block';
  }
  function pseudoInfoViewTransition(info, kind){
    info.display = 'block'; info.position = 'absolute';
    info.ua = kind === 'view-transition' || kind === 'view-transition-image-pair' ? { 'top': '0px', 'right': '0px', 'bottom': '0px', 'left': '0px' } : kind === 'view-transition-group' ? { 'top': '0px', 'right': 'auto', 'bottom': 'auto', 'left': '0px' } : { 'top': '0px', 'right': 'auto', 'bottom': 'auto', 'left': 'auto' };
  }
  // Dispatch table for the per-kind element-backed pseudo-elements. Keeps pseudoInfoForKind
  // under cc 12. Each handler takes (info, el, tag) uniformly — signatures that need fewer
  // arguments ignore the extras.
  var pseudoInfoKindHandlers = { __proto__: null,
    'placeholder': function(info, el, tag){ pseudoInfoPlaceholder(info, el, tag); },
    'file-selector-button': function(info, el, tag){ pseudoInfoFileButton(info, el, tag); },
    'details-content': function(info, el, tag){ pseudoInfoDetailsContent(info, el, tag); },
    'picker': function(info, el, tag){ pseudoInfoPicker(info, tag); },
    'scroll-marker-group': function(info){ info.display = 'block'; }
  };
  // Fills `info` for `kind`; returns the kind pseudoAccepts reads (highlights share one).
  function pseudoInfoForKindTail(info, kind, el, tag){
    var handler = pseudoInfoKindHandlers[kind];
    if (handler) { handler(info, el, tag); return kind; }
    if (kind === 'checkmark' || kind === 'picker-icon') pseudoInfoGenerated(info, kind, tag);
    else if ($c.slice(kind, 0, 15) === 'view-transition' && tag) pseudoInfoViewTransition(info, kind);
    return kind;
  }
  function pseudoInfoForKind(info, kind, el, tag, parentStyle){
    if (kind === 'marker') { pseudoInfoMarker(info, el, parentStyle); return kind; }
    if (kind === 'first-line' || kind === 'first-letter') { info.container = kind === 'first-line'; return kind; }
    if (highlightPseudoNames[kind]) { pseudoInfoHighlight(info); return 'highlight'; }
    return pseudoInfoForKindTail(info, kind, el, tag);
  }
  function computedPseudoInfo(el, pseudo, parentStyle){
    var kind = $c.replace(pseudo, /\(.*$/, ''), tag = computedHtmlTag(el);
    var info = { kind: kind, authored: true, accepts: null, ua: {}, important: null, display: 'inline', position: 'static', container: true, generated: false, highlight: false };
    kind = pseudoInfoForKind(info, kind, el, tag, parentStyle);
    // ::view-transition > -group() > -image-pair() > -old() / -new().
    if ($c.slice(kind, 0, 16) === 'view-transition-') {
      var argument = $c.slice(pseudo, kind.length);
      info.parentPseudo = kind === 'view-transition-group' ? 'view-transition' : kind === 'view-transition-image-pair' ? 'view-transition-group' + argument : 'view-transition-image-pair' + argument;
    }
    info.accepts = pseudoAccepts(kind);
    return info;
  }
  // Highlight inheritance (CSS Pseudo 4): once a rule of a highlight pseudo-element (say
  // ::selection) selects an element or an ancestor, that highlight inherits from the parent's
  // (the initial values at the top); otherwise Chrome computes it from the element.
  function computedHighlightMode(el, pseudo){
    for (var node = el, depth = 0; node && depth < 256; depth++) {
      var rules = computedStyleRules(node);
      for (var i = 0; i < rules.length; i++) { try { if (pseudoRuleMatches(node, rules[i], pseudo)) return true; } catch(e){} }
      var context = null; try { context = computedFontContext(node); } catch(e){}
      node = context ? context[2] : null;
    }
    return false;
  }
  // CSS Display 3 blockification (floats, absolute / fixed positioning, flex / grid items).
  var blockified = { __proto__: null, 'inline': 'block', 'inline-block': 'block', 'inline-table': 'table', 'inline-flex': 'flex', 'inline-grid': 'grid',
    'table-row-group': 'block', 'table-header-group': 'block', 'table-footer-group': 'block', 'table-row': 'block', 'table-cell': 'block',
    'table-column-group': 'block', 'table-column': 'block', 'table-caption': 'block', 'ruby': 'block', 'ruby-text': 'block', 'math': 'block math', 'inline math': 'block math' };
  function blockifiedDisplay(display){ var key = cssApply(cssFontLower, display, []); return blockified[key] || display; }
  // Is `el` a flex / grid item: its nearest composed ancestor that is not display: contents is a
  // flex or grid container.
  function computedFlexOrGridItem(el, parentStyle){
    var node = el;
    for (var depth = 0; depth < 64; depth++) {
      var context = null; try { context = computedFontContext(node); } catch(e){}
      var parent = context ? context[2] : null;
      if (!parent) return false;
      var display = computedSnapshotValue(parentStyle(parent), 'display');
      if (display === 'contents') { node = parent; continue; }
      return $c.test(/^(inline-)?(flex|grid)$/, display);
    }
    return false;
  }
  function computedMediaChild(el){
    var node = el;
    for (var depth = 0; depth < 256; depth++) {
      var context = null; try { context = computedFontContext(node); } catch(e){}
      var parent = context ? context[2] : null;
      if (!parent) return false;
      var namespace = '', tag = '';
      try { namespace = cssApply(computedStyleNativeNamespace, parent, []); tag = cssApply(computedStyleNativeTag, parent, []); } catch(e){}
      if (namespace === 'http://www.w3.org/1999/xhtml' && (tag === 'VIDEO' || tag === 'AUDIO')) return true;
      node = parent;
    }
    return false;
  }
  // The style sheets whose rules can select `el`: its owner's sheets, plus the document's
  // adopted sheets, for elements of the document tree. A shadow tree is its own style scope:
  // document rules never select its elements (turbo-dom#68; its own rules come from the
  // native shadow-scoped cascade).
  // The owner's sheet list, reread only when its tree version or the CSSOM epoch moved: every
  // element of one cascade (the read element and each ancestor) shares it. A reread list that
  // differs from the last one (a <style> / <link> inserted, removed or re-parsed through the
  // DOM) advances the CSSOM epoch: reused styles are keyed on a mutation's node scope
  // (turbo-dom#279), which need not contain the elements a new sheet selects. An unchanged
  // list keeps its selector analysis (computedOwnerScope).
  var computedSheetLists=new computedSnapshotMap();
  // turbo-dom#299: a child iframe document's tree is a detached subtree of this tree
  // (makeChildDocument) registered with its window. Its elements cascade like connected ones,
  // from the <style> / <link> sheets of that subtree only, with @media evaluated against the
  // child window's viewport (computeStyle picks the element's view).
  var frameRoots=new computedSnapshotMap(),frameSheetLists=new computedSnapshotMap(),frameRootCount=0,frameRootOptions={composed:true},frameFirstChild=null,frameNextSibling=null;
  __ns.registerFrameRoot=function(root,win){if(!computedSnapshotGet(frameRoots,root))frameRootCount++;computedSnapshotSet(frameRoots,root,win);};
  // The registered child-document root `el` belongs to (null for every element of the page).
  function computedFrameRoot(el){
    if(!frameRootCount||!el||typeof el!=='object')return null;
    var root=null;try{root=cssApply(computedStyleNativeRoot,el,[frameRootOptions]);}catch(e){return null;}
    return root&&computedSnapshotGet(frameRoots,root)?root:null;
  }
  function computedFrameView(el){var root=computedFrameRoot(el);return root?computedSnapshotGet(frameRoots,root):null;}
  // Connected for the cascade: in the page's document, or in a child iframe document.
  function computedConnected(el){return !!cssApply(computedStyleNativeConnected,el,[])||computedFrameRoot(el)!==null;}
  function frameCollectSheets(node,out){
    for(var child=cssApply(frameFirstChild,node,[]);child;child=cssApply(frameNextSibling,child,[])){
      var sheet=null;try{sheet=cssApply(currentSheetGetter,child,[]);}catch(e){}
      if(sheet)out[out.length]=sheet;
      frameCollectSheets(child,out);
    }
  }
  // The child document's sheets in tree order, reread when its tree or the CSSOM moved.
  function frameSheets(root){
    if(!frameFirstChild){frameFirstChild=__ns.domAccessor('firstChild',false,'Node');frameNextSibling=__ns.domAccessor('nextSibling',false,'Node');}
    var version=null;try{version=computedStyleSelectorVersion(root);}catch(e){}
    var entry=computedSnapshotGet(frameSheetLists,root);
    if(entry&&entry.version===version&&entry.epoch===cssomEpoch())return entry.sheets;
    var sheets=[];frameCollectSheets(root,sheets);
    computedSnapshotSet(frameSheetLists,root,{__proto__:null,version:version,epoch:cssomEpoch(),sheets:sheets});
    return sheets;
  }
  function computedSameSheets(list,sheets){if(list.length!==sheets.length)return false;for(var index=0;index<list.length;index++)if(list[index]!==sheets[index])return false;return true;}
  // The owner's sheet list, reread only after a change that can move it. Every mutation that
  // can (a stylesheet owner inserted, removed, edited or re-attributed) moves both the tree
  // version and its selector version; only a `style` attribute moves the version alone and
  // only an animation joining or leaving an element (Tree::invalidate_style quiet) moves the
  // selector version alone. So the list holds while either is where it was last validated.
  function computedSheetsEntryFresh(entry,version,full,epoch,host){
    return entry&&typeof version==='number'&&typeof full==='number'&&(entry.version===version||entry.full===full)&&entry.epoch===epoch&&entry.host===host;
  }
  function computedOwnerSheetsReuseList(entry,host,sheets){
    if(entry&&entry.host===host&&computedSameList(entry.list,sheets))return entry.sheets;
    return sheets;
  }
  function computedOwnerSheetsStore(owner,version,full,epoch,host,sheets,entry,features){
    var keep=!!features&&entry.epoch===epoch;
    if(typeof version==='number')computedSnapshotSet(computedSheetLists,owner,{__proto__:null,version:version,full:full,epoch:epoch,host:host,sheets:sheets,list:computedListCopy(sheets),scope:keep?entry.scope:undefined,features:keep?features:undefined});
  }
  function computedOwnerSheets(owner,host){
    var version=null,full=null;try{version=computedStyleSelectorVersion(owner);full=computedStyleVersion(owner);}catch(e){}
    var epoch=cssomEpoch(),entry=computedSnapshotGet(computedSheetLists,owner);
    if(computedSheetsEntryFresh(entry,version,full,epoch,host)){entry.version=version;entry.full=full;return entry.sheets;}
    if(noteSheetRead)noteSheetRead();
    var sheets=computedOwnerSheetsReuseList(entry,host,host.readSheets());
    var features=entry?computedListFeatures(owner,host,entry,sheets,epoch):undefined;
    epoch=cssomEpoch();
    computedOwnerSheetsStore(owner,version,full,epoch,host,sheets,entry,features);
    return sheets;
  }
  function computedSameList(a,b){if(a.length!==b.length)return false;for(var index=0;index<a.length;index++)if(a[index]!==b[index])return false;return true;}
  function computedListCopy(sheets){var list=[];for(var index=0;index<sheets.length;index++)list[index]=sheets[index];return list;}
  // The owner's selector features after a reread: the entry's when the list is the same, the
  // extended ones when sheets were only added (computedSheetsAdded); otherwise the CSSOM epoch
  // advances and there are none.
  function computedListFeatures(owner,host,entry,sheets,epoch){
    if(entry.host===host&&computedSameSheets(entry.list,sheets))return entry.features;
    var features=entry.host===host&&entry.epoch===epoch?computedSheetsAdded(owner,entry,sheets):null;
    if(!features){bumpCssomEpoch();return undefined;}
    computedSnapshotSet(computedEntryBuilds,sheets,null);
    try{computedStyleFilter(owner,features);}catch(e){}
    return features;
  }
  // Sheets added to an analysed list (emotion appends a <style> per rule; MUI's cache
  // prepends): like Chrome's rule-set invalidation, the elements the added rules can select
  // (their selectors with state and user-action pseudo-classes relaxed and pseudo-elements
  // dropped) move their style generation instead of the CSSOM epoch dropping every reused
  // style. Only when every old sheet is still listed in order, the added sheets hold style
  // rules (under @media / @supports at most) and they widen neither the scope nor need a
  // fallback; answers the owner's selector features with the added ones, or null.
  function computedSheetsAddedCanUse(entry){ return entry.scope!==undefined&&entry.scope>=0&&entry.scope<8&&!!entry.features; }
  function computedSheetsAddedProcess(added,entry,owner,features,targets){
    for(var si=0;si<added.length;si++){
      var memo=computedSheetScope(added[si]);
      if(computedScopeJoin(entry.scope,memo.scope)!==entry.scope)return false;
      computedMergeFeatures(features,memo.features);
      if(!computedRuleTargets(sheetRecord(added[si]).rules,owner,targets))return false;
    }
    return true;
  }
  function computedSheetsAdded(owner,entry,sheets){
    if(!computedSheetsAddedCanUse(entry))return null;
    var added=computedAddedSheets(entry.list,sheets);
    if(!added)return null;
    var features=computedFeatureLists(),targets=[];
    computedMergeFeatures(features,entry.features);
    if(!computedSheetsAddedProcess(added,entry,owner,features,targets))return null;
    try{computedStyleTouch(targets);}catch(e){return null;}
    return features;
  }
  // The sheets of `sheets` not in `old`, when `old` is still listed in order; else null.
  function computedAddedSheets(old,sheets){
    var at=0,added=[];
    for(var index=0;index<sheets.length;index++){if(at<old.length&&sheets[index]===old[at])at++;else $c.push(added,sheets[index]);}
    return at===old.length?added:null;
  }
  function computedMergeFeatures(into,from){for(var kind in into)for(var index=0;index<from[kind].length;index++)$c.push(into[kind],from[kind][index]);}
  var computedUserActionPseudo=/:(?:hover|active|focus|focus-visible|focus-within|target|target-within|visited|link|any-link)(?![\w-])/gi;
  var computedPseudoElement=/::[\w-]+(?:\([^)]*\))?|:(?:before|after|first-line|first-letter)(?![\w-])/gi;
  function computedRuleTargetQuery(record,root,targets){
    var selector=computedRelaxSelector($c.replace($c.replace(String(record.selectorText||''),computedUserActionPseudo,':is(*)'),computedPseudoElement,''));
    try{
      var found=cssApply(computedStyleNativeDocumentQueryAll,root,[selector]);
      for(var fi=0;fi<found.length;fi++)$c.push(targets,found[fi]);
    }catch(e){return false;}
    return true;
  }
  function computedRuleTargets(rules,root,targets){
    var items=cssItems(rules);
    for(var index=0;index<items.length;index++){
      var record=cssRuleRecord(items[index]);
      if(record.type===4||record.type===12){if(!computedRuleTargets(record.cssRules,root,targets))return false;continue;}
      if(record.type!==1||record.cssRules&&cssItems(record.cssRules).length)return false;
      if(!computedRuleTargetQuery(record,root,targets))return false;
    }
    return true;
  }
  // What a cascade of an element of the owner's tree reads beyond the element and its
  // ancestors (turbo-dom#279), from its sheets' selectors: -1 another element's inline style
  // (an attribute selector naming `style`, directly, through a combinator or inside :has(); an
  // @container or @import rule; adopted sheets, which the scan does not see: key on the plain
  // tree version); 8 anything else no node scope covers (:has(), :empty, :checked / :default /
  // :indeterminate over other elements, :host, @scope rules, unknown pseudo-classes, escapes:
  // key on the selector version); otherwise the bits of a style generation scope
  // (Tree::style_generation): 1 the child lists along the chain as later siblings see them
  // (+, ~, :nth-child, :nth-of-type, :first-child, :first-of-type, :enabled / :disabled /
  // :read-only / :read-write through a fieldset's legend), 2 every child-list change
  // (:last-child, :only-child, :nth-last-*, :last-of-type, :only-of-type), 4 form validity
  // (:valid, :invalid, :user-valid, :user-invalid); 0 the element and its ancestors only.
  // Scanned once per sheet and CSSOM epoch.
  var computedStyleAttributeSelector=/\[\s*(?:[\w*-]*\|)?style(?![\w-])/i;
  var computedForwardPseudo=/^(?:first-child|nth-child|first-of-type|nth-of-type|enabled|disabled|read-only|read-write)$/;
  var computedBackwardPseudo=/^(?:last-child|only-child|nth-last-child|last-of-type|only-of-type|nth-last-of-type)$/;
  var computedFormPseudo=/^(?:valid|invalid|user-valid|user-invalid)$/;
  // Pseudo-classes over the element itself and its ancestors only (their attributes and the
  // state the live-state epoch or the element's own generation tracks). Pseudo-elements never
  // widen the scope: `::slotted` / `::part` select into shadow trees, and a tree with a shadow
  // root keys on its tree-wide version (`__computedStyleGeneration` answers undefined).
  var computedOwnPseudo=/^(?:not|is|where|matches|-webkit-any|root|scope|link|any-link|visited|hover|active|focus|focus-visible|focus-within|required|optional|placeholder-shown|in-range|out-of-range|autofill|-webkit-autofill|defined|popover-open|modal|open|fullscreen|-webkit-full-screen|-webkit-full-screen-ancestor|state|before|after|first-line|first-letter|placeholder|selection|marker|backdrop|file-selector-button|-webkit-[\w-]+|-moz-[\w-]+)$/;
  function computedScopeJoin(a,b){return a<0||b<0?-1:a===8||b===8?8:a|b;}
  // `features` collects the class names, ids and (lowercase) attribute names the selector
  // names, for Tree::set_style_filter.
  var computedIdent=/^-?(?:[A-Za-z_\u0080-\uffff]|-)[\w\u0080-\uffff-]*/;
  // Characters that advance `index` without changing scope bits (except `[` which may
  // signal an inline-style attribute selector). The lookup replaces a chain of `||` tests
  // that pushed computedSelectorScope's cyclomatic complexity over the limit.
  var computedSelectorSkipChars = { __proto__: null, '"': 1, "'": 1, '[': 1, '.': 1, '#': 1 };
  var computedSelectorMetaChars = { __proto__: null, '+': 1, '~': 1, ':': 1 };
  // Advance past a skip-char; returns the new `index`, or -2 / -3 for the -1 / 8
  // bail-outs the outer loop forwards as -1 / 8.
  function computedSelectorScopeSkip(text, index, c, features){
    if (c === '"' || c === "'") return computedStringEnd(text, index);
    if (c === '[') {
      var e = computedAttributeEnd(text, index, features);
      return e < 0 ? (e === -1 ? -2 : -3) : e;
    }
    return computedFeatureEnd(text, index, features);  // c === '.' || c === '#'
  }
  // Updates `meta.end` to the new `index` and returns the new `scope`, or -1 when the
  // pseudo forces the whole selector to scope 8 (the outer loop then returns 8).
  function computedSelectorScopeMeta(text, index, c, scope, meta){
    if (c === '+' || c === '~') { meta.end = index; return scope | 1; }
    var pseudo = computedPseudoAt(text, index);  // c === ':'
    meta.end = pseudo.end;
    return pseudo.scope === 8 ? -1 : (scope | pseudo.scope);
  }
  function computedSelectorScope(text,features){
    var scope = 0, meta = { end: 0 };
    for (var index = 0; index < text.length; index++) {
      var c = text[index];
      if (c === '\\') return 8;
      if (computedSelectorSkipChars[c]) {
        index = computedSelectorScopeSkip(text, index, c, features);
        if (index < 0) return index === -2 ? -1 : 8;
      } else if (computedSelectorMetaChars[c]) {
        scope = computedSelectorScopeMeta(text, index, c, scope, meta);
        if (scope < 0) return 8;
        index = meta.end;
      }
    }
    return scope;
  }
  // The document's own `name` method, else the element prototype's (the document query methods).
  function computedOwnValue(document,proto,name){var descriptor=Object.getOwnPropertyDescriptor(document,name);if(!descriptor)descriptor=Object.getOwnPropertyDescriptor(proto,name);return descriptor.value;}
  function computedStringEnd(text,index){var quote=text[index];index++;while(index<text.length&&text[index]!==quote)index++;return index;}
  // The index of the `]` closing the attribute selector at `index` (its name joins
  // `features`), -1 for one naming `style`, -2 for one the scan cannot read.
  function computedAttributeEnd(text,index,features){
    var close=index;while(close<text.length&&text[close]!==']')close++;
    if(close>=text.length)return -2;
    var attribute=$c.slice(text,index,close+1);
    if(cssApply(cssZoomExec,computedStyleAttributeSelector,[attribute]))return -1;
    var name=cssApply(cssZoomExec,/^\[\s*(?:[\w*-]*\|)?([\w-]+)/,[attribute]);
    if(!name)return -2;
    $c.push(features.attributes,$c.toLowerCase(name[1]));
    return close;
  }
  // The index of the last character of the class or id name at `index` (it joins `features`).
  function computedFeatureEnd(text,index,features){
    var ident=cssApply(cssZoomExec,computedIdent,[$c.slice(text,index+1)]);
    if(!ident)return index;
    $c.push(text[index]==='.'?features.classes:features.ids,ident[0]);
    return index+ident[0].length;
  }
  // The pseudo-class or pseudo-element at `index`: its scope bits (8: no node scope) and the
  // index of its name's last character.
  function computedPseudoAt(text,index){
    var element=text[index+1]===':';if(element)index++;
    var name=$c.toLowerCase(cssApply(cssZoomExec,/^[A-Za-z0-9_-]*/,[$c.slice(text,index+1)])[0]),scope=8;
    if(element||cssApply(cssZoomExec,computedOwnPseudo,[name]))scope=0;
    else if(cssApply(cssZoomExec,computedForwardPseudo,[name]))scope=1;
    else if(cssApply(cssZoomExec,computedBackwardPseudo,[name]))scope=2;
    else if(cssApply(cssZoomExec,computedFormPseudo,[name]))scope=4;
    return {__proto__:null,scope:scope,end:index+name.length};
  }
  function computedRulesScopeStep(record,scope,features){
    if(record.type===3||record.kind==='container')return -2;
    if(record.kind==='scope')scope=8;
    if(record.type===7||record.type===5)return scope;
    if(record.type===1)scope=computedScopeJoin(scope,computedSelectorScope(String(record.selectorText||''),features));
    if(record.cssRules)scope=computedScopeJoin(scope,computedRulesScope(record.cssRules,features));
    return scope;
  }
  function computedRulesScope(rules,features){
    var items=cssItems(rules),scope=0;
    for(var i=0;i<items.length&&scope>=0;i++){
      scope=computedRulesScopeStep(cssRuleRecord(items[i]),scope,features);
      if(scope===-2)return -1;
    }
    return scope;
  }
  var computedSheetScopes=new computedSnapshotMap();
  function computedFeatureLists(){return {__proto__:null,classes:[],ids:[],attributes:[]};}
  function computedSheetScope(sheet){
    var epoch=cssomEpoch(),memo=computedSnapshotGet(computedSheetScopes,sheet);
    if(memo&&memo.epoch===epoch)return memo;
    var features=computedFeatureLists(),scope=8;try{scope=computedRulesScope(sheetRecord(sheet).rules,features);}catch(e){}
    memo={__proto__:null,epoch:epoch,scope:scope,features:features};
    computedSnapshotSet(computedSheetScopes,sheet,memo);
    return memo;
  }
  // The owner's scope, reported with the features its selectors name to the tree
  // (Tree::set_style_filter) when reused styles key on its style generations, so writes of
  // unnamed classes, ids and attributes move none.
  // Push `scope` / `features` into the cache entry (if still live) and ask the embedder
  // to narrow its style filter to the named features when the scope allows it.
  function computedOwnerScopeStore(owner, scope, sheets, features, entry){
    try { computedStyleFilter(owner, scope >= 0 && scope < 8 ? features : null); } catch(e){}
    if (entry && entry.sheets === sheets) { entry.scope = scope; entry.features = features; }
  }
  function computedOwnerScopeOwner(el){
    var owner=null;try{owner=cssApply(computedStyleNativeOwner,el,[]);}catch(e){return null;}
    if(!owner||computedAdopted(owner)||computedFrameRoot(el))return null;
    return owner;
  }
  function computedOwnerScope(el){
    var owner=computedOwnerScopeOwner(el);
    if(!owner)return -1;
    var ownerRecord=collectionGet(owner),host=ownerRecord&&ownerRecord.cssHost;
    if(!host)return 0;
    var sheets=computedOwnerSheets(owner,host),entry=computedSnapshotGet(computedSheetLists,owner);
    if(entry&&entry.sheets===sheets&&entry.scope!==undefined)return entry.scope;
    var features=computedFeatureLists(),scope=computedSheetsScope(sheets,features);
    computedOwnerScopeStore(owner, scope, sheets, features, entry);
    return scope;
  }
  // A tree other than the document's: a shadow tree (no document sheets) or a child iframe
  // document (its own sheets); null for the document tree and other detached elements.
  function computedScopeSheets(el,owner){
    if(cssApply(computedStyleNativeRoot,el,[])===owner)return null;
    if(cssApply(computedStyleNativeConnected,el,[]))return [];
    var frameRoot=computedFrameRoot(el);
    return frameRoot?frameSheets(frameRoot):null;
  }
  function computedSheetsScope(sheets,features){
    var scope=0;
    for(var si=0;si<sheets.length&&scope>=0;si++){var memo=computedSheetScope(sheets[si]);scope=computedScopeJoin(scope,memo.scope);computedMergeFeatures(features,memo.features);}
    return scope;
  }
  function computedAdopted(owner){var adopted=adoptionStates&&$c.get(adoptionStates,owner);return !!(adopted&&adopted.target&&adopted.target.length);}
  // Combine `sheets` (the owner's own) with `adopted.target` (adoptedStyleSheets), but
  // only when `el` is the owner itself (not a shadow tree inside it). The adopted-sheets
  // path is rare, so the extracted allocation stays off the common-case fast path.
  function computedSheetsWithAdopted(owner, el, sheets, adopted){
    if (!(adopted && adopted.target && adopted.target.length && cssApply(computedStyleNativeRoot, el, []) === owner)) return sheets;
    var active = [];
    cssSetPrototype(active, null);
    for (var i = 0; i < sheets.length; i++) active[active.length] = sheets[i];
    for (var j = 0; j < adopted.target.length; j++) active[active.length] = adopted.target[j];
    return active;
  }
  function computedSheets(el){
    var owner=el?cssApply(computedStyleNativeOwner,el,[]):null;
    var scoped=owner&&computedScopeSheets(el,owner);if(scoped)return scoped;
    // A live list refreshes on every indexed/length read. Refresh its private
    // backing array once per computation; retained declarations still recompute.
    var ownerRecord=owner&&collectionGet(owner),host=ownerRecord&&ownerRecord.cssHost;
    var sheets = host ? computedOwnerSheets(owner,host) : [];
    // Document adoption follows ordinary sheets, but is not a shadow-tree scope.
    var adopted=adoptionStates&&$c.get(adoptionStates,owner);
    return computedSheetsWithAdopted(owner, el, sheets, adopted);
  }
  // The selector text of every enabled style rule that can select `el`.
  function computedStyleRules(el){
    var out=[],sheets=[];try{sheets=computedSheets(el);}catch(e){}
    for (var si=0; si<sheets.length; si++) {
      if(sheetRecord(sheets[si]).disabled)continue;
      var entries = appliedSheetEntries(sheets[si],el,cssLayerTree());
      for (var ri=0; ri<entries.length; ri++) if(entries[ri].media!==2) $c.push(out, entries[ri].rule.selectorText);
    }
    return out;
  }
  // `pseudo` (turbo-dom#30, #47): compute that pseudo-element of `el` instead — its own rules
  // (`sel::pseudo`, the properties it accepts), its UA values, inheritance from `el`
  // (highlights: from the parent's highlight); never cached with `el`'s snapshot.
  // turbo-test#125: an element's computed style is reused across reads until something it was
  // computed from changes. The key: the element's tree version (any DOM change: structure,
  // attributes — inline style included — text, form and custom element state the selector
  // engine reads), the CSSOM epoch (every stylesheet, rule, declaration and adoption change
  // bumps it; native and shared by every realm, since a realm reads other realms' elements and
  // an edit runs in the realm owning the sheet — turbo-dom#113), the element's owner document,
  // its top-layer state (popover / modal / fullscreen), its font context (host defaults, composed parent) and, recursively, every ancestor snapshot
  // it read. A computation whose rules read live form / popover / dialog state (a selector with
  // :invalid, :checked, :popover-open, :modal, ... that matches or could match) is also keyed on
  // the native live-state epoch, which every such state change bumps. A computation that read
  // something outside the key is volatile and never reused: a rule over view-transition or
  // interest state that could match, an animated element, the fullscreen element
  // (viewport-sized), an @import's media query.
  var computedStyleCache=new computedSnapshotMap(),computedVolatile=false,computedStateRead=false;
  function invalidateComputedStyles(){bumpCssomStructure();}
  __ns.invalidateComputedStyles=invalidateComputedStyles;
  // What a CSS animation / transition update reads beyond the element's own values
  // (animations.js cssUpdate): null when the element has no stamp (it is then never skipped).
  // An animation starting or ending on a target also drops reuse (the targets' values change)
  // without changing any cascaded value; the stamp discounts this realm's such bumps (another
  // realm's only cause an extra update, never a missed one).
  var animatedStyleBumps=0;
  // `from` / `to`: the targets an animation left and joined (turbo-dom#279). Their style
  // generations move (not the tree version, so no other element's animation stamp does);
  // a target whose owner keys reused styles on the plain tree version drops every reuse.
  __ns.invalidateAnimatedStyles=function(from,to){
    var targets=[];if(from)$c.push(targets,from);if(to)$c.push(targets,to);
    for(var index=0;index<targets.length;index++){var scope=-1;try{scope=computedOwnerScope(targets[index]);}catch(e){}if(scope<0){targets=null;break;}}
    if(targets){try{computedStyleTouch(targets,true);return;}catch(e){}}
    animatedStyleBumps++;bumpCssomEpoch();
  };
  // An element's computed `display` (animations.js `rendered`): resolved through `snapshots`,
  // one map shared by a whole ancestor walk, so each ancestor resolves (or revalidates its
  // reuse) once per walk instead of once per level, and no declaration object is made.
  __ns.computedDisplay=function(el,snapshots){return computedSnapshotValue(computeStyleSnapshot(el,snapshots),'display');};
  __ns.computedSnapshots=function(){return new computedSnapshotMap();};
  __ns.computedStyleStamp=function(el){var key=computedCacheKey(el);return key?[key.full,key.epoch-animatedStyleBumps,styleStateEpoch(),key.owner]:null;};
  // The key's `version` (turbo-dom#279) follows what the owner's sheets can select on
  // (computedOwnerScope): the element's style generation (Tree::style_generation: a mutation
  // of the element or an ancestor; with sibling selectors, of a child list along that chain)
  // when nothing else, otherwise the tree's selector version (Tree::selector_version: inline
  // style writes do not move it), or its plain version when a cascade of the element can read
  // other elements' inline styles or the tree has a shadow root (whose own sheets are not
  // scanned). Outside the plain version the element's own inline style text is keyed too;
  // ancestors' inline styles reach it through the parent snapshots it depends on. `full` (the
  // plain version) is what `rendered` and the animation stamp read.
  function computedCacheKeyInline(el,scope,full){
    var inline=null,version=full;
    if(scope<0)return {inline:inline,version:version};
    try{inline=$c.call(appliedInlineRead,el);}catch(e){return {inline:null,version:full};}
    version=full;
    return {inline:inline,version:version,scope:scope};
  }
  function computedCacheKeyVersions(el){var full,version;try{full=computedStyleVersion(el);version=computedStyleSelectorVersion(el);}catch(e){return null;}if(typeof full!=='number'||typeof version!=='number')return null;return {full:full,version:version};}
  function computedCacheKeyGeneration(version,el,scope){if(scope>=0&&scope<8){var generation;try{generation=computedStyleGeneration(el,scope);}catch(e){}if(typeof generation==='number')return generation;}return version;}
  function computedCacheKey(el){
    var vers=computedCacheKeyVersions(el);if(!vers)return null;
    var owner=null;try{owner=cssApply(computedStyleNativeOwner,el,[]);}catch(e){return null;}
    var scope=computedOwnerScope(el),inl=computedCacheKeyInline(el,scope,vers.full);
    var version=computedCacheKeyGeneration(inl.version,el,inl.scope!==undefined?inl.scope:scope);
    return {__proto__:null,epoch:cssomEpoch(),version:version,full:vers.full,inline:inl.inline,owner:owner};
  }
  // What the key cannot see: the element's top layer and font context.
  function computedCacheProbe(el){
    var connected=false;try{connected=computedConnected(el);}catch(e){}
    var layer='';try{if(connected)layer=computedTopLayer(el);}catch(e){}
    var context=null;try{context=computedFontContext(el);}catch(e){}
    return context?[layer,context[0],context[1],context[2],context[3],context[4]]:[layer];
  }
  function computedProbeEqual(a,b){if(a.length!==b.length)return false;for(var index=0;index<a.length;index++)if(a[index]!==b[index])return false;return true;}
  // Whether a cache entry (or a recomputed key) has `key`'s epoch, versions and owner.
  function computedKeyEqual(entry,key){return !!entry&&entry.epoch===key.epoch&&entry.version===key.version&&entry.inline===key.inline&&entry.owner===key.owner;}
  function computedCacheHit(el,key,snapshots){
    var entry=computedSnapshotGet(computedStyleCache,el);
    if(!computedKeyEqual(entry,key))return null;
    if(entry.state!==null&&entry.state!==styleStateEpoch())return null;
    if(!computedMediaUnchanged(entry.media))return null;
    if(!computedProbeEqual(entry.probe,computedCacheProbe(el)))return null;
    for(var index=0;index<entry.deps.length;index+=2)if(computeStyleSnapshot(entry.deps[index],snapshots)!==entry.deps[index+1])return null;
    computedMarkStable(el,entry);
    return entry.snapshot;
  }
  // The stable-entry fast path. Everything the cache key and the dependency walk read moves the
  // element's tree version (any DOM, inline-override, form or custom state change), its
  // selector version (also an animation joining or leaving an element, which moves its style
  // generation but not the tree version), the CSSOM epoch (sheets, rules, adoption, animation
  // model changes) or the live-state epoch; @media
  // answers and the probe (top layer, font context) are checked as on a full hit. An entry
  // validated at a world stamp (tree and selector versions, CSSOM and state epochs) whose dependencies were
  // all stable entries at the same stamp therefore stays valid while the stamp holds: a read
  // then skips recomputing the key and revalidating the ancestor chain (once per property read
  // and per ancestor, which made testing-library's role / visibility queries quadratic in
  // depth). A dependency on a volatile style (a running transition) never marks it stable.
  function computedWorldStamp(el){var full,selector;try{full=computedStyleVersion(el);selector=computedStyleSelectorVersion(el);}catch(e){return null;}return typeof full==='number'&&typeof selector==='number'?[full,selector,cssomEpoch(),styleStateEpoch()]:null;}
  function computedStampEqual(a,b){return !!a&&!!b&&a[0]===b[0]&&a[1]===b[1]&&a[2]===b[2]&&a[3]===b[3];}
  function computedMarkStable(el,entry){
    var stamp=computedWorldStamp(el);entry.stable=null;if(!stamp)return;
    for(var index=0;index<entry.deps.length;index+=2){var dep=computedSnapshotGet(computedStyleCache,entry.deps[index]);if(!dep||dep.snapshot!==entry.deps[index+1]||!computedStampEqual(dep.stable,computedWorldStamp(entry.deps[index])))return;}
    entry.stable=stamp;
  }
  function computedStableHit(el){
    var entry=computedSnapshotGet(computedStyleCache,el);
    if(!entry||!entry.stable||!computedStampEqual(entry.stable,computedWorldStamp(el)))return null;
    if(!computedMediaUnchanged(entry.media))return null;
    if(!computedProbeEqual(entry.probe,computedCacheProbe(el)))return null;
    return entry.snapshot;
  }
  // turbo-test#227 part 2: the cascaded declarations (rules, UA tables, inline style; before
  // inheritance, animations and used values) of an element whose computed style is volatile
  // only after them, reused under the same key as a computed style while it holds.
  var computedBaseCache=new computedSnapshotMap();
  function computedBaseHit(el,key){
    var entry=computedSnapshotGet(computedBaseCache,el);
    if(!computedKeyEqual(entry,key))return null;
    if(entry.state!==null&&entry.state!==styleStateEpoch())return null;
    if(!computedMediaUnchanged(entry.media))return null;
    // No top-layer / font-context probe: entering or leaving the top layer moves the key (open
    // attribute, the fullscreen stack bumps the tree), font defaults, parent and root feed no
    // cascaded declaration, and the UA-table flag changes only in a retired document, which
    // has no computed style.
    return entry.base;
  }
  function computedBaseCapture(decl,priorities,uaValues){
    var out={__proto__:null,decl:{__proto__:null},priorities:{__proto__:null},ua:null,media:{__proto__:null},volatile:computedVolatile,stateRead:computedStateRead};
    for(var name in decl)if(typeof decl[name]!=='function'&&name!=='length')out.decl[name]=decl[name];
    for(var name in priorities)out.priorities[name]=priorities[name];
    if(uaValues){out.ua={__proto__:null};for(var name in uaValues)out.ua[name]=uaValues[name];}
    if(computedMediaReads)for(var text in computedMediaReads)out.media[text]=computedMediaReads[text];
    return out;
  }
  function computedBaseRestore(base,decl,priorities){
    for(var name in base.decl)decl[name]=base.decl[name];
    for(var name in base.priorities)priorities[name]=base.priorities[name];
    computedMergeMedia(base.media);if(base.stateRead)computedStateRead=true;
  }
  // Style sharing (turbo-dom#307 part 2, after Blink's): a new element takes the cascaded
  // declarations of a recently cascaded sibling (same parent, unchanged since: the parent's
  // style generation) with the same signature — tag, namespace, class list, id, the values of
  // every attribute the owner's sheets name and its UA-defaults flag — computed from the same
  // rule-entry build. Everything after the cascaded declarations (inheritance, display
  // blockification as a flex / grid item, fonts, animations, used values, computed defaults)
  // still runs for the element itself. A rule that tests more than the signature (a
  // pseudo-class: structural, :has, :hover, :lang, :dir, state; a sibling combinator) must give
  // the element the answer it gave the sibling, and a state rule must not be able to match
  // either of them (the relaxed test). Never shared: an element with an inline style, a form
  // control (UA tables read its attributes and state), an <iframe> (size hints), one in the top
  // layer or a child iframe document, one in a tree whose sheets key on the tree-wide version
  // (#279's fallback: shadow trees, adopted sheets, @import, container queries, escapes; also
  // every cascade that can go volatile), and a sibling whose cascade read live state.
  var styleShares=new computedSnapshotMap(),styleShareRefused={__proto__:null,INPUT:1,BUTTON:1,TEXTAREA:1,SELECT:1,OPTION:1,OPTGROUP:1,IFRAME:1};
  function styleShareProbeCollect(el){
    var out={};
    if(cssApply(computedStyleNativeGetAttribute,el,['style'])!==null)return null;
    out.tag=cssApply(computedStyleNativeTag,el,[]);if(styleShareRefused[out.tag])return null;
    out.parent=cssParentElement(el);if(!out.parent)return null;
    out.scope=computedOwnerScope(el);if(out.scope<0||out.scope>=8)return null;
    out.generation=computedStyleGeneration(out.parent,out.scope);if(typeof out.generation!=='number')return null;
    out.owner=cssApply(computedStyleNativeOwner,el,[]);out.namespace=cssApply(computedStyleNativeNamespace,el,[]);
    out.names=cssApply(computedStyleNativeGetAttribute,el,['class']);out.id=cssApply(computedStyleNativeGetAttribute,el,['id']);
    out.context=computedFontContext(el);
    return out;
  }
  // Returns the styleShares entry for `parent`, rebuilding it when the generation or
  // sheet-build digest changed (same allocation it replaces inline).
  function styleShareFetchRecord(parent, generation, build){
    var record = computedSnapshotGet(styleShares, parent);
    if (!record || record.generation !== generation || record.build !== build) {
      record = { __proto__: null, generation: generation, build: build, signatures: { __proto__: null }, count: 0 };
      computedSnapshotSet(styleShares, parent, record);
    }
    return record;
  }
  function styleShareSignature(el,info){
    var signature=info.tag+'\u0001'+info.namespace+'\u0001'+(info.context&&info.context[4]?1:0)+'\u0001'+info.names+'\u0001'+info.id;
    var attributes=styleShareAttributes(info.owner);
    for(var index=0;index<attributes.length;index++)signature+='\u0001'+cssApply(computedStyleNativeGetAttribute,el,[attributes[index]]);
    return signature;
  }
  function styleShareProbe(el){
    if(styleShareOff&&styleShareOff())return null;
    var info;
    try{info=styleShareProbeCollect(el);}catch(e){return null;}
    if(!info)return null;
    var signature=styleShareSignature(el,info);
    var sheets=computedSheets(el),build=computedSheetEntries(sheets,el);
    var record=styleShareFetchRecord(info.parent,info.generation,build);
    var probe={__proto__:null,record:record,signature:signature,hit:null},entry=record.signatures[signature];
    if(entry&&styleShareAgrees(el,entry.tests)){probe.hit=entry.base;if(noteStyleShare)noteStyleShare();}
    return probe;
  }
  // The attribute names the owner's sheets name (Tree::set_style_filter's features), once each.
  function styleShareAttributes(owner){
    var entry=computedSnapshotGet(computedSheetLists,owner),features=entry&&entry.features;
    if(!features)return [];
    if(!features.shareNames){var seen={__proto__:null},list=[];for(var index=0;index<features.attributes.length;index++){var name=features.attributes[index];if(!seen[name]){seen[name]=true;$c.push(list,name);}}features.shareNames=list;}
    return features.shareNames;
  }
  function styleShareAgrees(el,tests){for(var index=0;index<tests.length;index+=2)if(computedNativeMatches(el,tests[index])!==tests[index+1])return false;return true;}
  var styleShareRisk=/:|[+~]/;
  function styleShareStore(el,probe,base,index,order){
    var record=probe.record,tests=[];
    if(order)for(var at=0;at<order.length;at++){
      var entry=index.entries[order[at]];if(entry.scopes)return;
      var text=String(entry.rule.selectorText||'');if(!cssApply(cssZoomExec,styleShareRisk,[text]))continue;
      $c.push(tests,text);$c.push(tests,computedNativeMatches(el,text));
      if(cssApply(cssZoomExec,computedStatePattern,[text])){var relaxed=computedRelaxSelector(text);$c.push(tests,relaxed);$c.push(tests,computedNativeMatches(el,relaxed));}
    }
    if(record.count>=32){record.signatures={__proto__:null};record.count=0;}
    if(!record.signatures[probe.signature])record.count++;
    record.signatures[probe.signature]={__proto__:null,base:base,tests:tests};
  }
  function computeStyleSnapshotRun(el,snapshots,sink,pseudo,key){
    var before=key?computedWorldStamp(el):null,deps=[],outer=computedVolatile,outerState=computedStateRead,outerMedia=computedMediaReads,media={__proto__:null},snapshot,volatile,stateRead,state=styleStateEpoch();
    computedVolatile=false;computedStateRead=false;computedMediaReads=media;
    var base={__proto__:null,use:key?computedBaseHit(el,key):null,out:null};
    try{snapshot=computeStyleFresh(el,snapshots,sink,pseudo,deps,base);volatile=computedVolatile;stateRead=computedStateRead;}
    finally{computedVolatile=outer;computedStateRead=outerState;computedMediaReads=outerMedia;if(outerMedia)for(var mediaText in media)if(outerMedia[mediaText]===undefined)outerMedia[mediaText]=media[mediaText];}
    return {snapshot:snapshot,volatile:volatile,stateRead:stateRead,state:state,media:media,deps:deps,base:base,before:before};
  }
  function computeStyleSnapshotCached(el,snapshots){
    var local=computedSnapshotGet(snapshots,el);if(local)return local;
    if(noteStyleLookup)noteStyleLookup();
    var stable=computedStableHit(el);if(stable){computedSnapshotSet(snapshots,el,stable);return stable;}
    var key=computedCacheKey(el);
    if(key){var hit=computedCacheHit(el,key,snapshots);if(hit){computedSnapshotSet(snapshots,el,hit);return hit;}}
    return null;
  }
  function computeStyleSnapshot(el,snapshots,sink,pseudo){
    if(pseudo||sink||!el||typeof el!=='object')return computeStyleFresh(el,snapshots,sink,pseudo,null);
    var cached=computeStyleSnapshotCached(el,snapshots);if(cached)return cached;
    var key=computedCacheKey(el);
    var ctx=computeStyleSnapshotRun(el,snapshots,sink,pseudo,key);
    if(key)computeStyleSnapshotStore(el,key,ctx.state,ctx.stateRead,ctx.media,ctx.deps,ctx.base,ctx.snapshot,ctx.volatile,ctx.before);
    return ctx.snapshot;
  }
  function computeStyleSnapshotStoreVolatile(el,key,state,base,unchanged){
    computedSnapshotSet(computedStyleCache,el,null);
    var out=base.out;
    if(out&&unchanged&&!out.volatile)computedSnapshotSet(computedBaseCache,el,{__proto__:null,base:out,epoch:key.epoch,version:key.version,inline:key.inline,owner:key.owner,state:out.stateRead?state:null,media:out.media});
    else if(!base.use)computedSnapshotSet(computedBaseCache,el,null);
  }
  function computeStyleSnapshotStore(el,key,state,stateRead,media,deps,base,snapshot,volatile,before){
    var after=computedCacheKey(el);
    var unchanged=computedKeyEqual(after,key)&&!(stateRead&&styleStateEpoch()!==state);
    if(volatile||!unchanged){computeStyleSnapshotStoreVolatile(el,key,state,base,unchanged);return;}
    var stored={__proto__:null,snapshot:snapshot,epoch:key.epoch,version:key.version,inline:key.inline,owner:key.owner,state:stateRead?state:null,media:media,probe:computedCacheProbe(el),deps:deps,stable:null};
    computedSnapshotSet(computedStyleCache,el,stored);
    if(computedStampEqual(before,computedWorldStamp(el)))computedMarkStable(el,stored);
  }
  // Selector-dependent state: the pseudo-classes whose truth is live binding state (form
  // control state, open popovers, modal dialogs, active view transitions, interest invokers)
  // rather than the tree. A rule using one makes the computation state-dependent (keyed on the
  // live-state epoch; volatile for view-transition and interest state, which bump no epoch)
  // when it matches, or when it could match in another state: its
  // selector with every such pseudo-class (and every non-matching-list function containing one)
  // relaxed to `:is(*)` matches. Relaxing only widens a selector, so a rule that cannot match
  // even relaxed cannot match in any state.
  var computedStatePattern=/:(?:checked|indeterminate|default|placeholder-shown|read-only|read-write|in-range|out-of-range|valid|invalid|user-valid|user-invalid|enabled|disabled|required|optional|popover-open|modal|open|active-view-transition|active-view-transition-type|interest-source|interest-target)(?![\w-])/i;
  var computedStateName=/^(?:checked|indeterminate|default|placeholder-shown|read-only|read-write|in-range|out-of-range|valid|invalid|user-valid|user-invalid|enabled|disabled|required|optional|popover-open|modal|open|active-view-transition|active-view-transition-type|interest-source|interest-target)$/i;
  var computedListFunction=/^(?:is|where|has|-webkit-any|matches)$/i;
  var computedRelaxed={__proto__:null},computedRelaxedCount=0;
  // The index of the quote closing the string opened at `index` (escapes skipped).
  function relaxStringEnd(text,index,c){var end=index+1;while(end<text.length&&text[end]!==c){if(text[end]==='\\')end++;end++;}return end;}
  // The index after the `)` closing the argument opened before `after + 1`; -1 on an escape.
  function relaxSkipString(text,close,quote){var q=close+1;while(q<text.length&&text[q]!==quote)q++;return q+1;}
  function relaxArgumentClose(text,after){
    var depth=1,close=after+1;
    while(close<text.length&&depth){var d=text[close];if(d==='\\')return -1;if(d==='"'||d==="'"){close=relaxSkipString(text,close,d);continue;}if(d==='(')depth++;else if(d===')')depth--;close++;}
    return close;
  }
  // A functional pseudo-class `:name(inner)` relaxed: states match anything, lists recurse.
  function relaxFunctionalPseudo(text,index,name,inner,close){
    if(cssApply(cssZoomExec,computedStateName,[name]))return ':is(*)';
    if(cssApply(cssZoomExec,computedListFunction,[name]))return ':'+name+'('+computedRelaxSelector(inner)+')';
    return cssApply(cssZoomExec,computedStatePattern,[inner])?':is(*)':$c.slice(text, index,close);
  }
  function relaxNonColonChar(c,text,index,st){
    if(c==='"'||c==="'"){var end=relaxStringEnd(text,index,c);st.out+=$c.slice(text, index,end+1);st.index=end+1;return true;}
    if(c!==':'){st.out+=c;st.index++;return true;}
    if(text[index+1]===':'){st.out+='::';st.index+=2;return true;}
    return false;
  }
  function computedRelaxSelector(text){
    var st={out:'',index:0};
    while(st.index<text.length){
      var c=text[st.index];
      if(c==='\\')return '*';
      if(relaxNonColonChar(c,text,st.index,st))continue;
      var name=cssApply(cssZoomExec,/^[A-Za-z0-9_-]*/,[$c.slice(text, st.index+1)])[0],after=st.index+1+name.length;
      if(text[after]!=='('){st.out+=cssApply(cssZoomExec,computedStateName,[name])?':is(*)':':'+name;st.index=after;continue;}
      var close=relaxArgumentClose(text,after);
      if(close<0)return '*';
      st.out+=relaxFunctionalPseudo(text,st.index,name,$c.slice(text, after+1,close-1),close);
      st.index=close;
    }
    return st.out;
  }
  var computedUnkeyedState=/:(?:active-view-transition|active-view-transition-type|interest-source|interest-target)(?![\w-])/i;
  function computedRelaxedCached(selector){
    var relaxed=computedRelaxed[selector];
    if(relaxed===undefined){if(computedRelaxedCount>=4096){computedRelaxed={__proto__:null};computedRelaxedCount=0;}relaxed=computedRelaxed[selector]=computedRelaxSelector(selector);computedRelaxedCount++;}
    return relaxed;
  }
  function computedStateCouldMatch(el,selector,matched){
    if(matched)return true;
    var relaxed=computedRelaxedCached(selector);
    try{return !!cssApply(computedStyleNativeMatches,el,[relaxed]);}catch(e){return true;}
  }
  function computedStateVolatile(el,selector,matched){
    if(computedVolatile||!cssApply(cssZoomExec,computedStatePattern,[selector]))return;
    var unkeyed=!!cssApply(cssZoomExec,computedUnkeyedState,[selector]);
    if(!unkeyed&&computedStateRead)return;
    if(!computedStateCouldMatch(el,selector,matched))return;
    if(unkeyed)computedVolatile=true;else computedStateRead=true;
  }
  // turbo-dom#338: computed values of the tree-counting functions (sibling-index() /
  // sibling-count(): the element's 1-based position among its parent's element children and
  // their count) and of shape(). The cascaded text takes the element's counts; the math
  // functions then resolve the way Chrome 156 computes them (lengths in px against the font,
  // percentages kept, angles in deg, times in s; integers rounded half up, ranges clamped),
  // and shape() resolves its positions and lengths. Chrome 156 control:
  // css_tree_counting_shape_chrome156.json.
  var $Mfloor=Math.floor,$Mmax=Math.max,$Mabs=Math.abs,treeCountingPattern=/sibling-(?:index|count)\(\s*\)/i,treeSiblings=null;
  function treeCounts(el){
    if(!treeSiblings)treeSiblings=[__ns.domAccessor('parentNode',false,'Node'),__ns.domAccessor('firstChild',false,'Node'),__ns.domAccessor('nextSibling',false,'Node'),__ns.domAccessor('nodeType',false,'Node')];
    var parent=cssApply(treeSiblings[0],el,[]),index=1,count=0;
    if(!parent)return [1,1];
    for(var node=cssApply(treeSiblings[1],parent,[]);node;node=cssApply(treeSiblings[2],node,[])){
      if(cssApply(treeSiblings[3],node,[])!==1)continue;
      count++;if(node===el)index=count;
    }
    return [index,count];
  }
  // A color longhand that now holds a sibling-count substitution reparses to its canonical
  // spelling (`cssShorthand('longhand', …)` validates and serializes); non-color properties
  // pass through unchanged.
  function treeCountingApplyColor(name, value){
    if (!cssShorthand || !$c.test(/color$/i, name)) return value;
    var spelled = cssShorthand('longhand', cssPropertyName(name), value);
    return spelled[0] === 'valid' ? spelled[1] : value;
  }
  // The declarations holding a tree-counting function, with the element's counts in place.
  function treeCountingNeedsResolve(name,value){return typeof value==='string'&&$c.slice(name,0,2)!=='--'&&$c.test(treeCountingPattern,value);}
  function treeCountingReplace(value,counts){return $c.replace($c.replace(value,/sibling-index\(\s*\)/gi,String(counts[0])),/sibling-count\(\s*\)/gi,String(counts[1]));}
  function treeCountingValues(decl,el,pseudo){
    var names=null,counts=null;
    if(pseudo||!el)return null;
    for(var name in decl){
      if(!treeCountingNeedsResolve(name,decl[name]))continue;
      if(!counts)counts=treeCounts(el);
      var value=treeCountingReplace(decl[name],counts);
      value=treeCountingApplyColor(name,value);
      decl[name]=value;
      if($c.toLowerCase(name)===name){if(!names)names=[];$c.push(names,name);}
    }
    if(names)computedVolatile=true;
    return names;
  }
  // A math value: like-unit coefficients ('' number, px, %, deg, s).
  var mathUnits={__proto__:null,px:['px',1],pt:['px',96/72],pc:['px',16],in:['px',96],cm:['px',96/2.54],mm:['px',96/25.4],q:['px',96/101.6],deg:['deg',1],grad:['deg',0.9],rad:['deg',180/Math.PI],turn:['deg',360],s:['s',1],ms:['s',0.001],'%':['%',1],'':['',1]};
  var mathConstants={__proto__:null,e:Math.E,pi:Math.PI,infinity:Infinity,'-infinity':-Infinity,nan:NaN};
  function mathTerm(token,font){
    var match=$c.exec(/^([+-]?(?:\d*\.\d+|\d+)(?:e[+-]?\d+)?)([a-z]*|%)$/i,token);if(!match)return null;
    var number=cssFontNumber(match[1]),unit=$c.toLowerCase(match[2]),out={__proto__:null};
    if(unit==='em'||unit==='rem'){var base=font&&(unit==='em'?font.size:font.rootSize);if(typeof base!=='number')return null;out.px=number*base;return out;}
    var scale=mathUnits[unit];if(!scale)return null;
    out[scale[0]]=number*scale[1];return out;
  }
  function mathUnitOf(value){var keys=$O.keys(value);return keys.length===1?keys[0]:null;}
  function mathAdd(a,b,sign){var out={__proto__:null};for(var unit in a)out[unit]=a[unit];for(unit in b)out[unit]=(out[unit]||0)+sign*b[unit];return out;}
  function mathScale(a,factor){var out={__proto__:null};for(var unit in a)out[unit]=a[unit]*factor;return out;}
  // One operand: a literal, a constant, a parenthesized sum or a math function.
  function mathOperand(state){
    var token=state.tokens[state.at++];
    if(token==='('){var inner=mathSum(state);state.at++;return inner;}
    if(token&&token[token.length-1]==='(')return mathFunction($c.toLowerCase($c.slice(token,0,-1)),state);
    var constant=token&&mathConstants[$c.toLowerCase(token)];
    if(constant!==undefined){var out={__proto__:null};out['']=constant;return out;}
    return token?mathTerm(token,state.font):null;
  }
  function mathProduct(state){
    var value=mathOperand(state);
    while(value&&(state.tokens[state.at]==='*'||state.tokens[state.at]==='/')){
      var operator=state.tokens[state.at++],right=mathOperand(state);if(!right)return null;
      if(operator==='/'){if(mathUnitOf(right)!=='')return null;value=mathScale(value,1/right['']);}
      else if(mathUnitOf(right)==='')value=mathScale(value,right['']);
      else if(mathUnitOf(value)==='')value=mathScale(right,value['']);
      else return null;
    }
    return value;
  }
  function mathSum(state){
    var value=mathProduct(state);
    while(value&&(state.tokens[state.at]==='+'||state.tokens[state.at]==='-')){var sign=state.tokens[state.at++]==='-'?-1:1,right=mathProduct(state);value=right&&mathAdd(value,right,sign);}
    return value;
  }
  // The comma-separated arguments of a math function, up to its `)` (`none`: null).
  function mathArguments(state){
    var list=[];
    do{if(state.tokens[state.at]==='none'){state.at++;$c.push(list,null);}else $c.push(list,mathSum(state));}
    while(state.tokens[state.at++]===',');
    return list;
  }
  // calc() of a sum; min() / max() / clamp() / abs() / sign() / round() of one-unit arguments.
  function mathFunctionCollectValues(list){
    var unit=null,values=[];
    for(var index=0;index<list.length;index++){
      if(list[index]===null){$c.push(values,null);continue;}
      var own=list[index]&&mathUnitOf(list[index]);
      if(own===null||unit!==null&&own!==unit)return null;
      unit=own;$c.push(values,list[index][own]);
    }
    return {unit:unit,values:values};
  }
  function mathFunction(name,state){
    var list=mathArguments(state);
    if(name==='calc')return list.length===1?list[0]:null;
    var collected=mathFunctionCollectValues(list);if(!collected)return null;
    var number=mathResolve(name,collected.values);
    if(number===undefined)return null;
    var out={__proto__:null};out[name==='sign'?'':collected.unit]=number;return out;
  }
  var mathResolvers={__proto__:null,
    'min':function(v){return cssApply($Mmin,null,v);},
    'max':function(v){return cssApply($Mmax,null,v);},
    'clamp':function(v){return $Mmax(v[0]===null?-Infinity:v[0],$Mmin(v[1],v[2]===null?Infinity:v[2]));},
    'abs':function(v){return $Mabs(v[0]);},
    'sign':function(v){return v[0]>0?1:v[0]<0?-1:v[0];},
    'round':function(v){var b=v[1]===undefined?1:v[1];return $Mfloor(v[0]/b+0.5)*b;}
  };
  function mathResolve(name,values){
    var resolver=mathResolvers[name];
    return resolver?resolver(values):undefined;
  }
  // The value of one math function's text, or null when it does not resolve here.
  function mathValue(text,font){
    var tokens=[],pattern=/\s+|[+-]?(?:\d*\.\d+|\d+)(?:e[+-]?\d+)?(?:[a-z]+|%)?|-?[a-z][a-z-]*\(?|[()+*\/,-]/gi,match,at=0;
    pattern.lastIndex=0;
    while((match=$c.exec(pattern,text))!==null){if(match.index!==at)return null;at=pattern.lastIndex;if(!$c.test(/^\s/,match[0]))$c.push(tokens,match[0]);}
    if(at!==text.length)return null;
    var state={tokens:tokens,at:0,font:font},value=mathOperand(state);
    return state.at===tokens.length?value:null;
  }
  function mathNumberText(number){return cssFontString(cssFontNumber(cssApply(cssFontPrecision,number,[6])));}
  // Chrome's computed spelling: one unit, or a percentage and a length as calc().
  function mathText(value){
    var units=$O.keys(value);
    if(units.length===1)return mathNumberText(value[units[0]])+units[0];
    if(units.length!==2||value['%']===undefined||value.px===undefined)return null;
    var px=value.px;return 'calc('+mathNumberText(value['%'])+'% '+(px<0?'- ':'+ ')+mathNumberText($Mabs(px))+'px)';
  }
  // Properties whose number rounds to an integer, whose value is clamped to a range.
  var mathIntegers={__proto__:null,order:1,'z-index':1,'column-count':1,'orphans':1,'widows':1},mathRanges={__proto__:null,opacity:[0,1],'fill-opacity':[0,1],'stroke-opacity':[0,1],'flood-opacity':[0,1],'stop-opacity':[0,1],'font-weight':[1,1000],'flex-grow':[0,Infinity],'flex-shrink':[0,Infinity]};
  var mathNonNegative=/^(?:(?:min-|max-)?(?:width|height|block-size|inline-size)|padding(?:-.*)?|border-.*-width|outline-width|column-gap|row-gap|gap|font-size)$/;
  // A whole math value of `name` (a number or one length / percentage, clamped and rounded).
  function mathPropertyClamp(name,value,unit,number){
    var range=mathRanges[name];
    if(range)value[unit]=$Mmin($Mmax(number,range[0]),range[1]);
    if(mathIntegers[name]&&unit==='')value['']=$Mfloor((range?value[unit]:number)+0.5);
    if(number<0&&$c.test(mathNonNegative,name))value[unit]=0;
  }
  function mathPropertyText(name,value){
    var unit=mathUnitOf(value);
    if(unit==='%'&&mathRanges[name]){value=mathScale(value,0.01);value['']=value['%'];delete value['%'];unit='';}
    var number=unit===null?null:value[unit];
    if(number!==null)mathPropertyClamp(name,value,unit,number);
    return mathText(value);
  }
  // Every math function in `text` resolved (`whole`: the value is that one function, for `name`).
  function mathResolveText(name,text,font){
    var whole=mathValue(text,font);if(whole)return mathPropertyText(name,whole)||text;
    return $c.replace(text,/(?:calc|min|max|clamp|abs|sign|round)\((?:[^()]|\((?:[^()]|\([^()]*\))*\))*\)/gi,function(found){var value=mathValue(found,font),out=value&&mathText(value);return out||found;});
  }
  // shape() with its positions as percentages / lengths, lengths in px and angles in deg.
  var shapeSides={__proto__:null,left:'0%',top:'0%','x-start':'0%','y-start':'0%',center:'50%',right:'100%',bottom:'100%','x-end':'100%','y-end':'100%'};
  var shapeWords=/^(?:from|to|by|with|of|cw|large|rotate|evenodd|close|move|line|hline|vline|curve|smooth|arc|\/|start|end|origin)$/;
  function shapeLength(token,font){var value=mathValue(token,font),text=value&&mathText(value);return text||token;}
  // A right / bottom offset counts from the far side.
  function shapeSide(side,offset,font){
    if(side==='left'||side==='top')return shapeLength(offset,font);
    var value=mathValue(offset,font),start={__proto__:null};start['%']=100;
    return value&&mathText(mathAdd(start,value,-1))||offset;
  }
  function shapePosition(group,font){
    var point=group.length===4?[shapeSide(group[0],group[1],font),shapeSide(group[2],group[3],font)]:group;
    return $c.map(point,function(token){return shapeSides[token]||shapeLength(token,font);});
  }
  // One command's tokens (top-level words) with its points, lengths and angles computed.
  function shapeCommand(words,font){
    var out=[],group=[],context='',name=words[0];
    function flush(){
      if(!group.length)return;
      var computed=context==='rotate'||context==='of'||context==='by'?$c.map(group,function(token){return shapeLength(token,font);}):(name==='hline'||name==='vline')?[shapeSides[group[0]]||shapeLength(group[0],font)]:shapePosition(group,font);
      $c.forEach(computed,function(token){$c.push(out,token);});group=[];
    }
    for(var index=0;index<words.length;index++){
      var word=words[index];
      if($c.test(shapeWords,word)){flush();$c.push(out,word);context=word;}else $c.push(group,word);
    }
    flush();
    return $c.join(out,' ');
  }
  // Split `text` at top-level `separator`s (outside parentheses).
  function shapeSplit(text,separator){
    var parts=[],depth=0,start=0;
    for(var index=0;index<text.length;index++){var c=text[index];if(c==='(')depth++;else if(c===')')depth--;else if(c===separator&&!depth){$c.push(parts,$c.slice(text,start,index));start=index+1;}}
    $c.push(parts,$c.slice(text,start));
    return $c.filter($c.map(parts,function(part){return $c.trim(part);}),function(part){return part!=='';});
  }
  function shapeComputed(name,value,font){
    var spelled=cssShorthand?cssShorthand('longhand',name,value):null;
    if(!spelled||spelled[0]!=='valid'||!$c.startsWith(spelled[1],'shape('))return null;
    var text=spelled[1],end=$c.lastIndexOf(text,')'),commands=shapeSplit($c.slice(text,6,end),',');
    return 'shape('+$c.join($c.map(commands,function(command){return shapeCommand(shapeSplit(command,' '),font);}),', ')+')'+$c.slice(text,end+1);
  }
  // The math functions and shape()s of a computed style, once the font is known.
  function computedMathValues(decl,names,font,connected,set){
    if(!connected)return;
    if(names)for(var index=0;index<names.length;index++){var name=names[index],value=decl[name];if(typeof value==='string'&&!$c.test(/shape\(/i,value))set(name,mathResolveText(name,value,font));}
    $c.forEach(['clip-path','offset-path','shape-outside'],function(name){var value=decl[name];if(typeof value==='string'&&$c.test(/shape\(/i,value))set(name,shapeComputed(name,value,font)||value);});
  }
  // One computeStyleFresh read's state, shared by its phases below.
  function ComputedCascade(el,snapshots,sink,pseudo,deps,base,inFrame){
    this.el=el;this.snapshots=snapshots;this.sink=sink;this.pseudo=pseudo;this.deps=deps;this.base=base;this.inFrame=inFrame;
    this.decl=null;this.priorities=null;this.uaValues=null;this.expanding=null;this.info=null;this.layer='';
    this.parentStyle=null;this.setProp=null;this.bucketIndex=undefined;this.bucketOrder=undefined;
    this.inheritParent=null;this.inheritSnapshot=null;this.inheritedValue=null;this.connected=false;
  }
  function computedCamel(p){ return $c.replace(p,/-([a-z])/g, function(_,c){ return $c.toUpperCase(c); }); }
  // Normalize comma-spacing ("a,b" -> "a, b"): minified emotion rules drop the space, but jest-dom
  // toHaveStyle's expected value is normalized by the browser's CSS parser (which inserts ", "),
  // so font-family / shorthand lists must match that form.
  // Normalize authored hex colors to the computed rgb()/rgba() spelling used by the
  // declaration records and style assertions.
  function computedNormColor(v){
    var m = $c.exec(/^#([0-9a-fA-F]{3})$/, v); if (m){ var x=m[1]; return 'rgb('+parseInt(x[0]+x[0],16)+', '+parseInt(x[1]+x[1],16)+', '+parseInt(x[2]+x[2],16)+')'; }
    m = $c.exec(/^#([0-9a-fA-F]{6})$/, v); if (m){ var h=m[1]; return 'rgb('+parseInt($c.slice(h, 0,2),16)+', '+parseInt($c.slice(h, 2,4),16)+', '+parseInt($c.slice(h, 4,6),16)+')'; }
    m = $c.exec(/^#([0-9a-fA-F]{8})$/, v); if (m){ var h2=m[1]; return 'rgba('+parseInt($c.slice(h2, 0,2),16)+', '+parseInt($c.slice(h2, 2,4),16)+', '+parseInt($c.slice(h2, 4,6),16)+', '+($Mround(parseInt($c.slice(h2, 6,8),16)/255*100)/100)+')'; }
    return v;
  }
  // Length-valued properties resolve a bare zero to the computed "0px" spelling.
  var computedLenProp = /(width|height|^top$|^right$|^bottom$|^left$|^inset|margin|padding|gap|^flex-basis$|radius|^font-size$|letter-spacing|word-spacing|text-indent|^outline-offset$|^column-(width|gap)$|size$)/;
  // A cascaded value as stored: `revert` takes the UA value, hex colors / bare zeros / lists
  // take their computed spelling.
  function cascadeSpelledNormalize(name,val){var t=$c.trim(val);if(t[0]==='#')return computedNormColor(t);if(t==='0'&&$c.test(computedLenProp,name))return '0px';if($c.indexOf(val,',')>=0)return cssPrelude(val,true);return val;}
  function cascadeSpelledValue(cs,name,val){
    if(cs.uaValues&&typeof val==='string'&&$c.test(/^revert$/i,$c.trim(val))&&cs.uaValues[name]!==undefined)val=cs.uaValues[name];
    if (typeof val === 'string' && $c.slice(name,0, 2) !== '--') val = cascadeSpelledNormalize(name,val);
    return val;
  }
  // Priority is private per computed snapshot (cs.priorities); public CSSStyleDeclaration
  // methods and page overrides cannot change the importance of parsed declarations. A
  // cascaded longhand outdates the shorthand entries an earlier declaration (a UA table)
  // stored; the shorthand being expanded (cs.expanding) keeps its own.
  function cascadeSetProp(cs,name,val,important){
    name = $c.trim(String(name)); if (!name) return;
    if(important!==undefined){if(cs.priorities[name]&&!important)return;cs.priorities[name]=important;}
    val=cascadeSpelledValue(cs,name,val);
    var decl=cs.decl,sink=cs.sink;
    decl[name] = val; decl[computedCamel(name)] = val;
    $c.forEach(staleShorthands(name),function(shorthand){ if (shorthand !== cs.expanding) { delete decl[shorthand]; delete decl[computedCamel(shorthand)]; } });
    if (sink) sink.order[name] = ++sink.count;
    if (typeof val === 'string') { var outer = cs.expanding; cs.expanding = name; try { expandShorthand(name, val, function(property,value){cascadeSetProp(cs,property,value,important);}); } finally { cs.expanding = outer; } }
  }
  // `only`: true applies just the !important declarations, false just the normal ones.
  function cascadeApplyDeclarations(cs,record,accepts,only){for(var index=0;index<record.names.length;index++){var name=record.names[index],value=cssMapGet(record.values,name);if(accepts&&$c.slice(name,0,2)!=='--'&&!accepts(name))continue;if(only!==undefined&&only!==(value.priority==='important'))continue;cascadeSetProp(cs,name,value.pending?value.pending[1]:value.raw||value.text,value.priority==='important');}}
  // A disconnected element has no style (turbo-dom#67): every value reads "" until it is
  // connected again (the declaration stays live). Typed OM's cascade read (`sink`) still
  // reports the cascaded values. A pseudo-element of a disconnected (or <video>/<audio>
  // child) element has no style. Null when the element has a style.
  function computedStyleUnavailableDisconnected(el,snapshots,inFrame){
    var attached=true;try{attached=inFrame||__ns.domAccessor('isConnected',false,'Node').call(el);}catch(e){}
    if(!attached){var empty=__ns.makeComputedStyleDeclaration({},el,null);computedSnapshotSet(snapshots,el,empty);return empty;}
    return null;
  }
  function computedStyleUnavailablePseudo(el,pseudo){
    var live=false;try{live=computedConnected(el)&&!computedMediaChild(el);}catch(e){}
    return live?null:__ns.makeComputedStyleDeclaration({},el,null,pseudo);
  }
  function computedStyleUnavailable(el,snapshots,sink,pseudo,inFrame){
    if(!pseudo&&!sink&&el){var disc=computedStyleUnavailableDisconnected(el,snapshots,inFrame);if(disc)return disc;}
    if(pseudo)return computedStyleUnavailablePseudo(el,pseudo);
    return null;
  }
  function computedPseudoInfoFor(el,pseudo,parentStyle){return pseudo&&pseudo!=='backdrop'&&pseudo!=='before'&&pseudo!=='after'?computedPseudoInfo(el,pseudo,parentStyle):null;}
  function computedLayerOf(el,pseudo,inFrame){
    var layer='';
    try { if(pseudo||inFrame||cssApply(computedStyleNativeConnected,el,[]))layer=computedTopLayer(el); } catch(e){}
    return layer;
  }
  // Measured Chrome button defaults are UA declarations: every matching author
  // rule (including a universal selector) and inline value takes precedence.
  function computedBaseUserAgentHtml(cs){
    var el=cs.el;
    if(cssApply(computedStyleNativeNamespace,el,[])==='http://www.w3.org/1999/xhtml'&&(cs.inFrame||cssApply(computedStyleNativeConnected,el,[]))){var fontContext=computedFontContext(el);if(fontContext&&fontContext[4])return htmlUserAgent(el,cssApply(computedStyleNativeTag,el,[]));}
    return null;
  }
  function computedBaseUserAgent(cs){
    var pseudo=cs.pseudo,layer=cs.layer;
    if(pseudo==='backdrop')return backdropUserAgent(layer);
    if(cs.info)return cs.info.ua;
    if(pseudo)return null;
    if(layer==='modal')return modalDialogUserAgent;
    return computedBaseUserAgentHtml(cs);
  }
  function computedUserAgentDecls(cs){
    var ua=null;
    try { ua=computedBaseUserAgent(cs);
      if(!cs.pseudo&&cs.layer==='fullscreen')ua=$O.assign({},ua||{},fullscreenUserAgent); } catch(e){}
    return ua;
  }
  function computedBucketMatches(el,sheets,bucketIndex,bucketOrder,pseudo,matched){
    for(var bucketAt=0;bucketAt<bucketOrder.length;bucketAt++){var bucketEntry=bucketOrder[bucketAt];if(sheetRecord(sheets[bucketIndex.sheet[bucketEntry]]).disabled)continue;var bucketHit=appliedEntryMatch(el,bucketIndex.entries[bucketEntry],pseudo,matched.length);if(bucketHit)$c.push(matched,bucketHit);}
  }
  function computedSheetMatches(el,sheets,build,pseudo,matched){
    for (var si=0; si<sheets.length; si++) {
      if(sheetRecord(sheets[si]).disabled)continue;
      var entries = build.lists[si];
      for (var ri=0; ri<entries.length; ri++) { var record = appliedEntryMatch(el,entries[ri],pseudo,matched.length); if (record) $c.push(matched,record); }
    }
  }
  // Minimal cascade: merge declarations from every injected stylesheet rule whose selector matches
  // `el` (emotion's `sx` -> `.css-xxx{...}`), retaining per-longhand importance through
  // shorthand expansion.
  function computedApplySheets(cs){
    var el=cs.el,info=cs.info;
    try {
      // Element-backed pseudo-elements the element does not have see no author rules.
      // Match the private selector list before touching declarations. Native parsing
      // preserves quoted delimiters that serialized text splitting loses.
      var sheets = info && !info.authored ? [] : computedSheets(el),build=computedSheetEntries(sheets,el),layers=build.layers,matched=[];
      var bucketOwn=sheets.length?cssElementKeys(el):null;
      cs.bucketIndex=bucketOwn?cssBucketIndex(build):null;
      cs.bucketOrder=cs.bucketIndex?cssBucketCandidates(cs.bucketIndex,bucketOwn):null;
      if(cs.bucketOrder)computedBucketMatches(el,sheets,cs.bucketIndex,cs.bucketOrder,cs.pseudo,matched);
      else computedSheetMatches(el,sheets,build,cs.pseudo,matched);
      cascadeEntries(matched,layers,function(entry,only){cascadeApplyDeclarations(cs,declarationRecord(entry.rule.style),info&&info.accepts,only);});
    } catch(e){}
  }
  // UA declarations, presentational hints, author rules, then inline declarations (own props
  // React/components set: background, height, display, ...) and the !important UA ones
  // (::placeholder pointer-events, the fullscreen element's box), which beat every author one.
  function computedApplyUa(cs,decl){var ua=computedUserAgentDecls(cs);if(!ua)return;$c.forEach($O.keys(ua),function(name){cs.setProp(name,ua[name]);});cs.uaValues=$O.create(null);$c.forEach($O.keys(decl),function(name){if(typeof decl[name]==='string')cs.uaValues[name]=decl[name];});}
  function computedImportantDecls(cs){return cs.info?cs.info.important:!cs.pseudo&&cs.layer==='fullscreen'?fullscreenImportant:null;}
  function computedApplyCascade(cs){
    computedApplyUa(cs,cs.decl);
    var sizeHints=!cs.pseudo&&iframeSizeHints(cs.el);
    if(sizeHints)$c.forEach($O.keys(sizeHints),function(name){cs.setProp(name,sizeHints[name]);});
    computedApplySheets(cs);
    var st = !cs.pseudo && cs.el && __ns.makeInlineStyleDeclaration(cs.el);
    if (st) cascadeApplyDeclarations(cs,declarationRecord(st));
    var important = computedImportantDecls(cs);
    if (important) $c.forEach($O.keys(important), function(name){ cs.setProp(name, important[name], true); });
  }
  function computedShareProbe(cs,reused){var base=cs.base;return !reused&&base&&!cs.pseudo&&!cs.sink&&!cs.inFrame&&cs.layer===''?styleShareProbe(cs.el):null;}
  // The cascaded declarations come from this element's earlier computation (turbo-test#227)
  // or a same-signature sibling's (turbo-dom#307), else from the rules.
  function computedCascadeDeclsStore(cs,base,share){if(base&&!cs.pseudo&&!cs.sink){base.out=computedBaseCapture(cs.decl,cs.priorities,cs.uaValues);if(share&&!base.out.stateRead)styleShareStore(cs.el,share,base.out,cs.bucketIndex,cs.bucketOrder);}}
  function computedCascadeDecls(cs){
    var base=cs.base,reused=base?base.use:null,share=computedShareProbe(cs,reused);
    if(share&&share.hit)reused=share.hit;
    if(reused){computedBaseRestore(reused,cs.decl,cs.priorities);cs.uaValues=reused.ua;return;}
    computedApplyCascade(cs);
    computedCascadeDeclsStore(cs,base,share);
  }
  function computedSinkValues(decl){ var cascaded = { __proto__: null }; $c.forEach($O.keys(decl),function(name){ if (typeof decl[name] === 'string' && ($c.slice(name,0, 2) === '--' || name === $c.toLowerCase(name))) cascaded[name] = decl[name]; }); return cascaded; }
  // Inherited values come from the composed parent's snapshot in this read, so
  // CSSOM edits to document sheets reach descendants. Native cascade only adds
  // shadow-scoped rules (shadow <style>, :host, ::slotted) the JS cascade lacks.
  function computedInheritance(cs){
    var el=cs.el,pseudo=cs.pseudo,info=cs.info;
    try { var inheritContext = computedFontContext(el); cs.inheritParent = pseudo ? el : inheritContext ? inheritContext[2] : null; } catch(e) {}
    // A fresh internal result is already resolved. Reading its public live getter
    // would compute the same ancestors again and double recursion at every level.
    // A view-transition pseudo-element inherits from its parent in the pseudo-element tree.
    cs.inheritSnapshot = info && info.parentPseudo ? computeStyleSnapshot(el, cs.snapshots, undefined, info.parentPseudo) : null;
    cs.inheritedValue = function(name){ return cs.inheritSnapshot ? computedSnapshotValue(cs.inheritSnapshot,name) : cs.inheritParent ? computedSnapshotValue(cs.parentStyle(cs.inheritParent),name) : ''; };
  }
  // The element's own display as the UA / initial resolution below will compute it ("" for
  // a value it cannot tell before then), and this read's snapshot map: a transition update
  // decides "rendered" from them without resolving the element or its ancestors again
  // (turbo-test#227).
  function computedOwnDisplayCascade(el,value){if((value==null||value==='')&&typeof __ns.cascadeProp==='function'){try{return __ns.cascadeProp.call(el,'display')||'';}catch(e){return '';}}return value;}
  function computedOwnDisplayResolve(el,value,layer){if(!value||value==='revert'||value==='revert-layer'){try{return computedUserAgentBox(el,layer).display;}catch(e){return '';}}return $c.test(/^[a-z-]+(?: [a-z-]+)*$/,value)?value:'';}
  function computedOwnDisplay(cs){
    var value=computedOwnDisplayCascade(cs.el,cs.decl.display);
    value=$c.toLowerCase($c.trim(String(value==null?'':value)));
    return computedOwnDisplayResolve(cs.el,value,cs.layer);
  }
  // Web Animations compose over cascaded and inline values; !important declarations win.
  function computedAnimatedEntries(cs){var el=cs.el,pseudo=cs.pseudo,decl=cs.decl;if(pseudo||!el||!__ns.animatedStyle)return null;return __ns.animatedStyle(el,function(name){var value=decl[name];return value==null?'':String(value);},cs.inheritedValue,cs.snapshots,function(){return computedOwnDisplay(cs);});}
  function computedApplyAnimations(cs){
    if(!cs.pseudo&&cs.el&&__ns.hasAnimations&&__ns.hasAnimations(cs.el))computedVolatile=true;
    var animated=computedAnimatedEntries(cs);
    if(animated)for(var ai=0;ai<animated.length;ai++)if(!cs.priorities[animated[ai][0]])cs.setProp(animated[ai][0],animated[ai][1]);
  }
  function highlightInheritsKey(value){ return !value || $c.test(/^(inherit|unset)$/i, $c.trim(String(value))); }
  // A highlight in highlight-inheritance mode inherits color / background-color from the
  // parent's highlight, or takes the initial values (turbo-dom#47).
  function computedHighlightParent(el,pseudo,snapshots){var ctx=null;try{ctx=computedFontContext(el);}catch(e){}var parent=ctx?ctx[2]:null;return parent&&computedHighlightMode(parent,pseudo)?computeStyleSnapshot(parent,snapshots,undefined,pseudo):null;}
  function computedHighlightInherit(cs,color,hp){
    if(highlightInheritsKey(color))color=hp?computedSnapshotValue(hp,'color'):'rgb(0, 0, 0)';
    if(highlightInheritsKey(cs.decl['background-color']))cs.setProp('background-color',hp?computedSnapshotValue(hp,'background-color'):'rgba(0, 0, 0, 0)');
    return color;
  }
  function computedHighlightColor(cs,color){
    var highlightMode=false;try{highlightMode=computedHighlightMode(cs.el,cs.pseudo);}catch(e){}
    if(highlightMode)return computedHighlightInherit(cs,color,computedHighlightParent(cs.el,cs.pseudo,cs.snapshots));
    if(cs.decl['background-color']==null)cs.setProp('background-color','rgba(0, 0, 0, 0)');
    return color;
  }
  function computedColorKeyword(cs,color){
    var colorKeyword = $c.toLowerCase(String(color || ''));
    if (colorKeyword === 'initial' || colorKeyword === 'canvastext') return 'rgb(0, 0, 0)';
    if (!color || colorKeyword === 'inherit' || colorKeyword === 'unset' || colorKeyword === 'currentcolor') return cs.inheritParent ? cs.inheritedValue('color') : 'rgb(0, 0, 0)';
    return color;
  }
  // Children of <video> / <audio> are outside the flat tree: Chrome computes no style for them.
  function computedConnectedForStyle(el,inFrame){
    var connected = false;
    try { connected = inFrame || __ns.domAccessor('isConnected',false,'Node').call(el); } catch(e) {}
    if (connected && computedMediaChild(el)) connected = false;
    return connected;
  }
  function computedColorCascade(cs){var color=cs.decl.color||'';if(!color&&!cs.pseudo&&!cs.inFrame){try{color=__ns.cascadeProp.call(cs.el,'color');}catch(e){}}return color;}
  function computedColor(cs){
    var color=computedColorCascade(cs);
    if (cs.info && cs.info.highlight) color = computedHighlightColor(cs, color);
    color = computedColorKeyword(cs, color);
    cs.connected = computedConnectedForStyle(cs.el, cs.inFrame);
    cs.setProp('color', cs.connected ? normalizeComputedColor(color) : '');
    if (cs.connected) resolveWideKeywords(cs.decl, cs.inheritParent ? $c.get(declarationRecords, cs.parentStyle(cs.inheritParent)) : null, cs.setProp, function(name){ delete cs.decl[name]; delete cs.decl[computedCamel(name)]; });
  }
  function computedCascadedValueNative(cs,name){if(cs.pseudo||cs.inFrame||!cs.el||typeof __ns.cascadeProp!=='function')return null;try{return __ns.cascadeProp.call(cs.el,name)||'';}catch(e){return '';}}
  function computedCascadedValue(cs,name){
    var value=cs.decl[name];
    if(value==null||value===''){var native=computedCascadedValueNative(cs,name);if(native!==null)value=native;}
    return value==null?'':$c.trim(String(value));
  }
  function computedWideKeyword(value){ var key = $c.toLowerCase(value); return $c.test(/^(inherit|initial|unset|revert|revert-layer)$/, key) ? key : ''; }
  // display / position: an author / inline / shadow-scoped declaration wins, the CSS-wide
  // keywords resolve (inherit -> the composed parent's value, initial / unset -> the initial
  // value, revert -> the UA value), otherwise Chrome's html.css value.
  function computedBoxKeywordValue(cs,name,uaValue,fallback){
    var value = computedCascadedValue(cs,name), key = computedWideKeyword(value);
    if (!value || key === 'revert' || key === 'revert-layer') return uaValue;
    if (key === 'inherit') return cs.inheritedValue(name) || fallback;
    if (key) return fallback;
    return value;
  }
  function computedUaBox(cs){
    var pseudo=cs.pseudo,info=cs.info,layer=cs.layer;
    return pseudo === 'backdrop' ? { display: 'block', position: layer ? 'fixed' : 'absolute', topLayer: layer !== '' } : pseudo ? { display: info ? info.display : 'inline', position: info ? info.position : 'static', topLayer: false } : computedUserAgentBox(cs.el, layer);
  }
  function computedInFormattingContainer(cs){
    var pseudo=cs.pseudo,info=cs.info;
    return pseudo ? pseudo !== 'backdrop' && (!info || info.container) && computedPseudoInContainer(cs.el, cs.parentStyle) : computedFlexOrGridItem(cs.el, cs.parentStyle);
  }
  // UA and initial values for display / position / visibility (turbo-surf#36), measured in
  // computed_defaults_chrome156.json; display is then blockified for floats, absolutely
  // positioned elements and flex / grid items. Disconnected elements keep "" like every
  // other default.
  function computedDisplayPosition(cs){
    var decl=cs.decl;
    var ua = computedUaBox(cs);
    var displayValue = computedBoxKeywordValue(cs, 'display', ua.display, 'inline');
    var positionValue = $c.toLowerCase(computedBoxKeywordValue(cs, 'position', ua.position, 'static'));
    if (ua.topLayer) positionValue = topLayerPosition(positionValue);
    var floatValue = $c.toLowerCase($c.trim(String(decl['float'] == null ? '' : decl['float'])));
    var floated = floatValue !== '' && floatValue !== 'none' && !computedWideKeyword(floatValue);
    if (positionValue === 'absolute' || positionValue === 'fixed' || floated || computedInFormattingContainer(cs))
      displayValue = blockifiedDisplay(displayValue);
    decl.display = displayValue;
    decl.position = positionValue;
  }
  // `visibility` is INHERITED: a child with no explicit visibility takes the parent's computed
  // value. testing-library / dom-accessibility-api rely on this — they treat an element as
  // inaccessible when getComputedStyle(el).visibility === 'hidden', expecting a hidden ancestor to
  // have propagated down. The JS cascade above only sees this element's own rules, so we defer to
  // rtdom's native cascade (cascade::computed_style) for shadow-scoped rules; the composed
  // parent's snapshot supplies inheritance and the root's value is the initial `visible`.
  // Read the parent's visibility; '' on any exception from the inherit helper.
  function computedVisibilityInherited(cs){
    try { return cs.inheritedValue('visibility'); } catch(e){ return ''; }
  }
  // `initial` always resolves to `visible`; a connected empty or wide-keyword value
  // inherits from the composed parent; everything else keeps the cascaded value.
  function computedVisibilityResolve(cs, visibilityValue, visibilityKey){
    if (visibilityKey === 'initial') return 'visible';
    if (cs.connected && (visibilityValue === '' || visibilityKey)) return computedVisibilityInherited(cs);
    return visibilityValue;
  }
  function computedVisibility(cs){
    var decl=cs.decl,connected=cs.connected;
    var visibilityValue = connected ? computedCascadedValue(cs,'visibility') : String(decl.visibility == null ? '' : decl.visibility);
    var visibilityKey = computedWideKeyword(visibilityValue);
    visibilityValue = computedVisibilityResolve(cs, visibilityValue, visibilityKey);
    if (!visibilityValue && connected) visibilityValue = 'visible';
    // Highlights never take the element's visibility (turbo-dom#47).
    if (connected && cs.info && cs.info.highlight) visibilityValue = 'visible';
    decl.visibility = visibilityValue || '';
  }
  // `content` computes to normal, and to none on ::before / ::after / ::checkmark /
  // ::picker-icon; strings serialize double-quoted (turbo-dom#47).
  // The generated-content pseudos whose default `content` is `none`.
  function computedContentIsGenerated(pseudo, info){
    return pseudo === 'before' || pseudo === 'after' || info && info.generated;
  }
  function computedContentQuote(contentValue){return $c.replace(contentValue,/'((?:[^'\\\n]|\\[\s\S])*)'/g,function(_,inner){return '"'+$c.replace($c.replace(inner,/\\'/g,"'"),/"/g,'\\"')+'"';});}
  function computedContentResolve(decl,pseudo,info,contentValue,connected){
    if(computedContentIsGenerated(pseudo,info)){decl.content=contentValue===''||$c.test(/^normal$/i,contentValue)?'none':contentValue;}
    else if(pseudo&&contentValue)decl.content=contentValue;
    else if(decl.content==null)decl.content=connected?'normal':'';
  }
  function computedContent(cs){
    var contentValue=cs.decl.content==null?'':$c.trim(String(cs.decl.content));
    if(cs.pseudo&&contentValue)contentValue=computedContentQuote(contentValue);
    computedContentResolve(cs.decl,cs.pseudo,cs.info,contentValue,cs.connected);
  }
  // Resolve `transform` to matrix() / matrix3d() / none per Chrome 156 (the "resolved
  // value" for a used transform). Percentage translates and em need a layout box and the
  // font size; `ns.resolveTransform` returns null to keep the declared text when it cannot
  // resolve them (turbo-dom#353). No-op for pseudo-elements or without an embedder hook.
  function computedTransformResolve(cs, decl){
    if (cs.pseudo || typeof __ns.resolveTransform !== 'function' || typeof decl.transform !== 'string') return;
    try {
      var fontMatch = cssApply(cssZoomExec, /^([0-9.e+-]+)px$/, [String(decl['font-size'] == null ? '' : decl['font-size'])]);
      var resolvedTransform = __ns.resolveTransform(decl.transform, cs.el, fontMatch ? cssFontNumber(fontMatch[1]) : 16);
      if (typeof resolvedTransform === 'string') cs.setProp('transform', resolvedTransform);
    } catch(e){}
  }
  // Initial computed values of properties Web Animations commonly animate.
  function computedTransformValues(cs){
    var decl=cs.decl,connected=cs.connected;
    if (decl.opacity == null) decl.opacity = connected ? '1' : '';
    if (decl.transform == null) { decl.transform = connected ? 'none' : ''; }
    if (connected) computedTransformResolve(cs, decl);
  }
  // A modal dialog's overflow longhands recompose the overflow shorthand (modal UA).
  function computedNormalizeOverflowModal(cs, decl){
    if (!(cs.connected && !cs.pseudo && cs.layer === 'modal' && typeof decl['overflow-x'] === 'string' && typeof decl['overflow-y'] === 'string')) return;
    decl.overflow = decl['overflow-x'] === decl['overflow-y'] ? decl['overflow-x'] : decl['overflow-x'] + ' ' + decl['overflow-y'];
  }
  function computedNormalizeColorLonghands(cs){
    var decl=cs.decl,connected=cs.connected,pseudo=cs.pseudo;
    computedNormalizeOverflowModal(cs, decl);
    if (pseudo && typeof decl['background-color'] === 'string') decl['background-color'] = decl.backgroundColor = normalizeComputedColor(decl['background-color']);
    // turbo-dom#374: Chrome 156 serializes every `<color>` longhand in computed style as
    // rgb() / rgba() / the authored string for a value it does not recognize; a named
    // keyword (`red`) reads back as `rgb(255, 0, 0)`. `color` and (for pseudos)
    // `background-color` are already normalized above; normalize the rest here.
    if (connected) {
      for (var ci = 0; ci < computedColorLonghands.length; ci++) {
        var colorProp = computedColorLonghands[ci];
        var colorValue = decl[colorProp];
        if (typeof colorValue === 'string' && colorValue) decl[colorProp] = decl[computedCamel(colorProp)] = normalizeComputedColor(colorValue);
      }
    }
  }
  // Used-value resolution for percentage width/height. A no-layout env returns a raw "50%"
  // verbatim, but a containing-block percentage has a well-defined used value in normal flow
  // (pct * the containing block's content width/height), which a real browser reports in px.
  // Resolve against the nearest ancestor with a px (or, for width, auto=fill) measure so
  // getComputedStyle(child).width matches a browser without a full layout engine.
  // The `parentElement` fallback computedUsedPx walks up (an Element parent or an Element
  // sibling via parentNode).
  function computedUsedPxParent(node){
    return node.parentElement || (node.parentNode && node.parentNode.nodeType === 1 ? node.parentNode : null);
  }
  function computedUsedPxRaw(cs,node,axis){try{return node===cs.el?cs.decl[axis]:computedSnapshotValue(cs.parentStyle(node),axis);}catch(e){return null;}}
  function computedUsedPxResolve(raw,cs,parent,axis,depth){
    var mpc=$c.exec(/^(-?\d+(?:\.\d+)?)%$/,raw);
    if(mpc){var base=computedUsedPx(cs,parent,axis,depth+1);return base==null?null:(parseFloat(mpc[1])/100)*base;}
    if((raw===''||raw==='auto')&&axis==='width')return computedUsedPx(cs,parent,axis,depth+1);
    return null;
  }
  function computedUsedPx(cs, node, axis, depth){
    if (!node || depth > 32) return null;
    var raw=computedUsedPxRaw(cs,node,axis);if(raw==null)return null;
    raw=String(raw);
    var mpx = $c.exec(/^(-?\d+(?:\.\d+)?)px$/, raw); if (mpx) return parseFloat(mpx[1]);
    return computedUsedPxResolve(raw,cs,computedUsedPxParent(node),axis,depth);
  }
  function computedUsedSizes(cs){
    var pseudo=cs.pseudo,layer=cs.layer,decl=cs.decl;
    // The fullscreen element's width / height (100% !important of the viewport, its
    // containing block as a fixed box) are the viewport's innerWidth / innerHeight in px
    // (turbo-dom#58); without a viewport they stay 100%.
    if (!pseudo && layer === 'fullscreen') {
      computedVolatile = true;
      [['width', g.innerWidth], ['height', g.innerHeight]].forEach(function(pair){
        var size = +pair[1];
        if (size > 0 && cssFontFinite(size)) cs.setProp(pair[0], ($Mround(size * 1000) / 1000) + 'px', true);
      });
    }
    // An embedder's layout answers the used size at read time (computedUsedSize); a box-less
    // display keeps its computed percentage (Chrome).
    if (!pseudo && layer !== 'fullscreen' && typeof __ns.layoutUsedSize !== 'function' && decl.display !== 'none' && decl.display !== 'contents') try {
      $c.forEach(['width', 'height'], function(axis){
        var raw = decl[axis];
        if (typeof raw === 'string' && $c.test(/^-?\d+(?:\.\d+)?%$/, $c.trim(raw))) {
          var px = computedUsedPx(cs, cs.el, axis, 0);
          if (px != null && isFinite(px)) cs.setProp(axis, ($Mround(px * 1000) / 1000) + 'px');
        }
      });
    } catch(e){}
  }
  function computedFontDependents(cs,fontState){
    var decl=cs.decl,connected=cs.connected,pseudo=cs.pseudo;
    // dialog:modal max-width / max-height: calc(100% - 6px - 2em) at the dialog's font size.
    if (connected && !pseudo && cs.layer === 'modal' && fontState && typeof fontState.size === 'number') {
      var inset = 'calc(100% - ' + cssFontString(cssFontNumber(cssApply(cssFontPrecision, 2 * fontState.size + 6, [6]))) + 'px)';
      $c.forEach(['max-width', 'max-height'], function(name){ if (decl[name] == null) decl[name] = decl[computedCamel(name)] = inset; });
    }
    // Custom properties inherit their specified text (turbo-dom#286).
    if (connected && !pseudo && cs.inheritParent) inheritCustomProperties(decl, cs.parentStyle(cs.inheritParent));
  }
  function computeStyleFresh(el,snapshots,sink,pseudo,deps,base){
    var cached=!pseudo&&el&&computedSnapshotGet(snapshots,el);if(cached)return cached;
    if(noteCascadeRun)noteCascadeRun();
    // Percentage lookup may visit several ancestors before font inheritance asks
    // for the immediate parent again. Reuse each resolved ancestor only in this read.
    function parentStyle(node){var style=computeStyleSnapshot(node,snapshots);if(deps&&deps[deps.length-2]!==node){deps[deps.length]=node;deps[deps.length]=style;}return style;}
    // An element of a child iframe document (turbo-dom#299) cascades as connected.
    var inFrame = !!el && computedFrameRoot(el) !== null;
    var unavailable = computedStyleUnavailable(el,snapshots,sink,pseudo,inFrame);
    if (unavailable) return unavailable;
    var cs = new ComputedCascade(el,snapshots,sink,pseudo,deps,base,inFrame);
    // Properties are exposed both kebab and camelCase.
    var decl = cs.decl = { __proto__:null, getPropertyValue: function(p){ var v = this[p]; if (v == null) v = this[computedCamel(p)]; return v == null ? '' : String(v); }, getPropertyPriority: function(){ return ''; }, length: 0 };
    cs.priorities = {__proto__:null};
    cs.parentStyle = parentStyle;
    cs.setProp = function(name, val, important){ cascadeSetProp(cs, name, val, important); };
    cs.info = computedPseudoInfoFor(el,pseudo,parentStyle);
    cs.layer = computedLayerOf(el,pseudo,inFrame);
    computedCascadeDecls(cs);
    if (sink) { sink.values = computedSinkValues(decl); return null; }
    computedInheritance(cs);
    computedApplyAnimations(cs);
    // Tree-counting functions take this element's counts; their style follows its siblings.
    var treeNames=treeCountingValues(decl,el,pseudo);
    computedColor(cs);
    computeStyleFreshDisplay(cs, decl);
    computedVisibility(cs);
    computedContent(cs);
    computedTransformValues(cs);
    // `zoom` is not inherited; it computes to its factor, with normal and 0 as 1.
    decl.zoom = cs.connected ? computedZoom(decl.zoom, function(){ return cs.inheritedValue('zoom'); }) : '';
    computedNormalizeColorLonghands(cs);
    computedUsedSizes(cs);
    var fontState=computedFonts(decl,el,parentStyle,cs.connected,pseudo,cs.inheritSnapshot);
    computedMathValues(decl,treeNames,fontState,cs.connected,function(name,value){decl[name]=decl[computedCamel(name)]=value;});
    computedFontDependents(cs,fontState);
    // A connected element's unset properties compute lazily (computedDefault, turbo-dom#305).
    var snapshot=__ns.makeComputedStyleDeclaration(decl,el,fontState,pseudo,defaultsParentOf(el,pseudo,cs.connected,cs.inheritParent));
    computeStyleFreshStore(el,pseudo,snapshots,snapshot);
    return snapshot;
  }
  // display: connected elements pass through computedDisplayPosition; a disconnected one
  // leaves an unset `display` as '' (the empty default).
  function computeStyleFreshDisplay(cs, decl){
    if (cs.connected) computedDisplayPosition(cs);
    else if (decl.display == null) decl.display = '';
  }
  // Stash the fresh snapshot in the per-call cache (skip pseudo reads, which share a key).
  function computeStyleFreshStore(el, pseudo, snapshots, snapshot){
    if (el && !pseudo) computedSnapshotSet(snapshots, el, snapshot);
  }
  var ownMatchMedia = null;
  function installWindowOperations(){
    if (typeof g.requestAnimationFrame === 'undefined') g.requestAnimationFrame = function(cb){ return setTimeout(function(){ cb(Date.now()); }, 0); };
    if (typeof g.cancelAnimationFrame === 'undefined') g.cancelAnimationFrame = function(id){ clearTimeout(id); };
    if (typeof g.matchMedia === 'undefined') ownMatchMedia = g.matchMedia = function(q){ return __ns.matchMedia(q); };
    if (typeof g.scrollTo === 'undefined') g.scrollTo = function(){};
    // Window operations are not constructors: no own `prototype` (turbo-dom#221).
    ['getComputedStyle','requestAnimationFrame','cancelAnimationFrame','matchMedia','scrollTo'].forEach(function(name){
      var fn=g[name];
      if(typeof fn==='function'&&Object.prototype.hasOwnProperty.call(fn,'prototype')&&__ns.makeNativeFunction)g[name]=__ns.makeNativeFunction(fn,name,name==='scrollTo'?0:1);
    });
    if (ownMatchMedia) ownMatchMedia = g.matchMedia;
  }
  installWindowOperations();
  // The cascade evaluates media text directly against the viewport (no MediaQueryList per
  // rule); like Chrome (and jsdom) it never asks a page's replacement window.matchMedia.
  // A child iframe window's getComputedStyle evaluates @media against its own viewport.
  var mediaView = null;
  function mediaMatches(text){ return __ns.mediaTextMatches(text, mediaView); }
  // The embedder's media mode (set_media_mode): 'chrome' (the default) or 'jsdom', jsdom
  // 27's behaviour for vitest parity: no matchMedia / MediaQueryList / MediaQueryListEvent,
  // and a media list applies only when one of its items is exactly `screen` (jsdom's
  // forEachMatchingSheetRuleOfElement); an @import also applies with no media list.
  var mediaMode = 'chrome', jsdomHidden = null;
  function jsdomScreen(text){ var items = $c.split(String(text), ','); for (var i = 0; i < items.length; i++) if ($c.trim(items[i]) === 'screen') return true; return false; }
  __ns.setMediaMode = function(mode){
    if (mode === mediaMode) return;
    mediaMode = mode;
    var names = ['matchMedia', 'MediaQueryList', 'MediaQueryListEvent'];
    if (mode === 'jsdom') { jsdomHidden = names.map(function(name){ var d = Object.getOwnPropertyDescriptor(g, name); delete g[name]; return d; }); }
    else { names.forEach(function(name, i){ if (jsdomHidden[i]) Object.defineProperty(g, name, jsdomHidden[i]); }); jsdomHidden = null; }
    invalidateComputedStyles();
  };
  __ns.withMediaView = function(target, run){ var outer = mediaView; mediaView = target; try { return run(); } finally { mediaView = outer; } };
  // `Image(width, height)` is the native legacy factory in html_image.js (turbo-dom#23); image
  // loading (load / error events, natural size) is the host's (record.imageHost).
  function installLocationFallback(){
    if (typeof g.location === 'undefined' || g.location == null) {
      g.location = { href: 'http://localhost/', origin: 'http://localhost', protocol: 'http:', host: 'localhost', hostname: 'localhost', port: '', pathname: '/', search: '', hash: '', assign: function(){}, replace: function(){}, reload: function(){}, toString: function(){ return this.href; } };
    }
  }
  installLocationFallback();
  // DOM interface constructors (for `instanceof` / global presence). Stubs; identity not enforced.
  // Event records stay private: readonly Web-IDL getters and native dispatch
  // share this state without exposing writable bookkeeping to page scripts.
  var eventRecords = new WeakMap();
  var getEventRecord = WeakMap.prototype.get.bind(eventRecords);
  var setEventRecord = WeakMap.prototype.set.bind(eventRecords);
  var eventClockOrigin = Date.now();
  var eventDefineProperty = Object.defineProperty;
  var eventObjectFreeze = Object.freeze;
  // turbo-dom#423: the returned copy is frozen so a listener's stashed composedPath()
  // cannot be mutated (defense-in-depth; record.path itself is frozen per invocation in
  // event_listeners.js). Chrome 156 still hands out a fresh array per call; the extra
  // freeze matches the immutability the per-invocation path contract promises. The
  // captured `eventObjectFreeze` keeps the engine off page-patched Object.freeze
  // (turbo-dom#199 captured-intrinsics rule).
  function copyEventPath(path) {
    var result = [];
    for (var index=0; index<path.length; index++) eventDefineProperty(result,index,{__proto__:null,value:path[index],writable:true,configurable:true,enumerable:true});
    return eventObjectFreeze(result);
  }
  var nativeEventRecord = g.__nativeEventRecord;
  delete g.__nativeEventRecord;
  function eventRecord(event) {
    var record = getEventRecord(event);
    if (!record && nativeEventRecord) record = nativeEventRecord(event);
    if (!record) throw new TypeError('Illegal invocation');
    return record;
  }
  if (!__ns.ttEvent) {
    function eventString(value) {
      if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string');
      return String(value);
    }
    function eventValidateInit(init){if(init!=null&&typeof init!=='object'&&typeof init!=='function')throw new TypeError('EventInit must be a dictionary');return init||{};}
    function eventTimeStamp(){return g.performance&&typeof g.performance.now==='function'?g.performance.now():Date.now()-eventClockOrigin;}
    function eventMakeState(type,init){return {__proto__:null,type:eventString(type),bubbles:!!init.bubbles,cancelable:!!init.cancelable,composed:!!init.composed,defaultPrevented:false,target:null,currentTarget:null,stop:false,stopImmediate:false,eventPhase:0,timeStamp:eventTimeStamp(),trusted:false,dispatching:false,passive:false,path:[]};}
    function Event(type, init = {}) {
      if (!new.target && !(this instanceof Event)) throw new TypeError('Event requires new');
      if (!arguments.length) throw new TypeError('Event requires a type');
      init = eventValidateInit(init);
      setEventRecord(this,eventMakeState(type,init));
      var get = ({get isTrusted(){return eventRecord(this).trusted;}});
      var trustedGetter=$gopd(get,'isTrusted').get;
      if(__ns.makeNativeFunction)trustedGetter=__ns.makeNativeFunction(trustedGetter,'get isTrusted',0);
      $def(this,'isTrusted',{__proto__:null,get:trustedGetter,enumerable:true});
    }
    function eventMember(name, descriptor) {
      descriptor.enumerable=true;descriptor.configurable=true;
      if(descriptor.get&&__ns.makeNativeFunction)descriptor.get=__ns.makeNativeFunction(descriptor.get,'get '+name,0);
      if(descriptor.set&&__ns.makeNativeFunction)descriptor.set=__ns.makeNativeFunction(descriptor.set,'set '+name,1);
      if(descriptor.value){descriptor.writable=true;if(__ns.makeNativeFunction)descriptor.value=__ns.makeNativeFunction(descriptor.value,name,descriptor.value.length);}
      Object.defineProperty(Event.prototype,name,descriptor);
    }
    ['type','target','currentTarget','eventPhase','bubbles','cancelable','defaultPrevented','composed','timeStamp'].forEach(function(name){
      var getter=Object.getOwnPropertyDescriptor({get [name](){return eventRecord(this)[name];}},name).get;
      eventMember(name,{get:getter});
    });
    eventMember('srcElement',{get:Object.getOwnPropertyDescriptor({get srcElement(){return eventRecord(this).target;}},'srcElement').get});
    eventMember('returnValue',Object.getOwnPropertyDescriptor({get returnValue(){return !eventRecord(this).defaultPrevented;},set returnValue(value){var state=eventRecord(this);if(!value&&state.cancelable&&!state.passive)state.defaultPrevented=true;}},'returnValue'));
    eventMember('cancelBubble',Object.getOwnPropertyDescriptor({get cancelBubble(){return eventRecord(this).stop;},set cancelBubble(value){var state=eventRecord(this);if(value)state.stop=true;}},'cancelBubble'));
    var methods={
      preventDefault(){var state=eventRecord(this);if(state.cancelable&&!state.passive)state.defaultPrevented=true;},
      stopPropagation(){eventRecord(this).stop=true;},
      stopImmediatePropagation(){var state=eventRecord(this);state.stop=true;state.stopImmediate=true;},
      composedPath(){return copyEventPath(eventRecord(this).path);},
      initEvent(type,bubbles=false,cancelable=false){
        var state=eventRecord(this);if(!arguments.length)throw new TypeError('A type is required');if(state.dispatching)return;
        state.type=eventString(type);state.bubbles=!!bubbles;state.cancelable=!!cancelable;
        state.defaultPrevented=false;state.stop=false;state.stopImmediate=false;state.trusted=false;state.target=null;
      }
    };
    Object.keys(methods).forEach(function(name){eventMember(name,{value:methods[name]});});
    Object.defineProperty(Event.prototype,Symbol.toStringTag,{value:'Event',configurable:true});
    ['NONE','CAPTURING_PHASE','AT_TARGET','BUBBLING_PHASE'].forEach(function(name,value){
      // Configurable until key_order.js has arranged and frozen the interface (all constants below).
      [Event,Event.prototype].forEach(function(object){Object.defineProperty(object,name,{value:value,enumerable:true,configurable:true});});
    });
    g.Event = Event;
    function mkSub(extra){ return function(type, init = {}){ if (arguments.length === 0) throw new TypeError('Event requires a type'); init = init || {}; $call(Event, this, type, init); if (extra) extra(this, init); for (var k in init){ if (!(k in this)) this[k] = init[k]; } }; }
    function sub(name, extra){ var C = mkSub(extra); Object.defineProperty(C, 'name', {value: name, configurable: true}); C.prototype = Object.create(Event.prototype); C.prototype.constructor = C; Object.defineProperty(C.prototype,Symbol.toStringTag,{value:name,configurable:true});g[name] = C; }
    sub('MessageEvent',function(self,init){self.data=init.data===undefined?null:init.data;self.origin=init.origin===undefined?'':eventString(init.origin);self.lastEventId=init.lastEventId===undefined?'':eventString(init.lastEventId);self.source=init.source||null;self.ports=init.ports||[];});
    // UI input interfaces keep Web-IDL state private; arbitrary dictionary keys
    // never become instance properties and inherited getters retain their brand.
    var uiStates=new WeakMap(), mouseStates=new WeakMap(), keyStates=new WeakMap(), inputStates=new WeakMap(), focusStates=new WeakMap(), pointerStates=new WeakMap(), wheelStates=new WeakMap();
    var numberValue=function(value,fallback=0){if(value===undefined)return fallback;if(typeof value==='bigint')throw new TypeError('Cannot convert a BigInt to a number');return Number(value);};
    var intValue=function(value){return numberValue(value)>>0;};
    var shortValue=function(value){return (intValue(value)<<16)>>16;};
    var stringValue=function(value,fallback=''){return value===undefined?fallback:eventString(value);};
    var stateOf=function(states,receiver){var state=$call($wmGet,states,receiver);if(!state)throw new TypeError('Illegal invocation');return state;};
    var modifierNames={Alt:'altKey',Control:'ctrlKey',Meta:'metaKey',Shift:'shiftKey',AltGraph:'modifierAltGraph',CapsLock:'modifierCapsLock',Fn:'modifierFn',FnLock:'modifierFnLock',Hyper:'modifierHyper',NumLock:'modifierNumLock',ScrollLock:'modifierScrollLock',Super:'modifierSuper',Symbol:'modifierSymbol',SymbolLock:'modifierSymbolLock'};
    var modifierKeys=Object.keys(modifierNames);
    function modifiers(init){var state={};for(var index=0;index<modifierKeys.length;index++){var name=modifierNames[modifierKeys[index]];state[name]=!!init[name];}return state;}
    function member(prototype,name,descriptor){descriptor.configurable=true;descriptor.enumerable=true;if(descriptor.value){descriptor.writable=true;if(__ns.makeNativeFunction)descriptor.value=__ns.makeNativeFunction(descriptor.value,name,descriptor.value.length);}if(descriptor.get&&__ns.makeNativeFunction)descriptor.get=__ns.makeNativeFunction(descriptor.get,'get '+name,0);Object.defineProperty(prototype,name,descriptor);}
    // relatedTarget: the dispatch retargets it per listener (turbo-dom#66) through the private
    // record; outside a dispatch it is the last retargeted value, or the initialized one.
    function relatedRecord(event,related){var record=eventRecord(event);record.relatedTarget=related;record.relatedAdjusted=false;record.adjustedRelatedTarget=null;}
    function relatedGetter(prototype,states){member(prototype,'relatedTarget',{get:Object.getOwnPropertyDescriptor({get relatedTarget(){var state=stateOf(states,this),record=eventRecord(this);return record.relatedAdjusted?record.adjustedRelatedTarget:state.relatedTarget;}},'relatedTarget').get});}
    function readonly(prototype,states,names){names.forEach(function(name){member(prototype,name,{get:Object.getOwnPropertyDescriptor({get [name](){return stateOf(states,this)[name];}},name).get});});}
    function tagEvent(Constructor,name){Object.defineProperty(Constructor.prototype,Symbol.toStringTag,{value:name,configurable:true});}
    function initializeUI(event,init){var view=init.view===undefined?null:init.view;if(view!==null&&view!==g&&!((g.Window&&view instanceof g.Window)))throw new TypeError('view must be a Window');$call($wmSet,uiStates,event,{view:view,detail:intValue(init.detail),sourceCapabilities:null,pseudoTarget:null});}
    class UIEvent extends Event {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);initializeUI(this,init||{});}
    }
    readonly(UIEvent.prototype,uiStates,['view','detail','sourceCapabilities','pseudoTarget']);
    member(UIEvent.prototype,'which',{get:Object.getOwnPropertyDescriptor({get which(){stateOf(uiStates,this);var keys=$call($wmGet,keyStates,this);if(keys)return keys.charCode||keys.keyCode;var mouse=$call($wmGet,mouseStates,this);if(mouse)return mouse.button===2?3:mouse.button===1?2:mouse.button===0?1:0;return 0;}},'which').get});
    var initUIEvent=function initUIEvent(type,bubbles=false,cancelable=false,view=null,detail=0){stateOf(uiStates,this);if(!arguments.length)throw new TypeError('A type is required');if(eventRecord(this).dispatching)return;$call(methods.initEvent,this,type,bubbles,cancelable);initializeUI(this,{view:view,detail:detail});};member(UIEvent.prototype,'initUIEvent',{value:initUIEvent});
    tagEvent(UIEvent,'UIEvent');g.UIEvent=UIEvent;
    function mouseState(init){var state=modifiers(init);var axes=['screenX','screenY','clientX','clientY','movementX','movementY'];for(var index=0;index<axes.length;index++)state[axes[index]]=numberValue(init[axes[index]]);state.button=shortValue(init.button);state.buttons=intValue(init.buttons)&65535;state.relatedTarget=init.relatedTarget||null;if(state.relatedTarget!==null&&!(state.relatedTarget instanceof g.EventTarget))throw new TypeError('relatedTarget must be an EventTarget');return state;}
    class MouseEvent extends UIEvent {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);$call($wmSet,mouseStates,this,mouseState(init||{}));relatedRecord(this,$call($wmGet,mouseStates,this).relatedTarget);}
    }
    readonly(MouseEvent.prototype,mouseStates,['screenX','screenY','clientX','clientY','ctrlKey','shiftKey','altKey','metaKey','button','buttons','movementX','movementY']);relatedGetter(MouseEvent.prototype,mouseStates);
    ['pageX','pageY','x','y','offsetX','offsetY','layerX','layerY'].forEach(function(name){var axis=name.endsWith('X')||name==='x'?'clientX':'clientY',page=name.startsWith('page');member(MouseEvent.prototype,name,{get:Object.getOwnPropertyDescriptor({get [name](){var value=stateOf(mouseStates,this)[axis];return page?value+(axis==='clientX'?(g.scrollX||0):(g.scrollY||0)):value;}},name).get});});
    ['fromElement','toElement'].forEach(function(name){member(MouseEvent.prototype,name,{get:Object.getOwnPropertyDescriptor({get [name](){var state=stateOf(mouseStates,this),event=eventRecord(this);return (name==='fromElement')===(event.type==='mouseover'||event.type==='mouseenter')?state.relatedTarget:event.target;}},name).get});});
    function modifier(states,receiver,key){var state=stateOf(states,receiver);return !!state[modifierNames[eventString(key)]];}
    member(MouseEvent.prototype,'getModifierState',{value:function getModifierState(key){if(!arguments.length)throw new TypeError('A modifier is required');return modifier(mouseStates,this,key);}});
    var initMouseEventImpl=function(self,args){stateOf(mouseStates,self);if(eventRecord(self).dispatching)return;$call(initUIEvent,self,args.type,args.bubbles,args.cancelable,args.view,args.detail);$call($wmSet,mouseStates,self,mouseState(args));relatedRecord(self,$call($wmGet,mouseStates,self).relatedTarget);};
    var initMouseEventDefaults={bubbles:false,cancelable:false,view:null,detail:0,screenX:0,screenY:0,clientX:0,clientY:0,ctrlKey:false,altKey:false,shiftKey:false,metaKey:false,button:0,relatedTarget:null};
    member(MouseEvent.prototype,'initMouseEvent',{value:function initMouseEvent(type){stateOf(mouseStates,this);if(!arguments.length)throw new TypeError('A type is required');var args=$O.assign({__proto__:null,type:type},initMouseEventDefaults);var names=['bubbles','cancelable','view','detail','screenX','screenY','clientX','clientY','ctrlKey','altKey','shiftKey','metaKey','button','relatedTarget'];for(var i=0;i<names.length;i++)if(i+1<arguments.length)args[names[i]]=arguments[i+1];initMouseEventImpl(this,args);}});
    tagEvent(MouseEvent,'MouseEvent');g.MouseEvent=MouseEvent;
    class KeyboardEvent extends UIEvent {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};var state=modifiers(init);state.key=stringValue(init.key);state.code=stringValue(init.code);state.location=intValue(init.location)>>>0;state.repeat=!!init.repeat;state.isComposing=!!init.isComposing;state.charCode=intValue(init.charCode)>>>0;state.keyCode=intValue(init.keyCode)>>>0;$call($wmSet,keyStates,this,state);}
    }
    readonly(KeyboardEvent.prototype,keyStates,['key','code','location','ctrlKey','shiftKey','altKey','metaKey','repeat','isComposing','charCode','keyCode']);
    member(KeyboardEvent.prototype,'getModifierState',{value:function getModifierState(key){if(!arguments.length)throw new TypeError('A modifier is required');return modifier(keyStates,this,key);}});
    function initKeyboardEventApply(state,key,location,ctrlKey,altKey,shiftKey,metaKey){state.key=eventString(key);state.location=intValue(location)>>>0;state.ctrlKey=!!ctrlKey;state.altKey=!!altKey;state.shiftKey=!!shiftKey;state.metaKey=!!metaKey;}
    var initKeyboardEventDefaults=[false,false,null,'',0,false,false,false,false];
    function initKeyboardEventArgs(a){var d=initKeyboardEventDefaults,out=[];for(var i=0;i<d.length;i++)out[i]=i+1<a.length?a[i+1]:d[i];return out;}
    function initKeyboardEventBody(self,a){var state=stateOf(keyStates,self);if(!a.length)throw new TypeError('A type is required');if(eventRecord(self).dispatching)return;var p=initKeyboardEventArgs(a);$call(initUIEvent,self,a[0],p[0],p[1],p[2],0);initKeyboardEventApply(state,p[3],p[4],p[5],p[6],p[7],p[8]);}
    member(KeyboardEvent.prototype,'initKeyboardEvent',{value:function initKeyboardEvent(){initKeyboardEventBody(this,arguments);}});
    ['DOM_KEY_LOCATION_STANDARD','DOM_KEY_LOCATION_LEFT','DOM_KEY_LOCATION_RIGHT','DOM_KEY_LOCATION_NUMPAD'].forEach(function(name,value){[KeyboardEvent,KeyboardEvent.prototype].forEach(function(object){Object.defineProperty(object,name,{value:value,enumerable:true,configurable:true});});});
    tagEvent(KeyboardEvent,'KeyboardEvent');g.KeyboardEvent=KeyboardEvent;
    class InputEvent extends UIEvent {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};$call($wmSet,inputStates,this,{data:init.data==null?null:eventString(init.data),isComposing:!!init.isComposing,inputType:stringValue(init.inputType),dataTransfer:init.dataTransfer||null});}
    }
    readonly(InputEvent.prototype,inputStates,['data','isComposing','inputType','dataTransfer']);
    member(InputEvent.prototype,'getTargetRanges',{value:function getTargetRanges(){stateOf(inputStates,this);return [];}});
    tagEvent(InputEvent,'InputEvent');g.InputEvent=InputEvent;
    class FocusEvent extends UIEvent {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);var related=(init||{}).relatedTarget||null;if(related!==null&&!(related instanceof g.EventTarget))throw new TypeError('relatedTarget must be an EventTarget');$call($wmSet,focusStates,this,{relatedTarget:related});relatedRecord(this,related);}
    }
    relatedGetter(FocusEvent.prototype,focusStates);tagEvent(FocusEvent,'FocusEvent');g.FocusEvent=FocusEvent;
    class PointerEvent extends MouseEvent {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};var state={pointerId:intValue(init.pointerId),width:numberValue(init.width,1),height:numberValue(init.height,1),pressure:numberValue(init.pressure),tangentialPressure:numberValue(init.tangentialPressure),tiltX:intValue(init.tiltX),tiltY:intValue(init.tiltY),twist:intValue(init.twist)>>>0,pointerType:stringValue(init.pointerType),isPrimary:!!init.isPrimary,altitudeAngle:numberValue(init.altitudeAngle,Math.PI/2),azimuthAngle:numberValue(init.azimuthAngle),persistentDeviceId:intValue(init.persistentDeviceId)};$call($wmSet,pointerStates,this,state);}
    }
    readonly(PointerEvent.prototype,pointerStates,['pointerId','width','height','pressure','tangentialPressure','tiltX','tiltY','twist','pointerType','isPrimary','altitudeAngle','azimuthAngle','persistentDeviceId']);
    member(PointerEvent.prototype,'getCoalescedEvents',{value:function getCoalescedEvents(){stateOf(pointerStates,this);return [];}});
    member(PointerEvent.prototype,'getPredictedEvents',{value:function getPredictedEvents(){stateOf(pointerStates,this);return [];}});
    tagEvent(PointerEvent,'PointerEvent');g.PointerEvent=PointerEvent;
    class WheelEvent extends MouseEvent {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};$call($wmSet,wheelStates,this,{deltaX:numberValue(init.deltaX),deltaY:numberValue(init.deltaY),deltaZ:numberValue(init.deltaZ),deltaMode:intValue(init.deltaMode)>>>0,momentum:false});}
    }
    readonly(WheelEvent.prototype,wheelStates,['deltaX','deltaY','deltaZ','deltaMode','momentum']);
    ['wheelDelta','wheelDeltaX','wheelDeltaY'].forEach(function(name){member(WheelEvent.prototype,name,{get:Object.getOwnPropertyDescriptor({get [name](){var state=stateOf(wheelStates,this);return -3*(name==='wheelDeltaX'?state.deltaX:state.deltaY);}},name).get});});
    ['DOM_DELTA_PIXEL','DOM_DELTA_LINE','DOM_DELTA_PAGE'].forEach(function(name,value){[WheelEvent,WheelEvent.prototype].forEach(function(object){Object.defineProperty(object,name,{value:value,enumerable:true,configurable:true});});});
    tagEvent(WheelEvent,'WheelEvent');g.WheelEvent=WheelEvent;
    // Dictionary-member events (CustomEvent, PopStateEvent, HashChangeEvent): private state
    // behind readonly getters, like the UI family above.
    var customStates=new WeakMap(), popStates=new WeakMap(), hashStates=new WeakMap();
    class CustomEvent extends Event {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};$call($wmSet,customStates,this,{detail:init.detail===undefined?null:init.detail});}
    }
    readonly(CustomEvent.prototype,customStates,['detail']);
    member(CustomEvent.prototype,'initCustomEvent',{value:function initCustomEvent(type,bubbles=false,cancelable=false,detail=null){var state=stateOf(customStates,this);if(!arguments.length)throw new TypeError('A type is required');if(eventRecord(this).dispatching)return;$call(methods.initEvent,this,type,bubbles,cancelable);state.detail=detail;}});
    tagEvent(CustomEvent,'CustomEvent');
    class PopStateEvent extends Event {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};$call($wmSet,popStates,this,{state:init.state===undefined?null:init.state,hasUAVisualTransition:!!init.hasUAVisualTransition});}
    }
    readonly(PopStateEvent.prototype,popStates,['state','hasUAVisualTransition']);tagEvent(PopStateEvent,'PopStateEvent');
    class HashChangeEvent extends Event {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};$call($wmSet,hashStates,this,{oldURL:stringValue(init.oldURL),newURL:stringValue(init.newURL)});}
    }
    readonly(HashChangeEvent.prototype,hashStates,['oldURL','newURL']);tagEvent(HashChangeEvent,'HashChangeEvent');
    // ProgressEvent (FileReader/XHR) and CloseEvent (WebSocket).
    var progressStates=new WeakMap(), closeStates=new WeakMap();
    // ProgressEventInit loaded/total are restricted doubles in Chrome 156.
    var restrictedDouble=function(value,name){value=numberValue(value);if(!isFinite(value))throw new TypeError("Failed to construct 'ProgressEvent': Failed to read the '"+name+"' property from 'ProgressEventInit': The provided double value is non-finite.");return value;};
    class ProgressEvent extends Event {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};$call($wmSet,progressStates,this,{lengthComputable:!!init.lengthComputable,loaded:restrictedDouble(init.loaded,'loaded'),total:restrictedDouble(init.total,'total')});}
    }
    readonly(ProgressEvent.prototype,progressStates,['lengthComputable','loaded','total']);tagEvent(ProgressEvent,'ProgressEvent');
    class CloseEvent extends Event {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};$call($wmSet,closeStates,this,{wasClean:!!init.wasClean,code:intValue(init.code)&65535,reason:stringValue(init.reason)});}
    }
    readonly(CloseEvent.prototype,closeStates,['wasClean','code','reason']);tagEvent(CloseEvent,'CloseEvent');
    // Use the native class bridge for actual implementations, preserving private
    // records and subclass new.target rather than replacing Function.toString.
    [UIEvent,MouseEvent,KeyboardEvent,InputEvent,FocusEvent,PointerEvent,WheelEvent,CustomEvent,PopStateEvent,HashChangeEvent,ProgressEvent,CloseEvent].forEach(function(implementation){var name=implementation.name,constructor=__ns.makeNativeFunction?__ns.makeNativeFunction(implementation,name,1,true,true):implementation;constructor.prototype=implementation.prototype;Object.defineProperty(constructor.prototype,'constructor',{value:constructor,writable:true,configurable:true});Object.defineProperty(constructor,'prototype',{writable:false});Object.keys(implementation).forEach(function(key){Object.defineProperty(constructor,key,Object.getOwnPropertyDescriptor(implementation,key));});Object.defineProperty(g,name,{value:constructor,writable:true,configurable:true,enumerable:false});});
    [MouseEvent,KeyboardEvent,InputEvent,FocusEvent].forEach(function(implementation){Object.setPrototypeOf(g[implementation.name],g.UIEvent);});
    [CustomEvent,PopStateEvent,HashChangeEvent,ProgressEvent,CloseEvent].forEach(function(implementation){Object.setPrototypeOf(g[implementation.name],Event);});
    [PointerEvent,WheelEvent].forEach(function(implementation){Object.setPrototypeOf(g[implementation.name],g.MouseEvent);});
    sub('CompositionEvent'); sub('DragEvent'); sub('TouchEvent'); sub('ClipboardEvent');
    // document.createEvent: html_reflections.js (turbo-dom#341).
    // The installer captures and deletes this function before guest execution.
    g.__captureEventRecord=function(event){return getEventRecord(event);};
    __ns.ttEvent=true;
  }
const __domExceptionState = new WeakMap();
const __domExceptionCodes = {IndexSizeError:1, HierarchyRequestError:3, WrongDocumentError:4,
  InvalidCharacterError:5, NoModificationAllowedError:7, NotFoundError:8, NotSupportedError:9,
  InUseAttributeError:10, InvalidStateError:11, SyntaxError:12, InvalidModificationError:13,
  NamespaceError:14, InvalidAccessError:15, TypeMismatchError:17, SecurityError:18,
  NetworkError:19, AbortError:20, URLMismatchError:21, QuotaExceededError:22,
  TimeoutError:23, InvalidNodeTypeError:24, DataCloneError:25};
const __domExceptionGet = __domExceptionState.get.bind(__domExceptionState);
const __domExceptionSet = __domExceptionState.set.bind(__domExceptionState);
const __exceptionStringConstructor = String;
const __exceptionState = (exception) => {
  const state = __domExceptionGet(exception);
  if (!state) throw new TypeError("Illegal invocation");
  return state;
};
const __exceptionString = value => {
  if (typeof value === "symbol") throw new TypeError("Cannot convert a Symbol to a string");
  return __exceptionStringConstructor(value);
};
g.DOMException = class DOMException {
  constructor(message = "", name = "Error") {
    const text = __exceptionString(message), kind = __exceptionString(name);
    __domExceptionSet(this, {message: text, name: kind});
  }
  get name() { return __exceptionState(this).name; }
  get message() { return __exceptionState(this).message; }
  get code() { return __domExceptionCodes[__exceptionState(this).name] || 0; }
};
Object.setPrototypeOf(DOMException.prototype, Error.prototype);
// Wrap the Chrome 156 DOMException numeric-code tables and the `enumerable: true` pass
// over name/message/code into a nested helper, so the top-level IIFE's cc stays at 12.
installDOMExceptionCodes();
function installDOMExceptionCodes(){
  for (const name of ["name", "message", "code"]) {
    const descriptor = Object.getOwnPropertyDescriptor(DOMException.prototype, name);
    Object.defineProperty(DOMException.prototype, name, {...descriptor, enumerable: true});
  }
  Object.defineProperty(DOMException.prototype, Symbol.toStringTag, {value: "DOMException", configurable: true});
  for (const [name, code] of Object.entries({INDEX_SIZE_ERR:1, DOMSTRING_SIZE_ERR:2,
    HIERARCHY_REQUEST_ERR:3, WRONG_DOCUMENT_ERR:4, INVALID_CHARACTER_ERR:5, NO_DATA_ALLOWED_ERR:6,
    NO_MODIFICATION_ALLOWED_ERR:7, NOT_FOUND_ERR:8, NOT_SUPPORTED_ERR:9, INUSE_ATTRIBUTE_ERR:10,
    INVALID_STATE_ERR:11, SYNTAX_ERR:12, INVALID_MODIFICATION_ERR:13, NAMESPACE_ERR:14,
    INVALID_ACCESS_ERR:15, VALIDATION_ERR:16, TYPE_MISMATCH_ERR:17, SECURITY_ERR:18,
    NETWORK_ERR:19, ABORT_ERR:20, URL_MISMATCH_ERR:21, QUOTA_EXCEEDED_ERR:22, TIMEOUT_ERR:23,
    INVALID_NODE_TYPE_ERR:24, DATA_CLONE_ERR:25})) {
    for (const object of [DOMException, DOMException.prototype]) Object.defineProperty(object, name, {value: code, enumerable: true, configurable: true});
  }
}

if(typeof __ns.makeNativeFunction==='function'){
  var exceptionImplementation=g.DOMException;
  var exceptionConstructor=__ns.makeNativeFunction(exceptionImplementation,'DOMException',0,true,true);
  exceptionConstructor.prototype=exceptionImplementation.prototype;
  Object.defineProperty(exceptionConstructor.prototype,'constructor',{value:exceptionConstructor,configurable:true,writable:true});
  Object.defineProperty(exceptionConstructor,'prototype',{writable:false});
  for(var exceptionKey of Object.keys(exceptionImplementation))Object.defineProperty(exceptionConstructor,exceptionKey,Object.getOwnPropertyDescriptor(exceptionImplementation,exceptionKey));
  ['name','message','code'].forEach(function(key){var descriptor=Object.getOwnPropertyDescriptor(exceptionConstructor.prototype,key);descriptor.get=__ns.makeNativeFunction(descriptor.get,'get '+key,0);Object.defineProperty(exceptionConstructor.prototype,key,descriptor);});
  g.DOMException=exceptionConstructor;
}
// The realm's DOMException interface (turbo-surf#42, turbo-dom#72): the one an embedder registered
// (register_dom_exception: turbo-surf installs its own after the binding), else the binding's.
// Resolved whenever an exception is raised, so no module captures a constructor at install and a
// page reassigning globalThis.DOMException changes nothing. `new ts.DOMException(message, name)`
// constructs one; `instanceof ts.DOMException` tests against the current interface.
var __realmExceptionNative=g.__nativeRealmDOMException,__bindingException=g.DOMException,__exceptionConstruct=Reflect.construct;delete g.__nativeRealmDOMException;
// URL-reflecting getters and currentSrc parse against the node document's base URL (turbo-dom#73).
__ns.resolveDocumentURL=g.__nativeResolveDocumentURL;delete g.__nativeResolveDocumentURL;
__ns.realmDOMException=function(){var constructor=typeof __realmExceptionNative==='function'?__realmExceptionNative():undefined;return typeof constructor==='function'?constructor:__bindingException;};
__ns.DOMException=function DOMException(){return __exceptionConstruct(__ns.realmDOMException(),arguments);};
Object.defineProperty(__ns.DOMException,Symbol.hasInstance,{value:function(value){return value instanceof __ns.realmDOMException();}});
  function beginEventDispatch(event,target,path,host=false){
    var state=eventRecord(event);
    if(state.dispatching||!state.type)throw new __ns.DOMException('The event is already being dispatched or uninitialized.','InvalidStateError');
    state.dispatching=true;state.trusted=host;
    state.target=target;state.currentTarget=target;state.eventPhase=2;state.path=path;
    return state;
  }
  function endEventDispatch(state){state.currentTarget=null;state.eventPhase=0;state.path=[];state.dispatching=false;state.stop=false;state.stopImmediate=false;}
  var ctors = ['Node','Element','HTMLElement','HTMLDivElement','HTMLInputElement','HTMLButtonElement','HTMLAnchorElement','HTMLSelectElement','HTMLTextAreaElement','HTMLFormElement','HTMLImageElement','HTMLLabelElement','HTMLOptionElement','HTMLUListElement','HTMLLIElement','HTMLSpanElement','HTMLParagraphElement','HTMLHeadingElement','HTMLTableElement','HTMLIFrameElement','HTMLCanvasElement','HTMLStyleElement','HTMLScriptElement','HTMLDocument','Document','DocumentFragment','ShadowRoot','Text','Comment','SVGElement','SVGSVGElement','MathMLElement','DOMParser','EventTarget','AbortController','AbortSignal','DOMException',
    'UIEvent','MouseEvent','KeyboardEvent','FocusEvent','InputEvent','TouchEvent','PointerEvent','WheelEvent','DragEvent','ClipboardEvent','AnimationEvent','TransitionEvent','MessageEvent','ProgressEvent','CompositionEvent','PopStateEvent','HashChangeEvent','StorageEvent','ErrorEvent','CloseEvent',
    // DOM node/collection/CSS interfaces app bundles do `instanceof` against (an
    // undefined RHS throws "not an object" and aborts the chunk — a single missing
    // `Response`/`NodeList` cascades through webpack init and kills React hydration).
    'CharacterData','Attr','DocumentType','CDATASection','ProcessingInstruction','NodeList','HTMLCollection',
    'DOMTokenList','NamedNodeMap','CSSStyleDeclaration','CSSRule','StyleSheet','MediaList',
    'DOMStringMap','MutationRecord','MediaQueryList','DOMRect','DOMRectReadOnly','DOMPoint'];
  ctors.forEach(function(n){ if (typeof g[n] === "undefined") { var f = function(){}; f.prototype = {}; try { Object.defineProperty(f, "name", { value: n, configurable: true }); } catch(e){} Object.defineProperty(g, n, { value: f, writable: true, configurable: true, enumerable: false }); } });
  // EventTarget must be a REAL subclassable base. Apps + libraries write
  // `class Foo extends EventTarget { … this.addEventListener(…) }` (event buses, modal
  // managers, state stores — Nike's global-nav modal manager does exactly this); with an
  // empty `EventTarget.prototype` the instance's `addEventListener` is undefined and the
  // call throws. An uncaught throw mid-hydration makes React discard the SSR tree and
  // client-render the whole root (#425), blanking server-rendered content. Give the
  // prototype a working listener registry so subclass instances behave as event targets.
  (function(){
    var ET = g.EventTarget;
    if (ET && ET.prototype && typeof ET.prototype.addEventListener !== "function") {
      ET.prototype.addEventListener = function(type, cb){
        if (!cb || (typeof cb !== "function" && typeof cb.handleEvent !== "function")) return;
        var m = this.__etl || (this.__etl = {}); (m[type] = m[type] || []).push(cb);
      };
      ET.prototype.removeEventListener = function(type, cb){
        var m = this.__etl; if (!m || !m[type]) return;
        var i = m[type].indexOf(cb); if (i >= 0) m[type].splice(i, 1);
      };

    }
    if(ET&&ET.prototype)ET.prototype.dispatchEvent=function(ev){
      var state=beginEventDispatch(ev,this,[this]);
      var registry=this.__etl||this.__listeners,list=registry&&registry[state.type];
      try{if(list)for(var cb of list.slice()){
        if(state.stopImmediate)break;
        try{typeof cb==='function'?cb.call(this,ev):cb.handleEvent(ev);}catch(e){}
      }}finally{endEventDispatch(state);}
      return !state.defaultPrevented;
    };
  })();
  // matchMedia returns a live MediaQueryList (an EventTarget) evaluated against the window
  // viewport (innerWidth / innerHeight / devicePixelRatio, which the embedder sets) with
  // Chrome headless's defaults for the user-preference features; a `resize` dispatched on
  // the window re-evaluates every list and fires `change` on those that flipped
  // (turbo-dom#287, Chrome 156 control `match_media_chrome156.json`).
  (function(){
    // Builtins are captured: a page patching WeakMap / Reflect must not reach the engine.
    var construct = Reflect.construct, wmGet = WeakMap.prototype.get, wmSet = WeakMap.prototype.set;
    var getOf = function(map, key){ return $apply(wmGet, map, [key]); }, setOf = function(map, key, value){ $apply(wmSet, map, [key, value]); };
    var make = __ns.makeNativeFunction, $apply = Reflect.apply, records = new WeakMap(), lists = [], watching = false;
    // EventTarget is rebuilt later in the bootstrap: construct through, and call, the final one.
    var ET = g.EventTarget, base = ET.prototype;
    var call = function(name, target, args){ return $apply(base[name], target, args); };
    var TYPES = { all: true, screen: true, print: false, tty: false, tv: false, projection: false, handheld: false, braille: false, embossed: false, aural: false, speech: false };
    var IDENTS = { 'prefers-color-scheme': 'light', 'prefers-reduced-motion': 'no-preference', 'prefers-contrast': 'no-preference',
      'prefers-reduced-transparency': 'no-preference', 'prefers-reduced-data': 'no-preference', 'forced-colors': 'none', 'inverted-colors': 'none',
      hover: 'hover', 'any-hover': 'hover', pointer: 'fine', 'any-pointer': 'fine', 'display-mode': 'browser', scripting: 'enabled', update: 'fast',
      'overflow-block': 'scroll', 'overflow-inline': 'scroll', 'color-gamut': 'srgb', 'dynamic-range': 'standard', 'video-dynamic-range': 'standard' };
    // The window whose viewport a query is evaluated against: the top window, or a child
    // iframe window (its content box) while one of its lists evaluates.
    var view = g;
    var size = function(name){ var value = view[name]; return typeof value === 'number' && isFinite(value) ? value : NaN; };
    var dpr = function(){ var value = size('devicePixelRatio'); return value > 0 ? value : 1; };
    var screenSize = function(axis, fallback){ var screen = g.screen, value = screen && screen[axis]; return typeof value === 'number' ? value : size(fallback); };
    // Range features: [value kind, current value]; ratios compare as width/height pairs.
    var RANGES = {
      width: ['length', function(){ return size('innerWidth'); }], height: ['length', function(){ return size('innerHeight'); }],
      'device-width': ['length', function(){ return screenSize('width', 'innerWidth'); }], 'device-height': ['length', function(){ return screenSize('height', 'innerHeight'); }],
      'aspect-ratio': ['ratio', function(){ return [size('innerWidth'), size('innerHeight')]; }],
      'device-aspect-ratio': ['ratio', function(){ return [screenSize('width', 'innerWidth'), screenSize('height', 'innerHeight')]; }],
      resolution: ['resolution', dpr], '-webkit-device-pixel-ratio': ['number', dpr],
      color: ['number', function(){ return 8; }], 'color-index': ['number', function(){ return 0; }],
      monochrome: ['number', function(){ return 0; }], grid: ['number', function(){ return 0; }],
    };
    var UNITS = { px: 1, em: 16, rem: 16, cm: 96 / 2.54, mm: 96 / 25.4, q: 96 / 101.6, in: 96, pt: 4 / 3, pc: 16 };
    var RESOLUTION = { dppx: 1, x: 1, dpi: 1 / 96, dpcm: 2.54 / 96 };
    // Ratio literal `a / b` (default denominator `1`) as [[num, denom], serialization] or null.
    function parseRatio(text){
      var ratio = /^(\d+(?:\.\d+)?)(?:\s*\/\s*(\d+(?:\.\d+)?))?$/.exec(text);
      if (!ratio) return null;
      var denom = ratio[2] === undefined ? 1 : +ratio[2];
      var denomSer = ratio[2] === undefined ? '1' : ratio[2];
      return [[+ratio[1], denom], ratio[1] + ' / ' + denomSer];
    }
    // A value token of `kind` as [number (or ratio pair), serialization], or null when invalid.
    function parseValueUnit(table,kind,match,text){if(!match[2])return kind==='length'&&+match[1]===0?[0,text]:null;return table[match[2]]?[+match[1]*table[match[2]],text]:null;}
    function parseValue(kind, text){
      if (kind === 'ratio') return parseRatio(text);
      var match = /^(-?(?:\d+\.?\d*|\.\d+))([a-z]*)$/.exec(text);
      if (!match) return null;
      var table = kind === 'length' ? UNITS : kind === 'resolution' ? RESOLUTION : null;
      if (!table) return match[2] ? null : [+match[1], text];
      return parseValueUnit(table,kind,match,text);
    }
    function compare(kind, current, value){
      if (kind === 'ratio') return current[0] * value[1] - current[1] * value[0];
      return current - value;
    }
    var TEST = { '=': function(d){ return d === 0; }, '<': function(d){ return d < 0; }, '<=': function(d){ return d <= 0; }, '>': function(d){ return d > 0; }, '>=': function(d){ return d >= 0; } };
    var FLIP = { '=': '=', '<': '>', '<=': '>=', '>': '<', '>=': '<=' };
    // `name: value` (with min- / max- prefixes) as a node {ser, test}, or null when invalid.
    function plainFeatureIdent(name,text){return /^[a-z-]+$/.test(text)?{ser:'('+name+': '+text+')',test:function(){return IDENTS[name]===text;}}:null;}
    function plainFeatureOrientation(text){return text==='portrait'||text==='landscape'?{ser:'(orientation: '+text+')',test:function(){var width=size('innerWidth'),height=size('innerHeight');return text==='portrait'?height>=width:height<width;}}:null;}
    function plainFeature(name, text){
      var prefix = /^(-webkit-)?(min-|max-)/.exec(name), base = prefix ? (prefix[1] || '') + name.slice(prefix[0].length) : name;
      if (IDENTS[name] !== undefined) return plainFeatureIdent(name,text);
      if (name === 'orientation') return plainFeatureOrientation(text);
      var range = RANGES[base], value = range && parseValue(range[0], text);
      if (!value) return null;
      var op = !prefix ? '=' : prefix[2] === 'min-' ? '>=' : '<=';
      return { ser: '(' + name + ': ' + value[1] + ')', test: function(){ return TEST[op](compare(range[0], range[1](), value[0])); } };
    }
    function booleanFeature(name){
      if (IDENTS[name] !== undefined) return { ser: '(' + name + ')', test: function(){ return IDENTS[name] !== 'none' && IDENTS[name] !== 'no-preference'; } };
      if (name === 'orientation') return { ser: '(orientation)', test: function(){ return true; } };
      var range = RANGES[name];
      return range ? { ser: '(' + name + ')', test: function(){ var current = range[1](); return range[0] === 'ratio' ? current[0] !== 0 : current !== 0 && current === current; } } : null;
    }
    // One `<name> <op> <value>` (or `<value> <op> <name>`) pair inside a range feature.
    // Rewrites parts[valueAt] with the canonical serialization and appends a [op, value]
    // check to `checks`. Returns false when the operand fails to parse.
    function rangeFeatureCheck(parts, index, nameAt, range, checks){
      var valueAt = index - 1 === nameAt ? index + 1 : index - 1;
      var value = parseValue(range[0], parts[valueAt]);
      if (!value || !TEST[parts[index]]) return false;
      parts[valueAt] = value[1];
      checks.push([valueAt > nameAt ? parts[index] : FLIP[parts[index]], value[0]]);
      return true;
    }
    // `(width >= 600px)` / `(400px <= width <= 1100px)` range syntax.
    function rangeFeatureNameAt(parts){var at=RANGES[parts[0]]?0:RANGES[parts[2]]?2:-1;if(at<0||parts.length===5&&at!==2)return -1;return at;}
    function rangeFeature(text){
      var parts = text.split(/\s*(<=|>=|<|>|=)\s*/);
      if (parts.length !== 3 && parts.length !== 5) return null;
      var nameAt = rangeFeatureNameAt(parts);
      if (nameAt < 0) return null;
      var range = RANGES[parts[nameAt]], checks = [];
      for (var index = 1; index < parts.length; index += 2) if (!rangeFeatureCheck(parts, index, nameAt, range, checks)) return null;
      return { ser: '(' + parts.join(' ') + ')', test: function(){ var current = range[1](); return checks.every(function(check){ return TEST[check[0]](compare(range[0], current, check[1])); }); } };
    }
    function feature(text){
      var colon = text.indexOf(':');
      if (colon >= 0) return plainFeature(text.slice(0, colon).trim(), text.slice(colon + 1).trim());
      return /[<>=]/.test(text) ? rangeFeature(text) : booleanFeature(text);
    }
    // Top-level tokens: words and balanced `( … )` groups.
    // The inner loop body returns the new `start` on success, or -1 to signal an
    // unbalanced `)` which makes `tokens` return null. `state` carries { depth, start }.
    function tokensStepParen(text,index,state){if(state.depth++===0){if(state.start>=0)state.out.push(text.slice(state.start,index));state.start=index;}return true;}
    function tokensStepClose(text,index,state){if(--state.depth<0)return false;if(state.depth===0){state.out.push(text.slice(state.start,index+1));state.start=-1;}return true;}
    function tokensStep(text, index, c, state){
      if (c === '(') return tokensStepParen(text,index,state);
      if (c === ')') return tokensStepClose(text,index,state);
      if (state.depth === 0 && /\s/.test(c)) { if (state.start >= 0) state.out.push(text.slice(state.start, index)); state.start = -1; }
      else if (state.start < 0) state.start = index;
      return true;
    }
    function tokens(text){
      var state = { out: [], depth: 0, start: -1 };
      for (var index = 0; index < text.length; index++) {
        if (!tokensStep(text, index, text[index], state)) return null;
      }
      if (state.start >= 0) state.out.push(text.slice(state.start));
      return state.depth === 0 ? state.out : null;
    }
    function group(token){
      if (!token || token[0] !== '(') return null;
      var inner = token.slice(1, -1).trim();
      if (inner[0] === '(' || /^not\s/.test(inner)) { var nested = condition(tokens(inner) || []); return nested && { ser: '(' + nested.ser + ')', test: nested.test }; }
      return feature(inner);
    }
    // `[not] (a) [and|or (b)]…`
    function conditionJoinerValid(list,at,joiner,negate){return (list[at]==='and'||list[at]==='or')&&!(joiner&&joiner!==list[at])&&!negate;}
    function conditionResult(nodes,joiner,negate,first){
      var ser=nodes.map(function(node){return node.ser;}).join(' '+(joiner||'and')+' ');
      if(negate)return {ser:'not '+ser,test:function(){return !first.test();}};
      return {ser:ser,test:function(){return joiner==='or'?nodes.some(function(node){return node.test();}):nodes.every(function(node){return node.test();});}};
    }
    function condition(list){
      var negate = list[0] === 'not', at = negate ? 1 : 0, first = group(list[at]);
      if (!first) return null;
      var nodes = [first], joiner = null;
      for (at++; at < list.length; at += 2) {
        if (!conditionJoinerValid(list,at,joiner,negate)) return null;
        joiner = list[at]; var next = group(list[at + 1]); if (!next) return null; nodes.push(next);
      }
      return conditionResult(nodes,joiner,negate,first);
    }
    // `[not|only] type [and condition]`
    function typedCondition(rest){if(!rest.length)return {ok:true,cond:null};if(rest[0]!=='and'||rest[1]==='not')return {ok:false};var cond=condition(rest.slice(1));return cond?{ok:true,cond:cond}:{ok:false};}
    function typed(list){
      var modifier = list[0] === 'not' || list[0] === 'only' ? list[0] : '', at = modifier ? 1 : 0, type = list[at];
      if (!/^[a-z][a-z0-9-]*$/.test(type) || /^(and|or|not|only)$/.test(type)) return null;
      var result = typedCondition(list.slice(at + 1));
      if (!result.ok) return null;
      var cond = result.cond, ser = (modifier ? modifier + ' ' : '') + type + (cond ? ' and ' + cond.ser : '');
      return { ser: ser, test: function(){ var matches = TYPES[type] === true && (!cond || cond.test()); return modifier === 'not' ? !matches : matches; } };
    }
    var NOT_ALL = { ser: 'not all', test: function(){ return false; } };
    function query(text){
      var list = tokens(text);
      if (!list || !list.length) return NOT_ALL;
      return (list[0][0] === '(' || list[0] === 'not' && list[1] && list[1][0] === '(' ? condition(list) : typed(list)) || NOT_ALL;
    }
    // Split at top-level commas, close what EOF left open, lower-case (media queries are ASCII
    // case-insensitive).
    function parseListClose(text){var open=0;for(var index=0;index<text.length;index++)open+=text[index]==='('?1:text[index]===')'?-1:0;while(open-->0)text+=')';return text;}
    function parseListSplit(text){var parts=[],depth=0,start=0;for(var at=0;at<text.length;at++){if(text[at]==='(')depth++;else if(text[at]===')')depth--;else if(text[at]===','&&depth===0){parts.push(text.slice(start,at));start=at+1;}}parts.push(text.slice(start));return parts;}
    function parseList(text){
      text = parseListClose(String(text).toLowerCase().replace(/\s+/g, ' ').trim());
      if (text === '') return [];
      return parseListSplit(text).map(function(part){ return query(part.trim()); });
    }
    function evaluate(queries, target){
      var outer = view; view = target || g;
      try { return !queries.length || queries.some(function(item){ return item.test(); }); } finally { view = outer; }
    }
    var cache = new Map();
    function parsed(text){ var queries = cache.get(text); if (!queries) { queries = parseList(text); if (cache.size > 512) cache.clear(); cache.set(text, queries); } return queries; }
    __ns.mediaTextMatches = function(text, target){ return evaluate(parsed(String(text)), target); };
    var record = function(list){ var state = getOf(records, list); if (!state) throw new TypeError('Illegal invocation'); return state; };
    var MQLEvent = g.MediaQueryListEvent;
    if (typeof MQLEvent !== 'function' || !('matches' in MQLEvent.prototype)) {
      var eventRecords = new WeakMap(), Event = g.Event;
      var EventDelegate = function(type, init){
        if (!new.target) throw new TypeError("Failed to construct 'MediaQueryListEvent': Please use the 'new' operator.");
        if (!arguments.length) throw new TypeError("Failed to construct 'MediaQueryListEvent': 1 argument required, but only 0 present.");
        init = init == null ? {} : init;
        var event = construct(Event, [String(type), { bubbles: !!init.bubbles, cancelable: !!init.cancelable, composed: !!init.composed }], new.target);
        setOf(eventRecords, event, { matches: !!init.matches, media: init.media === undefined ? '' : String(init.media) });
        return event;
      };
      Object.setPrototypeOf(EventDelegate.prototype, Event.prototype);
      Object.defineProperty(EventDelegate.prototype, Symbol.toStringTag, { value: 'MediaQueryListEvent', configurable: true });
      ['matches', 'media'].forEach(function(name){
        Object.defineProperty(EventDelegate.prototype, name, { enumerable: true, configurable: true, get: make(function(){ var state = getOf(eventRecords, this); if (!state) throw new TypeError('Illegal invocation'); return state[name]; }, 'get ' + name, 0) });
      });
      MQLEvent = make(EventDelegate, 'MediaQueryListEvent', 1, true, true);
      MQLEvent.prototype = EventDelegate.prototype;
      Object.defineProperty(EventDelegate.prototype, 'constructor', { value: MQLEvent, writable: true, configurable: true });
      Object.defineProperty(MQLEvent, 'prototype', { writable: false });
      Object.setPrototypeOf(MQLEvent, Event);
      Object.defineProperty(g, 'MediaQueryListEvent', { value: MQLEvent, writable: true, configurable: true });
    }
    var MQL = make(function(){ throw new TypeError('Illegal constructor'); }, 'MediaQueryList', 0, true);
    var proto = Object.create(ET.prototype);
    Object.defineProperty(proto, Symbol.toStringTag, { value: 'MediaQueryList', configurable: true });
    Object.defineProperty(proto, 'constructor', { value: MQL, writable: true, configurable: true });
    var methods = {
      addListener: function(callback){ record(this); if (callback != null) call('addEventListener', this, ['change', callback]); },
      removeListener: function(callback){ record(this); if (callback != null) call('removeEventListener', this, ['change', callback]); },
    };
    Object.keys(methods).forEach(function(name){ Object.defineProperty(proto, name, { value: make(methods[name], name, 1), writable: true, enumerable: true, configurable: true }); });
    Object.defineProperty(proto, 'media', { enumerable: true, configurable: true, get: make(function(){ return record(this).media; }, 'get media', 0) });
    Object.defineProperty(proto, 'matches', { enumerable: true, configurable: true, get: make(function(){ var state = record(this); return evaluate(state.queries, state.view); }, 'get matches', 0) });
    Object.defineProperty(proto, 'onchange', { enumerable: true, configurable: true,
      get: make(function(){ return record(this).handler; }, 'get onchange', 0),
      set: make(function(value){
        var state = record(this);
        state.handler = typeof value === 'function' || value !== null && typeof value === 'object' ? value : null;
        if (state.handler && !state.invoker) { state.invoker = function(event){ var handler = state.handler; if (typeof handler === 'function') return $apply(handler, this, [event]); }; call('addEventListener', this, ['change', state.invoker]); }
      }, 'set onchange', 1) });
    MQL.prototype = proto;
    Object.defineProperty(MQL, 'prototype', { writable: false });
    Object.setPrototypeOf(MQL, ET);
    Object.defineProperty(g, 'MediaQueryList', { value: MQL, writable: true, configurable: true, enumerable: false });
    __ns.matchMedia = function(text, target){
      text = String(text); target = target || g;
      // Listen once the page runs: the window's listener store is installed after this block.
      if (!watching) { watching = true; try { g.addEventListener('resize', viewportChanged); } catch(e){} }
      if (target !== g && typeof __ns.watchFrameViewport === 'function') __ns.watchFrameViewport(target);
      var queries = parsed(text), list = construct(__ns.eventTargetConstructor || ET, [], MQL);
      setOf(records, list, { media: queries.map(function(item){ return item.ser; }).join(', '), queries: queries, view: target, handler: null, invoker: null, last: evaluate(queries, target) });
      lists.push(typeof WeakRef === 'function' ? new WeakRef(list) : { deref: function(){ return list; } });
      return list;
    };
    // A viewport change: every live list that flipped fires `change`, in creation order.
    function viewportChanged(){
      var live = [];
      lists.forEach(function(ref){ var list = ref.deref(); if (list) live.push([ref, list]); });
      lists = live.map(function(pair){ return pair[0]; });
      live.forEach(function(pair){
        var state = getOf(records, pair[1]), now = evaluate(state.queries, state.view);
        if (now === state.last) return;
        state.last = now;
        call('dispatchEvent', pair[1], [new MQLEvent('change', { matches: now, media: state.media })]);
      });
    }
    __ns.mediaViewportChanged = viewportChanged;
  })();
  // make `node instanceof HTMLElement/Element/Node/...` work for native DOM nodes (jest-dom's
  // checkHtmlElement + many libs rely on it) via Symbol.hasInstance keyed on nodeType.
  function iface(name, pred){ var f = g[name] || function(){}; try { Object.defineProperty(f, Symbol.hasInstance, { configurable: true, value: pred }); } catch(e){} g[name] = f; }
  var isNode = function(o){ return o != null && typeof o === 'object' && typeof o.nodeType === 'number'; };
  iface('Node', isNode);
  iface('Element', function(o){ return isNode(o) && o.nodeType === 1; });
  iface('HTMLElement', function(o){ return isNode(o) && o.nodeType === 1; });
  iface('SVGElement', function(o){ return isNode(o) && o.nodeType === 1 && String(o.namespaceURI||'').indexOf('svg') >= 0; });
  iface('Text', function(o){ return isNode(o) && o.nodeType === 3; });
  iface('Comment', function(o){ return isNode(o) && o.nodeType === 8; });
  iface('DocumentFragment', function(o){ return isNode(o) && o.nodeType === 11; });
  iface('HTMLInputElement', function(o){ return isNode(o) && o.nodeType === 1 && String(o.tagName).toUpperCase() === 'INPUT'; });
  iface('HTMLTextAreaElement', function(o){ return isNode(o) && o.nodeType === 1 && String(o.tagName).toUpperCase() === 'TEXTAREA'; });
  iface('HTMLSelectElement', function(o){ return isNode(o) && o.nodeType === 1 && String(o.tagName).toUpperCase() === 'SELECT'; });
  // Node subtypes keyed on nodeType (CharacterData is the Text/Comment/CDATA base).
  iface('CharacterData', function(o){ return isNode(o) && (o.nodeType === 3 || o.nodeType === 4 || o.nodeType === 7 || o.nodeType === 8); });
  iface('Attr', function(o){ return isNode(o) && o.nodeType === 2; });
  iface('CDATASection', function(o){ return isNode(o) && o.nodeType === 4; });
  iface('ProcessingInstruction', function(o){ return isNode(o) && o.nodeType === 7; });
  // CSSStyleDeclaration: what el.style / getComputedStyle() return (has getPropertyValue).
  iface('CSSStyleDeclaration', function(o){ return o != null && typeof o === 'object' && typeof o.getPropertyValue === 'function'; });
  // Live-ish collections: array-like with length + item(). NodeList also allows forEach.
  iface('HTMLCollection', function(o){ return o != null && typeof o === 'object' && typeof o.length === 'number' && typeof o.item === 'function'; });
  iface('NodeList', function(o){ return o != null && typeof o === 'object' && typeof o.length === 'number' && (typeof o.item === 'function' || typeof o.forEach === 'function'); });
  iface('DOMTokenList', function(o){ return o != null && typeof o === 'object' && typeof o.length === 'number' && typeof o.toggle === 'function'; });
  // Audio(src) — a real HTMLAudioElement built through the DOM (so it participates in
  // the tree + carries the media API stubs below), matching `new Audio(url)`.
  function installAudioFactory(){
    if (typeof g.Audio === 'undefined') {
      g.Audio = function Audio(src){
        var el = d.createElement('audio');
        if (src != null) { try { el.src = String(src); } catch(e){} }
        return el;
      };
      try { g.Audio.prototype = (g.HTMLAudioElement && g.HTMLAudioElement.prototype) || {}; } catch(e){}
    }
  }
  installAudioFactory();
  // Media-element methods on the shared element prototype (headless has no audio/video
  // pipeline; these no-op so `el.play()`/`.load()` don't throw and `.play()` still
  // returns a promise as the spec requires). `canPlayType` reports no support.
  (function(){
    var mp = (g.HTMLElement && g.HTMLElement.prototype) || {};
    if (typeof mp.play !== 'function') mp.play = function(){ return Promise.resolve(); };
    if (typeof mp.pause !== 'function') mp.pause = function(){};
    if (typeof mp.load !== 'function') mp.load = function(){};
    if (typeof mp.canPlayType !== 'function') mp.canPlayType = function(){ return ''; };
  })();
  // Worker — a real class with the full interface. A single deno_core isolate has no
  // second thread, so it doesn't spawn one (the worker body doesn't run); it queues
  // postMessage + exposes onmessage/onerror + terminate so apps that construct a
  // Worker (analytics, off-main-thread hashing) don't throw and degrade gracefully.
  function installWorkerStubs(){
    if (typeof g.Worker === 'undefined') {
      // An EventTarget subclass with no own instance state (turbo-dom#221).
      var workerHandlers = new WeakMap();
      g.Worker = function Worker(url){ if (new.target === undefined) throw new TypeError("Failed to construct 'Worker': Please use the 'new' operator, this DOM object constructor cannot be called as a function."); var worker = Reflect.construct(g.EventTarget, [], new.target); workerHandlers.set(worker, {}); return worker; };
      Object.setPrototypeOf(g.Worker, g.EventTarget);
      Object.setPrototypeOf(g.Worker.prototype, g.EventTarget.prototype);
      g.Worker.prototype.postMessage = function(){};
      g.Worker.prototype.terminate = function(){};
      ['onmessage','onmessageerror','onerror'].forEach(function(name){ Object.defineProperty(g.Worker.prototype, name, { enumerable: true, configurable: true, get: function(){ var h = workerHandlers.get(this); return h && h[name] || null; }, set: function(value){ var h = workerHandlers.get(this); if (h) h[name] = typeof value === 'function' || (value && typeof value === 'object') ? value : null; } }); });
      if (typeof g.SharedWorker === 'undefined') { g.SharedWorker = function SharedWorker(url){ this.url = String(url || ''); this.port = { postMessage: function(){}, start: function(){}, close: function(){}, addEventListener: function(){}, removeEventListener: function(){}, onmessage: null }; }; }
    }
  }
  installWorkerStubs();
  // Extra HTML*Element constructors the base ctor list omits. App bundles reference these for
  // feature-detect / instanceof / subclassing (MUI's Dialog touches HTMLDialogElement); an
  // undefined reference aborts the chunk mid-hydration, blanking the tree. Single-tag elements get
  // a tag-keyed `instanceof`; abstract / multi-tag interfaces fall back to a generic element check.
  var htmlByTag = {
    DIALOG:'HTMLDialogElement', DATALIST:'HTMLDataListElement', FIELDSET:'HTMLFieldSetElement',
    LEGEND:'HTMLLegendElement', OL:'HTMLOListElement', DL:'HTMLDListElement', PRE:'HTMLPreElement',
    TR:'HTMLTableRowElement', COL:'HTMLTableColElement', CAPTION:'HTMLTableCaptionElement',
    PROGRESS:'HTMLProgressElement', METER:'HTMLMeterElement', DETAILS:'HTMLDetailsElement',
    PICTURE:'HTMLPictureElement', SOURCE:'HTMLSourceElement', TRACK:'HTMLTrackElement', VIDEO:'HTMLVideoElement',
    AUDIO:'HTMLAudioElement', TEMPLATE:'HTMLTemplateElement', SLOT:'HTMLSlotElement',
    BODY:'HTMLBodyElement', HTML:'HTMLHtmlElement', HEAD:'HTMLHeadElement', META:'HTMLMetaElement',
    LINK:'HTMLLinkElement', TITLE:'HTMLTitleElement', BASE:'HTMLBaseElement', BR:'HTMLBRElement',
    HR:'HTMLHRElement', OPTGROUP:'HTMLOptGroupElement', MAP:'HTMLMapElement', AREA:'HTMLAreaElement',
    OBJECT:'HTMLObjectElement', EMBED:'HTMLEmbedElement', OUTPUT:'HTMLOutputElement',
    MENU:'HTMLMenuElement', DATA:'HTMLDataElement', TIME:'HTMLTimeElement', PARAM:'HTMLParamElement',
  };
  // abstract base / multi-tag interfaces (no single tag → generic nodeType-1 instanceof).
  var htmlAbstract = ['HTMLMediaElement','HTMLTableCellElement','HTMLTableSectionElement','HTMLQuoteElement','HTMLUnknownElement'];
  (function(){
    var baseProto = (g.HTMLElement && g.HTMLElement.prototype) || {};
    var mk = function(name){ var f = g[name]; if (typeof f !== 'function') { f = function(){}; f.prototype = Object.create(baseProto); try { Object.defineProperty(f, 'name', { value: name, configurable: true }); } catch(e){} g[name] = f; } return f; };
    Object.keys(htmlByTag).forEach(function(tag){ var name = htmlByTag[tag]; mk(name); iface(name, (function(t){ return function(o){ return isNode(o) && o.nodeType === 1 && String(o.tagName).toUpperCase() === t; }; })(tag)); });
    htmlAbstract.forEach(function(name){ mk(name); iface(name, function(o){ return isNode(o) && o.nodeType === 1; }); });
  })();
  // window-level event listeners — a real registry so window.addEventListener('keydown', …) +
  // window.dispatchEvent(new KeyboardEvent(...)) work (e.g. global keyboard shortcuts).
  if (!__ns.winListeners) {
    var winL = __ns.winListeners = {};
    g.addEventListener = function(type, fn){ if (typeof fn !== 'function') return; (winL[type] = winL[type] || []).push(fn); };
    g.removeEventListener = function(type, fn){ var a = winL[type]; if (a){ var i = a.indexOf(fn); if (i >= 0) a.splice(i, 1); } };
    g.dispatchEvent = function(ev){
      var state=beginEventDispatch(ev,g,[g]),listeners=winL[state.type];
      try{if(listeners)for(var fn of listeners.slice()){if(state.stopImmediate)break;try{fn.call(g,ev);}catch(e){}}}
      finally{endEventDispatch(state);}return !state.defaultPrevented;
    };
  }
  // Invoke each listener in `list` on `target` for `event`; honours stopImmediatePropagation
  // and swallows per-listener exceptions. Hoisted so __captureHostEventDispatch stays under cc 12.
  function captureHostEventDispatchRun(target, event, list, state){
    if (!list) return;
    for (var listener of list.slice()) {
      if (state.stopImmediate) break;
      try { typeof listener === 'function' ? listener.call(target, event) : listener.handleEvent(event); } catch(e){}
    }
  }
  // Captured privately by the host before page scripts can run.
  g.__captureHostEventDispatch=function(event,target){
    'use strict';
    var state=beginEventDispatch(event,target,[target],true);
    // Chrome's Window lifecycle dispatch retains its legacy target/path/dispatch state.
    // Independently verified with raw CDP; ordinary dispatch still clears its state.
    var legacyWindow=target===g&&(state.type==='load'||state.type==='pageshow');
    if(legacyWindow)state.target=g.document;
    var registry=target===g?__ns.winListeners:(target.__etl||target.__listeners);
    var list=registry&&registry[state.type];
    try{captureHostEventDispatchRun(target,event,list,state);}
    finally{if(!legacyWindow)endEventDispatch(state);}
    return !state.defaultPrevented;
  };
  // document extras (pure-JS shims over the native tree).
  var d = g.document;
  d.defaultView = g;
  // Native Lightning CSS validates selectors/declarations and recovers invalid rules.
  // This scanner builds the facade's mutable object graph from that parsed CSS.
  var cssStores=cssMap('list');
  // Retained sheets keep their own list brand after this context installs another document.
  var cssListPrototypes={};
  var cssRecord=function(list){var record=$c.get(cssStores,list);if(!record)throw new TypeError('Illegal invocation');if(record.refresh&&!record.refreshing){record.refreshing=true;try{record.items=record.refresh();}finally{record.refreshing=false;}}return record;};
  $c.forEach(['CSSRuleList','StyleSheetList'],function(name){
    var illegalConstructor=function(){throw new TypeError('Illegal constructor');};
    var ctor=__ns.makeNativeFunction?__ns.makeNativeFunction(illegalConstructor,name,0,true):illegalConstructor;$O.defineProperty(ctor,'name',{value:name});
    ctor.prototype={};
    $O.defineProperty(ctor.prototype,Symbol.toStringTag,{value:name,configurable:true});
    var lengthGetter=$O.getOwnPropertyDescriptor({get length(){return cssRecord(this).items.length;}},'length').get;
    if(__ns.makeNativeFunction)lengthGetter=__ns.makeNativeFunction(lengthGetter,'get length',0);
    $O.defineProperty(ctor.prototype,'length',{configurable:true,enumerable:true,get:lengthGetter});
    var item=({item(index){if(!arguments.length)throw new TypeError("Failed to execute 'item' on '"+name+"': 1 argument required, but only 0 present.");return cssRecord(this).items[Number(index)>>>0]||null;}}).item;
    if(__ns.makeNativeFunction)item=__ns.makeNativeFunction(item,'item',1);
    $O.defineProperty(ctor.prototype,'item',{configurable:true,enumerable:true,writable:true,value:item});
    $O.defineProperty(ctor.prototype,'constructor',{configurable:true,writable:true,value:ctor});
    $O.defineProperty(ctor.prototype,Symbol.iterator,{configurable:true,writable:true,value:Array.prototype.values});
    $O.defineProperty(ctor,'prototype',{writable:false});
    $O.defineProperty(g,name,{value:ctor,writable:true,configurable:true,enumerable:false});
    cssListPrototypes[name]=ctor.prototype;
  });
  var cssIndex=function(key){return typeof key==='string'&&$c.test(/^(0|[1-9]\d*)$/,key)?Number(key):-1;};
  var cssList=function(name,refresh){
    var target=$O.create(cssListPrototypes[name||'CSSRuleList']),record={items:[],refresh:refresh,refreshing:false},proxy;
    proxy=new Proxy(target,{
      get:function(obj,key,receiver){var index=cssIndex(key);return index>=0?cssRecord(proxy).items[index]:$R.get(obj,key,receiver);},
      has:function(obj,key){var index=cssIndex(key);return index>=0?index<cssRecord(proxy).items.length:$R.has(obj,key);},
      ownKeys:function(){return $c.concat($c.map(cssRecord(proxy).items,function(_,i){return String(i);}),$R.ownKeys(target));},
      getOwnPropertyDescriptor:function(obj,key){var index=cssIndex(key),items=cssRecord(proxy).items;return index>=0&&index<items.length?{value:items[index],writable:false,enumerable:true,configurable:true}:$R.getOwnPropertyDescriptor(obj,key);},
      set:function(obj,key,value,receiver){return cssIndex(key)<0&&$R.set(obj,key,value,receiver);},
      deleteProperty:function(obj,key){return cssIndex(key)<0&&$R.deleteProperty(obj,key);}
    });$c.set(cssStores,proxy,record);return proxy;
  };
  var cssItems=function(list){return $c.get(cssStores,list).items;};
  var cssPartsQuoteStep = function(state,c){
    if(state.escaped) state.escaped=false;
    else if(c==='\\') state.escaped=true;
    else if(c===state.quote) state.quote='';
  };
  function cssPartsDepth(c){
    if(c==='('||c==='[')return 1;
    if(c===')'||c===']')return -1;
    return 0;
  }
  function cssPartsEnterState(state,c,next){
    if(c==='/'&&next==='*'){state.comment=true;return 1;}
    if(c==='"'||c==="'"){state.quote=c;return 0;}
    return -1;
  }
  function cssPartsStep(state,c,next,depth,separator,out,text,i,start){
    if(state.comment){if(c==='*'&&next==='/'){state.comment=false;state.skip=1;}return depth;}
    if(state.quote){cssPartsQuoteStep(state,c);return depth;}
    var enter=cssPartsEnterState(state,c,next);
    if(enter>=0){state.skip=enter;return depth;}
    depth+=cssPartsDepth(c);
    if(c===separator&&depth===0){$c.push(out,$c.slice(text,start.v,i));start.v=i+1;}
    return depth;
  }
  var cssParts = function(text, separator){
    var out=[], start={v:0}, depth=0;
    var state={quote:'',escaped:false,comment:false,skip:0};
    for (var i=0;i<text.length;i++) {
      state.skip=0;
      depth=cssPartsStep(state,text[i],i+1<text.length?text[i+1]:undefined,depth,separator,out,text,i,start);
      i+=state.skip;
    }
    $c.push(out,$c.slice(text,start.v)); return out;
  };
  function cssPreludeComma(text,i){
    var out=', ';
    while(i+1<text.length&&$c.test(/\s/,text[i+1]))i++;
    return {skipTo:i,insert:out};
  }
  // Neutral-state step (not already inside a comment / quoted string). Returns the new
  // value of `i`; appends to `out.val` and may flip `state.comment` / `state.quote`.
  function cssPreludeStepNeutral(state, text, i, c, next, out){
    if (c === '\\') { out.val += c; if (next !== undefined) out.val += text[++i]; return i; }
    if (c === '/' && next === '*') { state.comment = true; return i + 1; }
    if (c === '"' || c === "'") { state.quote = c; out.val += c; return i; }
    if (out.commas && c === ',') { var step = cssPreludeComma(text, i); out.val = $c.replace(out.val, /\s*$/, '') + step.insert; return step.skipTo; }
    out.val += c; return i;
  }
  function cssPrelude(text,commas){
    var state = { quote:'', escaped:false, comment:false };
    var out = { val:'', commas:commas };
    for (var i = 0; i < text.length; i++) {
      var c = text[i], next = i + 1 < text.length ? text[i + 1] : undefined;
      if (state.comment) { if (c === '*' && next === '/') { state.comment = false; i++; } continue; }
      if (state.quote) { out.val += c; cssPartsQuoteStep(state, c); continue; }
      i = cssPreludeStepNeutral(state, text, i, c, next, out);
    }
    return $c.trim(out.val);
  }
  var declarationRecords=cssMap('declaration'),inlineDeclarations=cssMap('inlineDeclaration'),cssPropertyCache=new Map();
  function declarationRecord(style){var record=$c.get(declarationRecords,style);if(!record)throw new TypeError('Illegal invocation');if(record.owner){var text=$c.call(appliedInlineRead,record.owner);if(text!==record.source){record.source=text;parseDeclaration(record,text);}}if(record.refresh){var fresh=$c.get(declarationRecords,record.refresh());if(fresh){record.values=fresh.values;record.names=fresh.names;record.fontState=fresh.fontState;record.defaultsOf=fresh.defaultsOf;record.defaultsParent=fresh.defaultsParent;}}return record;}
  function cssString(value){if(typeof value==='symbol')throw new TypeError('Cannot convert a Symbol to a string');return String(value);}
  function cssPropertyName(name){if(name==='cssFloat')return 'float';return $c.replace($c.replace(String(name),/[A-Z]/g,function(c){return '-'+$c.toLowerCase(c);}),/^webkit-/,'-webkit-');}
  function knownProperty(name){if($c.has(cssPropertyCache,name))return $c.get(cssPropertyCache,name);var known=typeof cssNativeParse==='function'&&cssNativeParse(name,true);if(!known&&__ns.typedOmProperty)known=__ns.typedOmProperty(name);if(known||__ns.typedOmProperty)$c.set(cssPropertyCache,name,!!known);return !!known;}
  // CSSOM shorthands (turbo-dom#90): rtdom::css_shorthand (__cssShorthandNative) expands a
  // shorthand into its longhands and serializes shorthands and cssText from them like Chrome
  // 156. A value is {text, priority, pending}: `pending` is [shorthand, text] for a longhand a
  // var() shorthand set (its text is ''), which only that shorthand serializes back.
  // Name lookups and longhand spellings are pure: memoized per name / declaration.
  var shorthandNames=new Map(),propertyAliases=new Map(),longhandTexts=new Map();
  var shorthandLists=new Map();
  function staleShorthands(name){if(!cssShorthand||$c.slice(name,0,2)==='--')return [];var list=$c.get(shorthandLists,name);if(!list){list=cssShorthand('shorthands',name);$c.set(shorthandLists,name,list);}return list;}
  function shorthandName(name){if(!cssShorthand)return null;var known=$c.get(shorthandNames,name);if(known===undefined){known=cssShorthand('shorthand',name);$c.set(shorthandNames,name,known);}return known;}
  function propertyAlias(name){if(!cssShorthand||$c.slice(name,0,2)==='--')return name;var alias=$c.get(propertyAliases,name);if(alias===undefined){alias=cssShorthand('alias',name)||name;$c.set(propertyAliases,name,alias);}return alias;}
  function blockEntries(record){var flat=[];for(var index=0;index<record.names.length;index++){var name=record.names[index],value=cssMapGet(record.values,name);$c.push(flat,name,value.text,value.priority==='important',value.pending?value.pending[0]:null,value.pending?value.pending[1]:null);}return flat;}
  function declarationText(record){if(cssShorthand&&!record.fontFace)return cssShorthand('text',blockEntries(record));var text='';for(var index=0;index<record.names.length;index++){var name=record.names[index],value=cssMapGet(record.values,name);if(index)text+=' ';text+=name+': '+value.text+(value.priority?' !important':'')+';';}return text;}
  function shorthandValue(record,name){return cssShorthand('value',name,blockEntries(record));}
  // The longhands a valid shorthand value sets: [[name, text, pending], ...], or null.
  function expandShorthandValue(name,value){var flat=cssShorthand('expand',name,value);if(!flat)return null;var out=[];for(var index=0;index<flat.length;index+=4)$c.push(out,[flat[index],flat[index+1],flat[index+2]===null?null:[flat[index+2],flat[index+3]]]);return out;}
  __ns.readDeclarationText=function(style){return declarationText(declarationRecord(style));};
  function colorText(name,text){if($c.slice(name,0,2)==='--'||!$c.test(/(?:^color$|color$)/,name))return text;var match=$c.exec(/^#([\da-f]{3}|[\da-f]{6})$/i,text);if(!match)return text;var hex=match[1];if(hex.length===3)hex=$c.replace(hex,/./g,function(c){return c+c;});return 'rgb('+parseInt($c.slice(hex,0,2),16)+', '+parseInt($c.slice(hex,2,4),16)+', '+parseInt($c.slice(hex,4,6),16)+')';}
  // `raw` is the stylesheet grammar's text of a longhand set directly: the computed cascade
  // reads it (full number precision), CSSOM reads the Chrome spelling in `text`.
  function storeDeclaration(record,name,text,priority,pending,raw){if(!$c.has(record.values,name))$c.push(record.names,name);$c.set(record.values,name,{text:pending?'':colorText(name,text),priority:priority||'',pending:pending||null,raw:raw||null});}
  // cssText parsing: a repeated property moves to the end; an !important one keeps out a later normal one.
  function parsedDeclaration(record,name,text,priority,pending,raw){var previous=$c.get(record.values,name);if(previous){if(previous.priority&&!priority)return;$c.delete(record.values,name);$c.splice(record.names,$c.indexOf(record.names,name),1);}storeDeclaration(record,name,text,priority,pending,raw);}
  // A longhand's value as Chrome spells it (turbo-dom#97): [text, raw] or null. rtdom's grammar
  // admits (or rejects) and spells the longhands it knows; any other value the stylesheet
  // grammar admits, rtdom spells context-free. `raw` is the stylesheet grammar's text.
  function longhandTextSpelled(spelled,raw,value){if(spelled[0]==='valid')return [spelled[1],raw===null?spelled[1]:raw];if(spelled[0]==='invalid')return null;if(raw!==null)return [spelled[1],raw];if(spelled[1]!==value&&parseLonghandText(spelled[1],spelled[1])!==null)return [spelled[1],spelled[1]];return raw===null?null:[raw,raw];}
  function longhandTextResolve(name,value,raw){if(!cssShorthand||$c.slice(name,0,2)==='--')return raw===null?null:[raw,raw];return longhandTextSpelled(cssShorthand('longhand',name,value),raw,value);}
  function longhandText(name,value){var key=name+'\u0000'+value,cached=$c.get(longhandTexts,key);if(cached!==undefined)return cached;var result=longhandTextResolve(name,value,parseLonghandText(name,value));if($mapSize(longhandTexts)>=4096)$c.clear(longhandTexts);$c.set(longhandTexts,key,result);return result;}
  // A var() / env() value is accepted at parse time once the native parser kept it; Typed OM
  // has no typed reading of it (turbo-dom#162).
  function deferredCssText(text){return $c.test(/(^|[^-\w])(var|env|if)\(/i,text);}
  function parseLonghandText(name,value){if(!$c.startsWith(name,'--')&&!knownProperty(name))return null;var parsed=typeof cssNativeParse==='function'?cssNativeParse('x { '+name+': '+value+' }'):'x { '+name+': '+value+' }';var body=$c.slice(parsed,$c.indexOf(parsed,'{')+1,$c.lastIndexOf(parsed,'}')),accepted=null;$c.forEach(cssParts(body,';'),function(part){var index=$c.indexOf(part,':');if(index<1||$c.trim($c.slice(part,0,index))!==name)return;var text=$c.trim($c.slice(part,index+1));if($c.test(/\s*!important\s*$/i,text))return;if(!$c.startsWith(name,'--')&&!deferredCssText(text)&&__ns.typedOmValid&&!__ns.typedOmValid(name,text))return;accepted=text;});return accepted;}
  // The [[name, text, pending, raw], ...] a declaration sets, or null when it is invalid.
  function declarationEntries(name,value){name=propertyAlias(name);if(shorthandName(name))return expandShorthandValue(name,value);var text=longhandText(name,value);return text===null?null:[[name,text[0],null,text[1]]];}
  function parseDeclaration(record,text){if(cssShorthand&&!record.fontFace){record.names=[];record.values=new Map();var list=cssShorthand('declarations',text);for(var index=0;index<list.length;index+=3){var name=list[index];if($c.slice(name,0,2)!=='--')name=$c.toLowerCase(name);var entries=declarationEntries(name,list[index+1]);if(entries)$c.forEach(entries,function(entry){parsedDeclaration(record,entry[0],entry[1],list[index+2]?'important':'',entry[2],entry[3]);});}return;}parseLegacyDeclaration(record,text);}
  // Accept / reject a legacy-parsed property name before its value is classified.
  function parseLegacyNameKnown(record, name){ return record.fontFace || $c.startsWith(name, '--') || knownProperty(name); }
  // Accept / reject the value through Typed OM once the !important suffix is stripped.
  function parseLegacyTypedValid(record, name, value){
    if (record.fontFace || $c.startsWith(name, '--') || !__ns.typedOmValid) return true;
    return __ns.typedOmValid(name, value);
  }
  function parseLegacyDeclaration(record,text){record.names=[];record.values=new Map();var selector=record.fontFace?'@font-face':'x',parsed=typeof cssNativeParse==='function'?cssNativeParse(selector+' { '+text+' }'):selector+' { '+text+' }';var body=$c.slice(parsed,$c.indexOf(parsed,'{')+1,$c.lastIndexOf(parsed,'}'));$c.forEach(cssParts(body,';'),function(part){var index=$c.indexOf(part,':');if(index<1)return;var name=$c.trim($c.slice(part,0,index));if(!parseLegacyNameKnown(record,name))return;var value=$c.trim($c.slice(part,index+1)),important=$c.test(/\s*!important\s*$/i,value);if(important)value=$c.trim($c.replace(value,/\s*!important\s*$/i,''));if(!parseLegacyTypedValid(record,name,value))return;var previous=$c.get(record.values,name);if(previous&&previous.priority&&!important)return;storeDeclaration(record,name,value,important?'important':'');});}
  function writableDeclaration(record){if(record.readOnly)throw new __ns.DOMException('The declaration is read-only','NoModificationAllowedError');if(!record.owner)bumpCssomEpoch();}
  function commitDeclaration(record){if(record.owner){record.source=declarationText(record);$c.call(appliedInlineWrite,record.owner,record.source);}}
  // removeProperty: a shorthand removes its longhands and returns '' (Chrome).
  function removeDeclaration(record,name){if(!record.fontFace){name=propertyAlias(name);var longhands=shorthandName(name)&&cssShorthand('longhands',name);if(longhands){var removed=false;$c.forEach(longhands,function(longhand){if($c.delete(record.values,longhand)){$c.splice(record.names,$c.indexOf(record.names,longhand),1);removed=true;}});if(removed)commitDeclaration(record);return '';}}var previous=$c.get(record.values,name);if(!previous)return '';$c.delete(record.values,name);$c.splice(record.names,$c.indexOf(record.names,name),1);commitDeclaration(record);return previous.text;}
  function setDeclarationShorthand(record,name,value,priority){
    if(priority&&priority!=='important')return;
    if($c.slice(name,0,2)!=='--')name=$c.toLowerCase(name);
    var entries=declarationEntries(name,value);
    if(!entries)return;
    $c.forEach(entries,function(entry){storeDeclaration(record,entry[0],entry[1],priority,entry[2],entry[3]);});
    commitDeclaration(record);
  }
  function setDeclarationLegacyParse(record,name,value,priority){
    var parsed={names:[],values:new Map(),fontFace:record.fontFace};
    parseDeclaration(parsed,name+': '+value+(priority?' !important':'')+';');
    var accepted=$c.get(parsed.values,name);
    if(!accepted&&record.fontFace&&knownProperty(name)){
      parsed.fontFace=false;
      parseDeclaration(parsed,name+': '+value+(priority?' !important':'')+';');
      accepted=$c.get(parsed.values,name);
    }
    return accepted;
  }
  function setDeclarationLegacy(record,name,value,priority){
    if(priority&&priority!=='important'||!record.fontFace&&!$c.startsWith(name,'--')&&!knownProperty(name))return;
    var accepted=setDeclarationLegacyParse(record,name,value,priority);
    if(!accepted)return;
    storeDeclaration(record,name,accepted.text,accepted.priority);
    commitDeclaration(record);
  }
  function setDeclaration(record,name,value,priority){
    writableDeclaration(record);
    name=cssString(name);value=value===null?'':cssString(value);priority=priority==null?'':$c.toLowerCase(cssString(priority));
    if(!value){removeDeclaration(record,name);return;}
    if(cssShorthand&&!record.fontFace){setDeclarationShorthand(record,name,value,priority);return;}
    setDeclarationLegacy(record,name,value,priority);
  }
  // Typed OM writes (StylePropertyMap.set / append): Chrome stores the CSSStyleValue, so a
  // longhand keeps the value's own serialization (calc() sums in their order, products,
  // percentages) instead of the specified-value spelling setProperty applies (turbo-dom#97).
  __ns.setDeclarationAuthored=function(style,name,value){var record=declarationRecord(style);writableDeclaration(record);name=propertyAlias(cssString(name));value=cssString(value);if(record.fontFace||!cssShorthand||$c.slice(name,0,2)==='--'||shorthandName(name)){setDeclaration(record,name,value,'');return;}var raw=parseLonghandText(name,value);if(raw===null)return;storeDeclaration(record,name,raw,'',null,raw);commitDeclaration(record);};
  // Typed OM reads reify the value as parsed (Chrome's expression tree: calc() terms in their
  // authored order), not the specified-value spelling getPropertyValue returns.
  __ns.declarationAuthoredValue=function(style,name){var record=declarationRecord(style);if(record.fontFace||!cssShorthand)return propertyValue(record,name);name=propertyAlias(cssString(name));var value=$c.get(record.values,name);if(value)return value.pending||!value.raw?value.text:colorText(name,value.raw);return shorthandName(name)?shorthandValue(record,name):'';};
  // getPropertyValue: a shorthand serializes from its longhands (a computed declaration's own
  // shorthand entry wins); a pending longhand reads ''.
  function propertyValueShorthand(record,name){name=propertyAlias(name);var value=$c.get(record.values,name);if(value)return value.text;var text=shorthandName(name)?shorthandValue(record,name):'';return !text&&record.defaultsOf?computedDefault(record,name):text;}
  function propertyValueLegacy(record,name){var value=$c.get(record.values,name);return value?value.text:record.defaultsOf?computedDefault(record,name):'';}
  function propertyValue(record,name){var used=record.computed?computedUsedSize(record,name):null;if(used!==null)return used;return !record.fontFace&&cssShorthand?propertyValueShorthand(record,name):propertyValueLegacy(record,name);}
  var declarationConstructor=__ns.makeNativeFunction(function(){throw new TypeError('Illegal constructor');},'CSSStyleDeclaration',0,true);
  declarationConstructor.prototype={};$O.defineProperty(declarationConstructor.prototype,'constructor',{value:declarationConstructor,writable:true,configurable:true});$O.defineProperty(declarationConstructor.prototype,Symbol.toStringTag,{value:'CSSStyleDeclaration',configurable:true});$O.defineProperty(declarationConstructor,'prototype',{writable:false});$O.defineProperty(g,'CSSStyleDeclaration',{value:declarationConstructor,writable:true,configurable:true,enumerable:false});
  var declarationPrototype=declarationConstructor.prototype;
  function declarationMember(name,get,set){var descriptor={enumerable:true,configurable:true,get:__ns.makeNativeFunction(get,'get '+name,0)};if(set)descriptor.set=__ns.makeNativeFunction(set,'set '+name,1);$O.defineProperty(declarationPrototype,name,descriptor);}
  declarationMember('length',function(){return declarationRecord(this).names.length;});
  declarationMember('parentRule',function(){return declarationRecord(this).parentRule||null;});
  declarationMember('cssFloat',function(){var record=declarationRecord(this),value=$c.get(record.values,'float');return value?value.text:record.defaultsOf?computedDefault(record,'float'):'';},function(value){setDeclaration(declarationRecord(this),'float',value,'');});
  declarationMember('cssText',function(){var record=declarationRecord(this);return record.computed?'':declarationText(record);},function(value){var record=declarationRecord(this);writableDeclaration(record);parseDeclaration(record,cssString(value));commitDeclaration(record);});
  var declarationMethods={getPropertyValue:function(name){if(!arguments.length)throw new TypeError('getPropertyValue requires 1 argument');if(!$c.get(declarationRecords,this))throw new TypeError('Illegal invocation');name=cssString(name);var record=declarationRecord(this);return propertyValue(record,name);},getPropertyPriority:function(name){if(!arguments.length)throw new TypeError('getPropertyPriority requires 1 argument');var record=declarationRecord(this);name=cssString(name);if(!record.fontFace&&cssShorthand){name=propertyAlias(name);var longhands=shorthandName(name)&&cssShorthand('longhands',name);if(longhands)return $c.every(longhands,function(longhand){var value=$c.get(record.values,longhand);return value&&value.priority==='important';})?'important':'';}var value=$c.get(record.values,name);return value?value.priority:'';},setProperty:function(name,value,priority){if(arguments.length<2)throw new TypeError('setProperty requires 2 arguments');setDeclaration(declarationRecord(this),name,value,priority);},removeProperty:function(name){if(!arguments.length)throw new TypeError('removeProperty requires 1 argument');var record=declarationRecord(this);writableDeclaration(record);return removeDeclaration(record,cssString(name));},item:function(index){if(!arguments.length)throw new TypeError('item requires 1 argument');return declarationRecord(this).names[Number(index)>>>0]||'';}};
  $c.forEach($O.keys(declarationMethods),function(name){$O.defineProperty(declarationPrototype,name,{value:__ns.makeNativeFunction(declarationMethods[name],name,name==='setProperty'?2:1),writable:true,enumerable:true,configurable:true});});
  $O.defineProperty(declarationPrototype,Symbol.iterator,{value:Array.prototype.values,writable:true,configurable:true});
  function declarationProperty(name,record){if(typeof name!=='string'||$c.startsWith(name,'--'))return null;var cssName=cssPropertyName(name);return knownProperty(cssName)||record&&record.fontFace&&$c.indexOf(['src','font-display','unicode-range','size-adjust','ascent-override','descent-override','line-gap-override'],cssName)>=0?cssName:null;}
  var cssDeclaration=function(text,owner,parentRule,readOnly,fontFace){var target=$O.create(declarationPrototype),record={__proto__:null,fontFace:!!fontFace,refresh:null,names:[],values:new Map(),owner:owner||null,parentRule:parentRule||null,source:String(text||''),readOnly:!!readOnly},proxy;parseDeclaration(record,record.source);proxy=new Proxy(target,{get:function(obj,key,receiver){var index=cssIndex(key);if(index>=0)return declarationRecord(proxy).names[index];if($R.has(obj,key))return $R.get(obj,key,receiver);var name=declarationProperty(key,record);return name?propertyValue(declarationRecord(proxy),name):undefined;},set:function(obj,key,value,receiver){if(cssIndex(key)>=0)return false;if($R.has(obj,key))return $R.set(obj,key,value,receiver);var name=declarationProperty(key,record);if(name){setDeclaration(declarationRecord(proxy),name,value,'');return true;}return $R.set(obj,key,value,receiver);},has:function(obj,key){return cssIndex(key)>=0?cssIndex(key)<declarationRecord(proxy).names.length:$R.has(obj,key)||!!declarationProperty(key,record);},ownKeys:function(){return $c.concat($c.map(declarationRecord(proxy).names,function(_,i){return String(i);}),$R.ownKeys(target));},getOwnPropertyDescriptor:function(obj,key){var index=cssIndex(key),names=declarationRecord(proxy).names;if(index>=0&&index<names.length)return {value:names[index],writable:false,enumerable:true,configurable:true};var descriptor=$R.getOwnPropertyDescriptor(obj,key);if(descriptor)return descriptor;if($R.has(declarationPrototype,key))return undefined;var name=declarationProperty(key,record);return name?{value:propertyValue(declarationRecord(proxy),name),writable:true,enumerable:true,configurable:true}:undefined;},deleteProperty:function(obj,key){return cssIndex(key)<0&&$R.deleteProperty(obj,key);}});$c.set(declarationRecords,proxy,record);return proxy;};
  var inlineDeclarationReceiver=__ns.domAccessor('nodeType',false,'Element');
  __ns.makeInlineStyleDeclaration=function(element){$c.call(inlineDeclarationReceiver,element);var style=$c.get(inlineDeclarations,element);if(!style){style=cssDeclaration($c.call(appliedInlineRead,element),element);$c.set(inlineDeclarations,element,style);}return style;};
  var computedStyleReader=g.getComputedStyle;
  __ns.cloneComputedStyleDeclaration=function(snapshot,element){var source=$c.get(declarationRecords,snapshot),style=cssDeclaration(''),record=$c.get(declarationRecords,style);record.values=source.values;record.names=source.names;record.readOnly=true;record.computed=true;record.fontState=source.fontState;record.defaultsOf=source.defaultsOf;record.defaultsParent=source.defaultsParent;record.refresh=function(){return computedStyleReader(element);};return style;};
  __ns.makeComputedStyleDeclaration=function(values,element,fontState,pseudo,defaultsParent){var style=cssDeclaration(''),record=$c.get(declarationRecords,style);$c.forEach($O.keys(values),function(name){var custom=$c.slice(name,0,2)==='--',cssName=custom?name:cssPropertyName(name),value=values[name];if(typeof value==='string'&&custom)value=$c.trim(value);if(typeof value==='string'&&value&&(custom||knownProperty(cssName)))storeDeclaration(record,cssName,value,'');});record.readOnly=true;record.computed=true;record.fontState=fontState;if(defaultsParent!==undefined){record.defaultsOf=element;record.defaultsParent=defaultsParent;}if(element)record.refresh=pseudo?function(){return computedStyleReader(element,'::'+pseudo);}:function(){return computedStyleReader(element);};return style;};
  var cssRuleStates=cssMap('rule');
  var cssRuleRecord=function(rule,type){var record=$c.get(cssRuleStates,rule);if(!record||type&&record.type!==type)throw new TypeError('Illegal invocation');return record;};
  function cssRuleInterface(name,parent){var ctor=__ns.makeNativeFunction(function(){throw new TypeError('Illegal constructor');},name,0,true);ctor.prototype=$O.create(parent||$O.prototype);$O.defineProperty(ctor.prototype,'constructor',{value:ctor,writable:true,configurable:true});$O.defineProperty(ctor.prototype,Symbol.toStringTag,{value:name,configurable:true});$O.defineProperty(ctor,'prototype',{writable:false});if(parent)$O.setPrototypeOf(ctor,parent.constructor);$O.defineProperty(g,name,{value:ctor,writable:true,configurable:true,enumerable:false});}
  cssRuleInterface('CSSRule');cssRuleInterface('CSSStyleRule',g.CSSRule.prototype);cssRuleInterface('CSSImportRule',g.CSSRule.prototype);cssRuleInterface('CSSNestedDeclarations',g.CSSRule.prototype);
  // Every CSSOM setter and mutating method drops reused computed styles (turbo-test#125).
  function cssInvalidating(fn){return function(){try{return cssApply(fn,this,arguments);}finally{invalidateComputedStyles();}};}
  function cssRuleMember(owner,name,get,set){var descriptor={enumerable:true,configurable:true,get:__ns.makeNativeFunction(get,'get '+name,0)};if(set)descriptor.set=__ns.makeNativeFunction(cssInvalidating(set),'set '+name,1);$O.defineProperty(g[owner].prototype,name,descriptor);}
  $c.forEach(['type','parentRule','parentStyleSheet'],function(name){cssRuleMember('CSSRule',name,function(){return cssRuleRecord(this)[name];});});
  // A style rule with nested cssRules (CSS Nesting, turbo-dom#358) serializes multi-line
  // using its authored order; the fallback puts declarations before children.
  function nestedStyleRuleText(record){
    var out=record.selectorText+' {\n';
    if(record.$orderOfPrint){
      for(var $oi=0;$oi<record.$orderOfPrint.length;$oi++){
        var $o=record.$orderOfPrint[$oi];
        out+='  '+($o.decl||cssRuleText(cssRuleRecord($o.child)))+'\n';
      }
    } else {
      var $decl=declarationText(declarationRecord(record.style));
      if($decl)out+='  '+$decl+'\n';
      var $list=cssItems(record.cssRules);
      for(var $ci=0;$ci<$list.length;$ci++)out+='  '+cssRuleText(cssRuleRecord($list[$ci]))+'\n';
    }
    return out+'}';
  }
  // Serialize a style rule (type 1) or an @import (type 5) — selector/prelude + the body text.
  function cssRuleTextStyle(record){
    var text = declarationText(declarationRecord(record.style));
    return (record.type === 1 ? record.selectorText : record.prelude) + ' {' + (text ? ' ' + text + ' ' : ' ') + '}';
  }
  // Serialize a grouping rule (@media / @supports / @container / …) or @keyframes (type 7),
  // listing one child rule per line like Chrome 156 (turbo-dom#134).
  function cssRuleTextGrouping(record){
    var rules = cssItems(record.cssRules);
    var out = (record.kind ? groupPrelude(record) : record.prelude) + (record.type === 7 ? ' { \n' : ' {\n');
    for (var index = 0; index < rules.length; index++) out += '  ' + cssRuleText(cssRuleRecord(rules[index])) + '\n';
    return out + '}';
  }
  function cssRuleTextIsNested(record){return record.type===1&&record.cssRules&&cssItems(record.cssRules).length;}
  function cssRuleText(record){
    if (record.kind === 'nested-declarations') return declarationText(declarationRecord(record.style));
    if (cssRuleTextIsNested(record)) return nestedStyleRuleText(record);
    if (record.type === 1 || record.type === 5) return cssRuleTextStyle(record);
    if ((record.grouping || record.type === 7) && record.cssRules) return cssRuleTextGrouping(record);
    return record.cssText;
  }
  __ns.readCssRuleText=function(rule){return cssRuleText(cssRuleRecord(rule));};
  cssRuleMember('CSSRule','cssText',function(){return cssRuleText(cssRuleRecord(this));},function(value){cssRuleRecord(this);if(typeof value==='symbol')throw new TypeError('Cannot convert a Symbol to a string');String(value);});
  cssRuleMember('CSSStyleRule','selectorText',function(){return cssRuleRecord(this,1).selectorText;},function(value){var record=cssRuleRecord(this,1);if(typeof value==='symbol')throw new TypeError('Cannot convert a Symbol to a string');var parsed=parseCssRules(String(value)+' {}',record.parentStyleSheet,record.parentRule);if(parsed.length===1&&cssRuleRecord(parsed[0]).type===1)record.selectorText=cssRuleRecord(parsed[0],1).selectorText;});
  cssRuleMember('CSSStyleRule','style',function(){return cssRuleRecord(this,1).style;},function(value){var record=cssRuleRecord(this,1);if(typeof value==='symbol')throw new TypeError('Cannot convert a Symbol to a string');record.style.cssText=String(value);});
  $c.forEach(['href','media','styleSheet'],function(name){cssRuleMember('CSSImportRule',name,function(){return cssRuleRecord(this,3)[name];});});
  // Grouping / condition rules (turbo-dom#254): Chrome 156's interface per at-rule. `kind`
  // is the at-keyword; the attributes read the (native-normalized) prelude.
  var groupingInterfaces={media:'CSSMediaRule',supports:'CSSSupportsRule',container:'CSSContainerRule',layer:'CSSLayerBlockRule','layer-statement':'CSSLayerStatementRule',scope:'CSSScopeRule','starting-style':'CSSStartingStyleRule'};
  cssRuleInterface('CSSGroupingRule',g.CSSRule.prototype);cssRuleInterface('CSSConditionRule',g.CSSGroupingRule.prototype);
  $c.forEach(['CSSContainerRule','CSSMediaRule','CSSSupportsRule'],function(name){cssRuleInterface(name,g.CSSConditionRule.prototype);});
  $c.forEach(['CSSLayerBlockRule','CSSScopeRule','CSSStartingStyleRule'],function(name){cssRuleInterface(name,g.CSSGroupingRule.prototype);});
  cssRuleInterface('CSSLayerStatementRule',g.CSSRule.prototype);
  var conditionKinds={media:1,supports:1,container:1};
  function groupRecord(rule,kinds){var record=$c.get(cssRuleStates,rule);if(!record||!record.kind||(kinds?!kinds[record.kind]:record.kind==='layer-statement'))throw new TypeError('Illegal invocation');return record;}
  function preludeRest(record){return $c.trim($c.replace(record.prelude,/^@[a-z-]+/i,''));}
  function mediaList(record){if(!record.mediaList)record.mediaList=makeMedia(preludeRest(record));return record.mediaList;}
  function containerParts(record){var rest=preludeRest(record),match=$c.exec(/^([^\s(]+)\s*([\s\S]*)$/,rest),parts=match&&$c.toLowerCase(match[1])!=='not'?[match[1],match[2]]:['',rest];parts[1]=containerQueryText(parts[1]);return parts;}
  // Size queries serialize like media features (Chrome: `(min-width: 200px)`); a query the
  // media grammar rejects (style(), scroll-state()) keeps its authored text.
  function containerQueryText(query){if(!query||typeof cssNativeParse!=='function')return query;var text=cssNativeParse(query,'media');return text&&text!=='not all'?text:query;}
  function scopeParts(record){var match=$c.exec(/^(?:\(([\s\S]*?)\))?\s*(?:to\s*\(([\s\S]*)\))?$/i,preludeRest(record));return match?[match[1]===undefined?null:$c.trim(match[1]),match[2]===undefined?null:$c.trim(match[2])]:[null,null];}
  function groupPrelude(record){if(record.kind==='media')return '@media '+conditionText(record);if(record.kind==='container')return '@container '+$c.trim($c.join(containerParts(record),' '));return record.prelude;}
  function conditionText(record){if(record.kind==='media')return $c.join(mediaRecord(mediaList(record)).items,', ');if(record.kind==='container')return $c.trim($c.join(containerParts(record),' '));return preludeRest(record);}
  cssRuleMember('CSSGroupingRule','cssRules',function(){return groupRecord(this).cssRules;});
  cssRuleMember('CSSConditionRule','conditionText',function(){return conditionText(groupRecord(this,conditionKinds));});
  cssRuleMember('CSSContainerRule','containerName',function(){return containerParts(groupRecord(this,{container:1}))[0];});
  cssRuleMember('CSSContainerRule','containerQuery',function(){return containerParts(groupRecord(this,{container:1}))[1];});
  cssRuleMember('CSSContainerRule','conditions',function(){var parts=containerParts(groupRecord(this,{container:1})),condition={};condition.name=parts[0];condition.query=parts[1];return [condition];});
  cssRuleMember('CSSMediaRule','media',function(){return mediaList(groupRecord(this,{media:1}));},function(value){mediaList(groupRecord(this,{media:1})).mediaText=value;});
  cssRuleMember('CSSLayerBlockRule','name',function(){return preludeRest(groupRecord(this,{layer:1}));});
  cssRuleMember('CSSLayerStatementRule','nameList',function(){return $c.map(cssParts(preludeRest(groupRecord(this,{'layer-statement':1})),','),function(name){return $c.trim(name);});});
  cssRuleMember('CSSScopeRule','start',function(){return scopeParts(groupRecord(this,{scope:1}))[0];});
  cssRuleMember('CSSScopeRule','end',function(){return scopeParts(groupRecord(this,{scope:1}))[1];});
  function insertGroupRuleIsImport(text,source,parsed){return $c.test(/^\s*@import\b/i,text)||source!==null&&$c.test(/^\s*@import\b/i,source)||parsed.length===1&&cssItems(parsed)[0].type===3;}
  function insertGroupRule(rule,record,text,index,interfaceName){var list=cssItems(record.cssRules),i=index===undefined?0:Number(index)>>>0;if(i>list.length)throw new __ns.DOMException("Failed to execute 'insertRule' on '"+(interfaceName||'CSSGroupingRule')+"': the index "+i+" must be less than or equal to the length of the rule list.",'IndexSizeError');text=cssString(text);var source=cssNativeParse(text,false,true),parsed=source===null?cssList():parseCssRules(source,record.parentStyleSheet,rule,true);if(insertGroupRuleIsImport(text,source,parsed))throw new __ns.DOMException('Imports are not allowed in a grouping rule','HierarchyRequestError');if(parsed.length!==1)throw new __ns.DOMException('Expected one CSS rule','SyntaxError');$c.splice(list,i,0,cssItems(parsed)[0]);return i;}
  function deleteGroupRule(record,index,interfaceName){var list=cssItems(record.cssRules),i=Number(index)>>>0;if(i>=list.length)throw new __ns.DOMException("Failed to execute 'deleteRule' on '"+(interfaceName||'CSSGroupingRule')+"': the index "+i+" must be less than the length of the rule list.",'IndexSizeError');detachCssRule(list[i]);$c.splice(list,i,1);}
  cssMethod(g.CSSGroupingRule.prototype,'insertRule',1,function(text,index){var record=groupRecord(this);if(!arguments.length)throw new TypeError("Failed to execute 'insertRule' on 'CSSGroupingRule': 1 argument required, but only 0 present.");return insertGroupRule(this,record,text,index);});
  cssMethod(g.CSSGroupingRule.prototype,'deleteRule',1,function(index){var record=groupRecord(this);if(!arguments.length)throw new TypeError("Failed to execute 'deleteRule' on 'CSSGroupingRule': 1 argument required, but only 0 present.");deleteGroupRule(record,index);});
  // turbo-dom#358: CSSStyleRule gains cssRules / insertRule / deleteRule (CSS Nesting). The
  // rule is a grouping rule too; insertRule / deleteRule reuse insertGroupRule /
  // deleteGroupRule, with the parent record coming from the style rule's own cssRuleStates entry.
  cssRuleMember('CSSStyleRule','cssRules',function(){var record=cssRuleRecord(this,1);if(!record.cssRules)record.cssRules=cssList();return record.cssRules;});
  // A bare declaration body becomes a CSSNestedDeclarations rule under the host style rule
  // (Chrome 156; turbo-dom#358). Returns the inserted index.
  function insertNestedDeclarations(ownerRule, record, list, text, i){
    var declText = $c.trim(text);
    if (declText[declText.length - 1] !== ';') declText += ';';
    var nested = { type:0, parentStyleSheet:record.parentStyleSheet, parentRule:ownerRule, prelude:'', kind:'nested-declarations', cssText:declText };
    nested.style = cssDeclaration(declText, null, nested);
    var nestedRule = finishCssRule(nested);
    cssRuleRecord(nestedRule).effectiveSelector = record.effectiveSelector || record.selectorText;
    $c.splice(list, i, 0, nestedRule);
    invalidateComputedStyles();
    return i;
  }
  // Rewrite a nested style rule's selector through the same normalization as parseCssRules
  // (prepend '& ' to a selector that doesn't start with '&').
  function nestedInsertRuleText(text, source){
    if (source === null) return text;
    var braceOpen = $c.indexOf(source, '{');
    if (braceOpen > 0 && source[0] !== '@') {
      var preludePart = $c.trim($c.slice(source, 0, braceOpen)), rest = $c.slice(source, braceOpen);
      return nestedSelectorList(preludePart) + ' ' + rest;
    }
    return source;
  }
  // After insertGroupRule returns, stamp the inserted rule's effectiveSelector.
  function finishInsertedNestedRule(record, inserted){
    if (!inserted) return;
    var ir = cssRuleRecord(inserted);
    if (ir.type === 1) ir.effectiveSelector = nestedCascadeSelector(ir.selectorText, record.effectiveSelector || record.selectorText);
  }
  cssMethod(g.CSSStyleRule.prototype,'insertRule',1,function(text,index){
    var record=cssRuleRecord(this,1);if(!record.cssRules)record.cssRules=cssList();
    if(!arguments.length)throw new TypeError("Failed to execute 'insertRule' on 'CSSStyleRule': 1 argument required, but only 0 present.");
    text=cssString(text);var i=index===undefined?0:Number(index)>>>0,list=cssItems(record.cssRules);
    if(i>list.length)throw new __ns.DOMException("Failed to execute 'insertRule' on 'CSSStyleRule': the index "+i+" must be less than or equal to the length of the rule list.",'IndexSizeError');
    if(!$c.test(/[{}]/,text))return insertNestedDeclarations(this,record,list,text,i);
    text=nestedInsertRuleText(text,cssNativeParse(text,false,true));
    var position=insertGroupRule(this,record,text,index,'CSSStyleRule');
    finishInsertedNestedRule(record,cssItems(record.cssRules)[position]);
    return position;
  });
  cssMethod(g.CSSStyleRule.prototype,'deleteRule',1,function(index){var record=cssRuleRecord(this,1);if(!record.cssRules)record.cssRules=cssList();if(!arguments.length)throw new TypeError("Failed to execute 'deleteRule' on 'CSSStyleRule': 1 argument required, but only 0 present.");deleteGroupRule(record,index,'CSSStyleRule');});
  cssRuleMember('CSSNestedDeclarations','style',function(){return cssRuleRecord(this).style;});
  var cssRuleConstants={STYLE_RULE:1,CHARSET_RULE:2,IMPORT_RULE:3,MEDIA_RULE:4,FONT_FACE_RULE:5,PAGE_RULE:6,KEYFRAMES_RULE:7,KEYFRAME_RULE:8,MARGIN_RULE:9,NAMESPACE_RULE:10,COUNTER_STYLE_RULE:11,SUPPORTS_RULE:12,FONT_FEATURE_VALUES_RULE:14};
  $c.forEach($O.keys(cssRuleConstants),function(name){$c.forEach([g.CSSRule,g.CSSRule.prototype],function(owner){$O.defineProperty(owner,name,{value:cssRuleConstants[name],enumerable:true,configurable:true});});});
  // turbo-dom#358 CSS Nesting. A style rule's body may hold both declarations and nested
  // rules. splitRuleBody returns a list of {decl} | {rule:{head,body}} parts in order.
  function splitRuleBody(text){
    var parts=[],start=0,i=0,open;
    while((open=scanToTopBrace(text,i))>=0){
      var state={at:open,depth:1,quote:'',escaped:false,comment:false};
      while(state.at<text.length-1){state.at++;if(scanStep(text,state,state.at))break;}
      splitRuleBodyPush(parts,text,start,open,state.at);
      i=start=state.at+1;
    }
    var tail=$c.trim($c.slice(text,start));
    if(tail)$c.push(parts,{decl:tail});
    return parts;
  }
  // Push a {decl} for any declarations between `start` and the rule's prelude, then the
  // {rule} for the block [open, close].
  function splitRuleBodyPush(parts,text,start,open,close){
    var preludeStart=start;
    for(var j=open-1;j>=start;j--)if(text[j]===';'){preludeStart=j+1;break;}
    if(preludeStart>start){var declRun=$c.trim($c.slice(text,start,preludeStart));if(declRun)$c.push(parts,{decl:declRun});}
    var head=$c.trim($c.slice(text,preludeStart,open)),body=$c.slice(text,open+1,close);
    if(head)$c.push(parts,{rule:{head:head,body:body}});
  }
  // Return the position of the first `{` at depth 0, skipping comments/strings; or -1.
  function scanToTopBrace(text,from){
    var state={at:from-1,depth:0,quote:'',escaped:false,comment:false};
    while(state.at<text.length-1){state.at++;var c=text[state.at];if(skipStateChar(state,c,text[state.at+1]))continue;if(c==='{')return state.at;}
    return -1;
  }
  // One step of the brace scanner; returns true when the state closed back to depth 0.
  function scanStep(text,state,i){
    var c=text[i];if(skipStateChar(state,c,text[i+1]))return false;
    if(c==='{')state.depth++;
    else if(c==='}'&&--state.depth===0)return true;
    return false;
  }
  // Shared lex-state transitions (comments, quotes, escapes); returns true when consumed.
  function skipStateCharQuote(state,c){if(state.escaped)state.escaped=false;else if(c==='\\')state.escaped=true;else if(c===state.quote)state.quote='';return true;}
  function skipStateChar(state,c,next){
    if(state.comment){if(c==='*'&&next==='/'){state.comment=false;state.at++;}return true;}
    if(state.quote)return skipStateCharQuote(state,c);
    if(c==='/'&&next==='*'){state.comment=true;state.at++;return true;}
    if(c==='"'||c==="'"){state.quote=c;return true;}
    return false;
  }
  // Chrome 156 prepends '& ' to a nested selector that doesn't start with '&'.
  // Chrome 156: a nested style rule's cascade selector is the child's selector with each
  // standalone `&` replaced by `:is(<parent>)`, and (for a selector that doesn't start with
  // `&`) `:is(<parent>) ` prepended. We approximate the `&`-replacement with a text scan.
  function nestedReplaceAmpersand(trimmed,parentIs){
    var out='',quote='',escaped=false;
    for(var i=0;i<trimmed.length;i++){
      var c=trimmed[i];
      if(quote){out+=c;if(escaped)escaped=false;else if(c==='\\')escaped=true;else if(c===quote)quote='';continue;}
      if(c==='"'||c==="'"){out+=c;quote=c;continue;}
      out+=c==='&'?parentIs:c;
    }
    return out;
  }
  function nestedCascadeSelector(childSelector, parentEffective){
    if(!parentEffective)return childSelector;
    var parentIs=':is('+parentEffective+')';
    return $c.join($c.map(cssParts(childSelector,','),function(item){
      var trimmed=$c.trim(item);
      return trimmed[0]!=='&'?parentIs+' '+trimmed:nestedReplaceAmpersand(trimmed,parentIs);
    }),', ');
  }
  function nestedSelectorList(selectors){
    return $c.join($c.map(cssParts(selectors,','),function(item){
      var trimmed=$c.trim(item);
      if(trimmed[0]==='&')return trimmed;
      return '& '+trimmed;
    }),', ');
  }
  function finishCssRuleProto(record){var kind=record.kind&&groupingInterfaces[record.kind];if(kind)return g[kind].prototype;if(record.kind==='nested-declarations')return g.CSSNestedDeclarations.prototype;if(record.type===1)return g.CSSStyleRule.prototype;if(record.type===3)return g.CSSImportRule.prototype;return g.CSSRule.prototype;}
  function finishCssRule(rule){var record={__proto__:null};$c.forEach(['type','parentRule','parentStyleSheet','cssText','selectorText','href','media','styleSheet','prelude','grouping','kind','$orderOfPrint'],function(name){if($O.hasOwn(rule,name)){record[name]=rule[name];delete rule[name];}});if(rule.style){record.style=rule.style;if(record.type===1)delete rule.style;}if(rule.cssRules){record.cssRules=rule.cssRules;if(record.kind)delete rule.cssRules;}$c.set(cssRuleStates,rule,record);$O.setPrototypeOf(rule,finishCssRuleProto(record));return rule;}
  // A rule object about to be finished keeps its type/parentRule on the proxy itself;
  // a finished rule carries them in its record. Walk via whichever is present.
  // Build a style rule from its head and body (CSS Nesting splits it into decls + rules).
  function fillStyleRule(rule,head,body,sheet){
    rule.type=1;rule.selectorText=cssPrelude(head,true);
    var nestedParts=splitRuleBody(body),hasNested=false;
    for(var $ni=0;$ni<nestedParts.length;$ni++)if(nestedParts[$ni].rule){hasNested=true;break;}
    if(!hasNested){rule.style=cssDeclaration(body,null,rule);rule.cssText=head+' { '+$c.trim(body)+' }';}
    else fillNestedStyleRule(rule,head,sheet,nestedParts);
  }
  // Build an at-rule from its head and body (@media / @supports / @font-face / ...).
  function fillAtRuleType(head){if($c.test(/^@media\b/i,head))return 4;if($c.test(/^@supports\b/i,head))return 12;if($c.test(/^@font-face\b/i,head))return 5;if($c.test(/^@(-webkit-)?keyframes\b/i,head))return 7;return 0;}
  function fillAtRule(rule,head,body,sheet,parent){
    rule.prelude=head;
    rule.type=fillAtRuleType(head);
    rule.grouping=$c.test(/^@(media|supports|container|scope|starting-style|layer)\b/i,head);
    if(rule.grouping)rule.kind=$c.toLowerCase($c.exec(/^@([a-z-]+)/i,head)[1]);
    rule.cssText=head+($c.trim(body)?' { '+$c.trim(body)+' }':' { }');
    if(rule.type===5)rule.style=cssDeclaration(body,null,rule,false,true);
    else if(!$c.test(/^@(property|view-transition)\b/i,head))
      rule.cssRules = parent && rule.grouping && hasStyleRuleAncestor(parent) ? parseNestedGroupBody(body,sheet,rule) : parseCssRules(body,sheet,rule,true);
  }
  // turbo-dom#358 CSS Nesting. Build a style rule whose body holds both declarations and
  // nested rules. Merges the parent-level declarations into rule.style, builds its cssRules
  // (CSSNestedDeclarations for a decl run that follows a nested rule), computes the cascade
  // effectiveSelector down the nested tree, and writes the authored multi-line cssText.
  function fillNestedStyleRule(rule,head,sheet,nestedParts){
    var ownDecls='',children=cssList(),seenRule=false,order=[];
    for(var $pi=0;$pi<nestedParts.length;$pi++){
      var $part=nestedParts[$pi];
      if($part.decl){
        if(ownDecls)ownDecls+=' ';ownDecls+=$part.decl;
        if(seenRule){var nestedRule=makeCssNestedDeclarationsRule(sheet,rule,$part.decl);$c.push(cssItems(children),nestedRule);$c.push(order,{child:nestedRule});}
        else $c.push(order,{decl:$part.decl});
      } else {
        seenRule=true;
        var childRule=parseOneNestedChild(sheet,rule,$part.rule.head,$part.rule.body);
        if(childRule){$c.push(cssItems(children),childRule);$c.push(order,{child:childRule});}
      }
    }
    rule.style=cssDeclaration(ownDecls,null,rule);
    rule.cssRules=children;
    rule.$orderOfPrint=order;
    propagateNestedCascade(rule.selectorText,children);
    var $text=head+' {\n';
    for(var $oi=0;$oi<order.length;$oi++){var $o=order[$oi];$text+='  '+($o.decl||cssRuleText(cssRuleRecord($o.child)))+'\n';}
    rule.cssText=$text+'}';
  }
  // Build a CSSNestedDeclarations rule for a bare declaration run.
  function makeCssNestedDeclarationsRule(sheet,parent,declText){
    var nested={type:0,parentStyleSheet:sheet,parentRule:parent,prelude:'',kind:'nested-declarations',cssText:declText};
    nested.style=cssDeclaration(declText,null,nested);
    return finishCssRule(nested);
  }
  // Parse one nested rule inside a style rule's body: an at-rule keeps its head; any other
  // selector gets '& ' prepended if it doesn't start with '&' (Chrome 156).
  function parseOneNestedChild(sheet,parent,head,body){
    var nestedText=head[0]==='@'?head+' { '+$c.trim(body)+' }':nestedSelectorList(head)+' { '+$c.trim(body)+' }';
    var items=cssItems(parseCssRules(nestedText,sheet,parent,true));
    return items.length?items[0]:null;
  }
  // Propagate the cascade effective selector down nested cssRules (CSSStyleRule gets
  // `&` resolved against the parent's effective selector; CSSNestedDeclarations inherit it).
  function propagateNestedCascade(parentSel,list){
    var items=cssItems(list);
    for(var $ii=0;$ii<items.length;$ii++){
      var $r=cssRuleRecord(items[$ii]);
      if($r.kind==='nested-declarations'){$r.effectiveSelector=parentSel;continue;}
      if($r.type===1){$r.effectiveSelector=nestedCascadeSelector($r.selectorText,parentSel);if($r.cssRules)propagateNestedCascade($r.effectiveSelector,$r.cssRules);}
      else if($r.kind){if($r.cssRules)propagateNestedCascade(parentSel,$r.cssRules);}
    }
  }
  function hasStyleRuleAncestor(rule){
    while(rule){
      var record=$c.get(cssRuleStates,rule),type=record?record.type:rule.type,next=record?record.parentRule:rule.parentRule;
      if(type===1)return true;
      rule=next;
    }
    return false;
  }
  // turbo-dom#358: a grouping at-rule nested inside a style rule admits bare
  // declarations (CSSNestedDeclarations). Walk its body with splitRuleBody, emitting a
  // CSSNestedDeclarations rule for each bare run and parsing each nested rule.
  function parseNestedGroupBody(body,sheet,parent){
    var out=cssList();
    var parts=splitRuleBody(body);
    for(var $pi=0;$pi<parts.length;$pi++){
      var part=parts[$pi];
      if(part.decl){
        var declText=part.decl;if(declText[declText.length-1]!==';')declText+=';';
        var nested={type:0,parentStyleSheet:sheet,parentRule:parent,prelude:'',kind:'nested-declarations',cssText:declText};
        nested.style=cssDeclaration(declText,null,nested);
        $c.push(cssItems(out),finishCssRule(nested));
      } else {
        var head2=part.rule.head,body2=part.rule.body,nestedText;
        if(head2[0]==='@')nestedText=head2+' { '+$c.trim(body2)+' }';
        else nestedText=nestedSelectorList(head2)+' { '+$c.trim(body2)+' }';
        var childRules=cssItems(parseCssRules(nestedText,sheet,parent,true));
        if(childRules.length)$c.push(cssItems(out),childRules[0]);
      }
    }
    return out;
  }
  // parseCssRules' comment / string state: cssScanSkip steps over them and returns how many
  // characters it consumed (0 when text[i] is outside both).
  function CssRuleScan(){this.quote='';this.escaped=false;this.comment=false;}
  function cssScanSkipComment(scan,c,next){if(c==='*'&&next==='/'){scan.comment=false;return 2;}return 1;}
  function cssScanSkipQuote(scan,c){if(scan.escaped)scan.escaped=false;else if(c==='\\')scan.escaped=true;else if(c===scan.quote)scan.quote='';return 1;}
  function cssScanSkip(scan,c,next){
    if(scan.comment)return cssScanSkipComment(scan,c,next);
    if(scan.quote)return cssScanSkipQuote(scan,c);
    if(c==='/'&&next==='*'){scan.comment=true;return 2;}
    if(c==='"'||c==="'"){scan.quote=c;return 1;}
    return 0;
  }
  // A `head { body }` block: an at-rule or a style rule.
  function cssPushBlockRule(out,text,start,open,close,sheet,parent){
    var head=cssPrelude($c.slice(text,start,open),false),body=$c.slice(text,open+1,close);
    if(!head)return;
    var rule={parentStyleSheet:sheet,parentRule:parent||null};
    if(head[0]==='@')fillAtRule(rule,head,body,sheet,parent);
    else fillStyleRule(rule,head,body,sheet);
    $c.push(cssItems(out),finishCssRule(rule));
  }
  // A top-level `;` statement: @import or an @layer statement.
  function cssPushStatementRule(out,statement,sheet,parent){
    var imported=$c.exec(/^@import\s+(?:url\(\s*)?["']([^"']+)["']\s*\)?\s*(.*)$/i,statement);
    if(imported){var rule={};rule.type=3;rule.href=imported[1];rule.media=makeMedia(imported[2]||'');rule.styleSheet=null;rule.parentStyleSheet=sheet;rule.parentRule=parent||null;rule.cssText=statement+';';$c.push(cssItems(out),finishCssRule(rule));}
    else if($c.test(/^@layer\s/i,statement)){var layer={};layer.type=0;layer.kind='layer-statement';layer.prelude=statement;layer.parentStyleSheet=sheet;layer.parentRule=parent||null;layer.cssText=statement+';';$c.push(cssItems(out),finishCssRule(layer));}
  }
  // One brace/semicolon dispatch step for parseCssRules. `state` holds the running
  // depth / open-brace position / block start (all three change per rule boundary).
  function parseCssRulesStep(out, text, i, c, state, sheet, parent){
    if (c === '{') { if (state.depth++ === 0) state.open = i; return; }
    if (c === '}' && state.depth > 0 && --state.depth === 0) {
      cssPushBlockRule(out, text, state.start, state.open, i, sheet, parent);
      state.start = i + 1; return;
    }
    if (c === ';' && state.depth === 0) {
      cssPushStatementRule(out, $c.trim($c.slice(text, state.start, i)), sheet, parent);
      state.start = i + 1;
    }
  }
  var parseCssRules = function(text, sheet, parent, validated){
    text = String(text);
    if (!validated && typeof cssNativeParse === 'function') text = cssNativeParse(text);
    var out = cssList(), state = { depth: 0, open: -1, start: 0 }, scan = new CssRuleScan();
    for (var i = 0; i < text.length; i++) {
      var c = text[i], next = i + 1 < text.length ? text[i + 1] : undefined;
      var skip = cssScanSkip(scan, c, next);
      if (skip) { i += skip - 1; continue; }
      parseCssRulesStep(out, text, i, c, state, sheet, parent);
    }
    return out;
  };
  var sheetRecords=cssMap('sheet'),mediaRecords=cssMap('media'),elementSheets=cssMap('elementSheet');
  function sheetRecord(sheet){var record=$c.get(sheetRecords,sheet);if(!record)throw new TypeError('Illegal invocation');return record;}
  function mediaRecord(media){var record=$c.get(mediaRecords,media);if(!record)throw new TypeError('Illegal invocation');if(record.owner){var text=sheetNativeAttribute(record.owner,'media')||'';if(text!==record.source){record.source=text;record.items=mediaQueries(text);}}return record;}
  function cssInterface(name,parent,construct){var ctor=__ns.makeNativeFunction(construct||function(){throw new TypeError('Illegal constructor');},name,0,true);ctor.prototype=$O.create(parent?parent.prototype:$O.prototype);$O.defineProperty(ctor.prototype,'constructor',{value:ctor,writable:true,configurable:true});$O.defineProperty(ctor.prototype,Symbol.toStringTag,{value:name,configurable:true});$O.defineProperty(ctor,'prototype',{writable:false});if(parent)$O.setPrototypeOf(ctor,parent);$O.defineProperty(g,name,{value:ctor,writable:true,configurable:true,enumerable:false});return ctor;}
  function cssMember(proto,name,get,set){var descriptor={enumerable:true,configurable:true,get:__ns.makeNativeFunction(get,'get '+name,0)};if(set)descriptor.set=__ns.makeNativeFunction(cssInvalidating(set),'set '+name,1);$O.defineProperty(proto,name,descriptor);}
  function cssMethod(proto,name,length,method){$O.defineProperty(proto,name,{value:__ns.makeNativeFunction(name==='item'||name==='toString'?method:cssInvalidating(method),name,length),enumerable:true,writable:true,configurable:true});}
  function mediaQueries(text){text=$c.trim(cssString(text));if(!text)return [];return $c.map(cssParts(text,','),function(query){query=$c.trim(query);return cssNativeParse(query,'media');});}
  var mediaConstructor=cssInterface('MediaList');
  cssMember(mediaConstructor.prototype,'length',function(){return mediaRecord(this).items.length;});
  cssMember(mediaConstructor.prototype,'mediaText',function(){return $c.join(mediaRecord(this).items,', ');},function(text){mediaRecord(this).items=mediaQueries(text);});
  cssMethod(mediaConstructor.prototype,'item',1,function(index){if(!arguments.length)throw new TypeError('item requires 1 argument');return mediaRecord(this).items[Number(index)>>>0]||null;});
  cssMethod(mediaConstructor.prototype,'appendMedium',1,function(text){var record=mediaRecord(this);if(!arguments.length)throw new TypeError('appendMedium requires 1 argument');var queries=mediaQueries(text);if(queries.length!==1)return;if($c.indexOf(record.items,queries[0])<0)$c.push(record.items,queries[0]);});
  cssMethod(mediaConstructor.prototype,'deleteMedium',1,function(text){var record=mediaRecord(this);if(!arguments.length)throw new TypeError('deleteMedium requires 1 argument');var query=$c.join(mediaQueries(text),', '),index=$c.indexOf(record.items,query);if(index<0)throw new __ns.DOMException('The medium was not found','NotFoundError');$c.splice(record.items,index,1);});
  cssMethod(mediaConstructor.prototype,'toString',0,function(){return $c.join(mediaRecord(this).items,', ');});
  $O.defineProperty(mediaConstructor.prototype,Symbol.iterator,{value:Array.prototype.values,writable:true,configurable:true});
  function makeMedia(text,owner){var target=$O.create(mediaConstructor.prototype),record={items:mediaQueries(text||''),source:text||'',owner:owner||null},proxy;proxy=new Proxy(target,{get:function(obj,key,receiver){var index=cssIndex(key);return index>=0?mediaRecord(proxy).items[index]:$R.get(obj,key,receiver);},has:function(obj,key){var index=cssIndex(key);return index>=0?index<mediaRecord(proxy).items.length:$R.has(obj,key);},ownKeys:function(){return $c.concat($c.map(mediaRecord(proxy).items,function(_,i){return String(i);}),$R.ownKeys(target));},getOwnPropertyDescriptor:function(obj,key){var index=cssIndex(key),items=mediaRecord(proxy).items;return index>=0&&index<items.length?{value:items[index],writable:false,enumerable:true,configurable:true}:$R.getOwnPropertyDescriptor(obj,key);},set:function(obj,key,value,receiver){return cssIndex(key)<0&&$R.set(obj,key,value,receiver);},deleteProperty:function(obj,key){return cssIndex(key)<0&&$R.deleteProperty(obj,key);}});$c.set(mediaRecords,proxy,record);return proxy;}
  var styleSheetConstructor=cssInterface('StyleSheet');
  function newSheetRecord(options){return {rules:cssList(),constructed:true,constructorDocument:d,ownerNode:null,href:null,parentStyleSheet:null,ownerRule:null,title:null,media:makeMedia(options&&options.media!==undefined?options.media:''),disabled:!!(options&&options.disabled),replacing:false,originClean:true};}
  var sheetConstructor=cssInterface('CSSStyleSheet',styleSheetConstructor,function(options){if(options!==null&&options!==undefined&&typeof options!=='object'&&typeof options!=='function')throw new TypeError('CSSStyleSheetInit must be an object');$c.set(sheetRecords,this,newSheetRecord(options));});
  $c.forEach(['href','ownerNode','parentStyleSheet'],function(name){cssMember(styleSheetConstructor.prototype,name,function(){return sheetRecord(this)[name];});});
  cssMember(styleSheetConstructor.prototype,'title',function(){var record=sheetRecord(this);return record.ownerNode?sheetNativeAttribute(record.ownerNode,'title')||null:record.title;});
  cssMember(styleSheetConstructor.prototype,'type',function(){sheetRecord(this);return 'text/css';});
  cssMember(styleSheetConstructor.prototype,'media',function(){return sheetRecord(this).media;},function(value){mediaRecord(sheetRecord(this).media).items=mediaQueries(value);});
  cssMember(styleSheetConstructor.prototype,'disabled',function(){return sheetRecord(this).disabled;},function(value){sheetRecord(this).disabled=!!value;});
  cssMember(sheetConstructor.prototype,'ownerRule',function(){return sheetRecord(this).ownerRule;});
  // The style rules of `sheet` (and its @import sheets) that apply, each with its grouping
  // context (turbo-dom#273): children of @media (matching), @supports (true), @layer and
  // @scope rules apply; @container (no size container is ever established here) and
  // @starting-style (steady state) children never do. `layers` is the cascade-layer tree
  // shared by every sheet of one computation; `el` checks @supports selector() conditions.
  function cssLayerChild(node,name){var child=name===null?null:node.named[name];if(!child){child={__proto__:null,named:$O.create(null),children:[],rank:0};if(name!==null)node.named[name]=child;$c.push(node.children,child);}return child;}
  function cssLayerPath(node,name){var parts=cssParts(name,'.');for(var index=0;index<parts.length;index++)node=cssLayerChild(node,$c.trim(parts[index]));return node;}
  function cssLayerTree(){return {__proto__:null,named:$O.create(null),children:[],rank:0,root:true};}
  // An entry's layer rank; an unlayered entry kept from an earlier build (computedSheetEntries)
  // holds that build's root, so every root ranks as the current one.
  function cssLayerRank(layer,layers){return layer.root?layers.rank:layer.rank;}
  // Sub-layers precede their layer's own rules; the root (unlayered rules) comes last.
  function cssLayerRanks(root){var next=0;(function visit(node){for(var index=0;index<node.children.length;index++)visit(node.children[index]);node.rank=next++;})(root);}
  var cssSupportsCache=$O.create(null);
  function cssSupportsDeclaration(inner){var parts=cssParts(inner,':'),name=$c.trim(parts[0]),record={__proto__:null};if(parts.length<2||!name||cssParts(inner,';').length>1)return false;parseDeclaration(record,name+': '+$c.trim($c.slice(inner,parts[0].length+1)));return record.names.length>0;}
  // turbo-dom#390: `selector()` takes one <complex-selector>, not a list, and Chrome parses
  // its forgiving `:is()` / `:where()` arguments non-forgivingly (`:is(:bogus)` and
  // `:where(:bogus, p)` make the whole `selector()` false). cssSupportsForgivingChecks walks
  // every `:is()` / `:where()` and validates each comma-split argument as a selector on its
  // own; nativeMatches throws SyntaxError on an unknown pseudo outside the forgiving wrap.
  function cssSupportsForgivingArg(piece,el){if(!piece)return true;try{cssApply(computedStyleNativeMatches,el,[piece]);}catch(e){return false;}return cssSupportsSelectorStrict(piece,el);}
  function cssSupportsForgivingChecks(sel,el){
    var re=/:(?:is|where)\(/gi, m;
    while((m=$c.exec(re,sel))){
      var open=m.index+m[0].length-1, depth=1, j=open+1;
      while(j<sel.length&&depth>0){var cc=sel[j];if(cc==='(')depth++;else if(cc===')')depth--;j++;}
      if(depth!==0)return false;
      var pieces=cssParts($c.slice(sel,open+1,j-1),',');
      for(var k=0;k<pieces.length;k++)if(!cssSupportsForgivingArg($c.trim(pieces[k]),el))return false;
      re.lastIndex=j;
    }
    return true;
  }
  function cssSupportsSelectorStrict(sel,el){
    if(cssParts(sel,',').length>1)return false;
    try{cssApply(computedStyleNativeMatches,el,[sel]);}catch(e){return false;}
    return cssSupportsForgivingChecks(sel,el);
  }
  function cssSupportsTerm(term,el){
    if(term[0]==='('&&term[term.length-1]===')'){var inner=$c.trim($c.slice(term,1,term.length-1));return inner[0]==='('||$c.test(/^not\s/i,inner)?cssSupportsCondition(inner,el):cssSupportsDeclaration(inner);}
    var selector=$c.exec(/^selector\(([\s\S]*)\)$/i,term);if(!selector)return false;
    return cssSupportsSelectorStrict($c.trim(selector[1]),el);
  }
  // A <supports-condition>: `not <term>`, or terms joined by one of `and` / `or`.
  function cssSupportsConditionStep(tokens,index,operator,el){if($c.toLowerCase(tokens[index])!==operator||operator!=='and'&&operator!=='or'||index+1>=tokens.length)return undefined;return cssSupportsTerm(tokens[index+1],el);}
  function cssSupportsCondition(text,el){
    var tokens=$c.filter(cssParts($c.trim(text),' '),function(token){return token!=='';});
    if(tokens.length===2&&$c.toLowerCase(tokens[0])==='not')return !cssSupportsTerm(tokens[1],el);
    var value=cssSupportsTerm(tokens[0],el),operator=tokens.length>1?$c.toLowerCase(tokens[1]):'';
    for(var index=1;index<tokens.length;index+=2){var next=cssSupportsConditionStep(tokens,index,operator,el);if(next===undefined)return false;value=operator==='and'?value&&next:value||next;}
    return value;
  }
  // CSS.supports() for an embedder's CSS namespace, answered by the grammar this engine's
  // cascade and @supports use (turbo-dom#322): one argument is a <supports-condition> or a bare
  // declaration; two are a property name (no surrounding whitespace) and a value (no
  // !important). selector() conditions are checked against the document element.
  function cssSupportsScope(){var root=g.document&&g.document.documentElement;return root||g.document.createElement('div');}
  __ns.cssSupports=function(text){text=$c.trim($String(text));var el=cssSupportsScope();return cssSupportsCondition(text,el)||cssSupportsCondition('('+text+')',el);};
  __ns.cssSupportsProperty=function(property,value){property=$String(property);value=$String(value);if(!$c.test(/^(--)?[^\s:;!{}()]+$/,property)||$c.test(/!\s*important\s*$/i,value)||$c.slice(property,0,2)!=='--'&&cssParts(value,';').length>1)return false;return cssSupportsDeclaration(property+': '+value);};
  // A condition is fixed for the engine, so it is evaluated once per text.
  function cssSupportsMatches(text,el){var cached=cssSupportsCache[text];if(cached===undefined)cached=cssSupportsCache[text]=cssSupportsCondition(text,el);return cached;}
  // 0: unconditional; 1: under @media that matches now; 2: under @media that does not.
  // The media conditions the current computation evaluated, with their answers (null: not
  // tracking): a reused computed style is reused only while each still answers the same.
  var computedMediaReads=null;
  function cssMediaStateJsdom(record,state){return state===0&&jsdomScreen(preludeRest(record))?1:2;}
  function cssMediaState(record,state){if(mediaMode==='jsdom')return cssMediaStateJsdom(record,state);var text=conditionText(record);if(state===2||text==='not all')return 2;if(text===''||text==='all')return state;var matches=mediaMatches(text);if(computedMediaReads&&computedMediaReads[text]===undefined)computedMediaReads[text]=matches;return matches?1:2;}
  // The rule entries of a sheet list (its style rules under their grouping rules, the layer
  // tree they rank in, the @media answers they were built with): built once per list, CSSOM
  // epoch and media answers, and shared by every element computed meanwhile. The build reads
  // no element (an @supports answer is cached by text). A build that went volatile (an
  // @import under a media query) is not kept.
  var computedEntryBuilds=new computedSnapshotMap();
  function computedSheetEntriesCached(build,epoch,media){return build&&build.epoch===epoch&&computedMediaUnchanged(build.media);}
  function computedSheetEntries(sheets,el){
    var epoch=cssomEpoch(),build=computedSnapshotGet(computedEntryBuilds,sheets);
    if(computedSheetEntriesCached(build,epoch)){computedMergeMedia(build.media);return build;}
    if(noteEntryBuild&&sheets.length)noteEntryBuild();
    var outerMedia=computedMediaReads,outerVolatile=computedVolatile,media={__proto__:null},layers=cssLayerTree(),lists=[];
    computedMediaReads=media;computedVolatile=false;
    var volatile=false;
    try{var seen=new Set();for(var si=0;si<sheets.length;si++)$c.push(lists,computedOneSheetEntries(sheets[si],el,layers,seen));volatile=computedVolatile;}
    finally{computedMediaReads=outerMedia;computedVolatile=outerVolatile||volatile;computedMergeMedia(media);}
    build={__proto__:null,epoch:epoch,media:media,layers:layers,lists:lists};
    if(!volatile&&sheets.length)computedSnapshotSet(computedEntryBuilds,sheets,build);
    return build;
  }
  // One sheet's rule entries, kept across entry builds (an emotion test run adds a <style> per
  // rule, so every build would otherwise re-walk every earlier sheet) while no CSSOM change but a
  // declaration write happened and its @media answers hold. Only a sheet of unlayered rules
  // without @import is kept: a layer joins the build's layer tree, an import the build's seen set.
  var computedSheetCache=new computedSnapshotMap();
  // Can this sheet's entries be reused on the next cascade: no volatile reads, no new
  // @layer joins, no @import, every entry still in the active layer tree.
  function computedSheetKeepable(went, layers, layersBefore, rules, list){
    if (went || layers.children.length !== layersBefore) return false;
    for (var at = 0; at < rules.length; at++) if (cssRuleRecord(rules[at]).type === 3) return false;
    for (var index = 0; index < list.length; index++) if (list[index].layer !== layers) return false;
    return true;
  }
  function computedOneSheetEntries(sheet,el,layers,seen){
    if($c.has(seen,sheet))return [];
    var kept=computedSnapshotGet(computedSheetCache,sheet);
    var structure=cssomStructureEpoch();
    if(kept&&kept.epoch===structure&&computedMediaUnchanged(kept.media)){$c.add(seen,sheet);computedMergeMedia(kept.media);return kept.list;}
    var outer=computedMediaReads,media={__proto__:null},volatile=computedVolatile,layersBefore=layers.children.length,list;
    computedMediaReads=media;computedVolatile=false;
    try{list=appliedSheetEntries(sheet,el,layers,undefined,seen);}
    finally{var went=computedVolatile;computedMediaReads=outer;computedVolatile=volatile||went;computedMergeMedia(media);}
    var keep=computedSheetKeepable(went,layers,layersBefore,cssItems(sheetRecord(sheet).rules),list);
    computedSnapshotSet(computedSheetCache,sheet,keep?{__proto__:null,epoch:structure,media:media,list:list}:null);
    return list;
  }
  // Rule buckets (turbo-dom#307): a style rule can select an element only when, for some
  // alternative of its selector list, the element carries every class and id named at the top
  // level of that alternative's rightmost compound. Each entry keeps one such name per
  // alternative ('.a' / '#b', ASCII- and Unicode-lowercased, so quirks mode's case-insensitive
  // class and id matching is never under-approximated), or none when some alternative names
  // none or the scan cannot read it. Each entry build indexes its entries by those names once,
  // and a cascade visits, in document order, only the unkeyed entries and those keyed on a name
  // the element carries. Skipping only drops rules that cannot match, in any live state (relaxing a state
  // pseudo-class keeps the names), so the matched list, its order and every volatility read
  // are unchanged. Names inside functional pseudo-classes (:is, :not, :has, :host(), ...) are
  // never keys; :host / ::slotted / ::part compounds, nesting (&), namespaces (|) and escapes
  // leave the rule unkeyed.
  // The build's bucket index, made on first use: every entry by its position in the build
  // (sheet by sheet, in order), the sheet it came from, the positions of the unkeyed entries and,
  // per name, the positions of the entries keyed on it (all ascending).
  // The last index made is extended instead when its build's lists are a prefix of the new
  // one's (a sheet appended: an emotion test run adds a <style> per rule); the earlier build
  // then drops it and indexes afresh if it is used again.
  var cssLastBuckets=null;
  function cssBucketReuseStart(build){
    var last=cssLastBuckets;
    if(!last||last.build.buckets!==last.index||last.build.lists.length>=build.lists.length)return 0;
    var start=last.build.lists.length;
    for(var at=0;at<start;at++)if(last.build.lists[at]!==build.lists[at])return 0;
    return start;
  }
  function cssBucketAddEntry(index,entry,si){
    var at=index.entries.length,keys=entry.bucketKeys;
    if(keys===undefined)keys=entry.bucketKeys=entry.scopes?null:cssRuleKeys($c.trim(String(entry.rule.selectorText||'')));
    $c.push(index.entries,entry);$c.push(index.sheet,si);
    if(!keys){$c.push(index.unkeyed,at);return;}
    for(var ki=0;ki<keys.length;ki++){var list=index.keyed[keys[ki]];if(!list)list=index.keyed[keys[ki]]=[];if(list[list.length-1]!==at)$c.push(list,at);}
  }
  function cssBucketIndex(build){
    var index=build.buckets;if(index)return index;
    var start=cssBucketReuseStart(build),last=cssLastBuckets;
    if(start){index=last.index;last.build.buckets=null;}
    else index={__proto__:null,entries:[],sheet:[],unkeyed:[],keyed:{__proto__:null}};
    build.buckets=index;cssLastBuckets={__proto__:null,build:build,index:index};
    for(var si=start;si<build.lists.length;si++){
      var entries=build.lists[si];
      for(var ri=0;ri<entries.length;ri++)cssBucketAddEntry(index,entries[ri],si);
    }
    return index;
  }
  // The positions of the entries that can select an element with the names `own`, ascending
  // and once each: the unkeyed ones merged with each of its names' lists.
  function cssBucketCandidates(index,own){
    var out=index.unkeyed;
    for(var name in own){var list=index.keyed[name];if(list)out=cssMergeSorted(out,list);}
    return out;
  }
  function cssMergeSorted(a,b){
    var out=[],i=0,j=0;
    while(i<a.length||j<b.length){
      if(j>=b.length||i<a.length&&a[i]<b[j])$c.push(out,a[i++]);
      else if(i>=a.length||b[j]<a[i])$c.push(out,b[j++]);
      else{$c.push(out,a[i]);i++;j++;}
    }
    return out;
  }
  var cssUnkeyedCompound=/^::?(?:host|slotted|part)(?![\w-])/i;
  function cssRuleKeysIsSeparator(c){return c===' '||c==='\t'||c==='\n'||c==='\r'||c==='\f'||c==='>'||c==='+'||c==='~';}
  function cssRuleKeysIsReject(c){return c==='\\'||c==='&'||c==='|';}
  function cssRuleKeysSkip(c,text,index){
    if(c==='"'||c==="'")return {next:computedStringEnd(text,index)};
    if(c==='[')return {next:cssBracketEnd(text,index)};
    return null;
  }
  function cssRuleKeysDepthDelta(c){return c==='('?1:c===')'?-1:0;}
  function cssRuleKeysHandleDot(text,index,c,key){
    var ident=cssApply(cssZoomExec,computedIdent,[$c.slice(text,index+1)]);
    if(!ident)return null;
    return {key:key||(c+$c.toLowerCase(ident[0])),skip:ident[0].length};
  }
  function cssRuleKeysTopChar(text,index,c,keys,key){
    if(c===','){if(!key)return null;$c.push(keys,key);return {key:null,skip:0};}
    if(cssRuleKeysIsSeparator(c))return {key:null,skip:0};
    if(c===':'&&cssApply(cssZoomExec,cssUnkeyedCompound,[$c.slice(text,index)]))return null;
    if(c==='.'||c==='#')return cssRuleKeysHandleDot(text,index,c,key);
    return {key:key,skip:0};
  }
  function cssRuleKeys(text){
    if($c.indexOf(text,'\\')>=0)return null;
    var keys=[],key=null,depth=0;
    for(var index=0;index<text.length;index++){
      var c=text[index];
      if(cssRuleKeysIsReject(c))return null;
      var skip=cssRuleKeysSkip(c,text,index);
      if(skip){index=skip.next;continue;}
      var delta=cssRuleKeysDepthDelta(c);
      if(delta){depth+=delta;continue;}
      if(depth)continue;
      var step=cssRuleKeysTopChar(text,index,c,keys,key);
      if(!step)return null;
      key=step.key;index+=step.skip;
    }
    if(!key)return null;
    $c.push(keys,key);
    return keys;
  }
  // The index of the `]` closing the attribute selector at `index` (quoted values skipped).
  function cssBracketEnd(text,index){
    while(index<text.length&&text[index]!==']'){if(text[index]==='"'||text[index]==="'")index=computedStringEnd(text,index);index++;}
    return index;
  }
  // The element's class and id names, lowercased like cssRuleKeys' names; null (test every rule)
  // when they cannot be read.
  function cssElementKeysIsWs(c){return c===' '||c==='\t'||c==='\n'||c==='\f'||c==='\r';}
  function cssElementKeysClasses(keys,names){names=$c.toLowerCase(names);var start=0;for(var index=0;index<=names.length;index++){if(index===names.length||cssElementKeysIsWs(names[index])){if(index>start)keys['.'+$c.slice(names,start,index)]=true;start=index+1;}}}
  function cssElementKeys(el){
    var keys={__proto__:null},names,id;
    try{names=cssApply(computedStyleNativeGetAttribute,el,['class']);id=cssApply(computedStyleNativeGetAttribute,el,['id']);}catch(e){return null;}
    if(id)keys['#'+$c.toLowerCase(id)]=true;
    if(names)cssElementKeysClasses(keys,names);
    return keys;
  }
  function computedMergeMedia(reads){if(computedMediaReads&&reads)for(var text in reads)if(computedMediaReads[text]===undefined)computedMediaReads[text]=reads[text];}
  function computedMediaUnchanged(reads){
    if(!reads)return true;
    try{for(var text in reads)if(mediaMatches(text)!==reads[text])return false;}catch(e){return false;}
    return true;
  }
  function appliedImportJsdom(record,el,context,out){var source=mediaRecord(record.media).source;if(record.styleSheet&&(!$c.trim(source)||jsdomScreen(source)))appliedSheetEntries(record.styleSheet,el,context.layers,out,context.seen);}
  function appliedImport(record,el,context,out){if(mediaMode==='jsdom'){appliedImportJsdom(record,el,context,out);return;}if(record.styleSheet&&record.media.mediaText)computedVolatile=true;if(record.styleSheet&&(!record.media.mediaText||mediaMatches(record.media.mediaText)))appliedSheetEntries(record.styleSheet,el,context.layers,out,context.seen);}
  // The cascade's fixed-shape records (no null-prototype literal per rule or element).
  function AppliedContext(layers,seen,owner,layer,media,scopes){this.layers=layers;this.seen=seen;this.owner=owner;this.layer=layer;this.media=media;this.scopes=scopes;}
  AppliedContext.prototype=$O.create(null);
  function ScopeFrame(root,limit,owner){this.root=root;this.limit=limit;this.owner=owner;}
  ScopeFrame.prototype=$O.create(null);
  function AppliedEntry(rule,layer,media,scopes){this.rule=rule;this.layer=layer;this.media=media;this.scopes=scopes;this.bucketKeys=undefined;}
  AppliedEntry.prototype=$O.create(null);
  function EntryMatch(entry,specificity,proximity,order){this.entry=entry;this.specificity=specificity;this.proximity=proximity;this.order=order;}
  EntryMatch.prototype=$O.create(null);
  // The context of a grouping rule's children, or null when they never apply.
  function groupContextSkip(record,kind,el){return kind==='container'||kind==='starting-style'||kind==='supports'&&!cssSupportsMatches(preludeRest(record),el);}
  function groupContextApply(inner,record,kind,context){
    if(kind==='layer'){var name=preludeRest(record);inner.layer=name?cssLayerPath(context.layer,name):cssLayerChild(context.layer,null);}
    else if(kind==='media')inner.media=cssMediaState(record,context.media);
    else if(kind==='scope'){var parts=scopeParts(record),frame=new ScopeFrame(parts[0],parts[1],context.owner);inner.scopes=context.scopes?$c.concat(context.scopes,[frame]):[frame];}
  }
  function groupContext(record,kind,el,context){
    if(groupContextSkip(record,kind,el))return null;
    var inner=new AppliedContext(context.layers,context.seen,context.owner,context.layer,context.media,context.scopes);
    groupContextApply(inner,record,kind,context);
    return inner;
  }
  function appliedGroupRulesStep(record,kind,el,context,out){
    if(record.type===3){appliedImport(record,el,context,out);return;}
    if(kind==='layer-statement'){var names=cssParts(preludeRest(record),',');for(var index=0;index<names.length;index++)cssLayerPath(context.layer,$c.trim(names[index]));return;}
    if(kind==='nested-declarations'){$c.push(out,new AppliedEntry(record,context.layer,context.media,context.scopes));return;}
    if(!kind){if(record.type===1){$c.push(out,new AppliedEntry(record,context.layer,context.media,context.scopes));if(record.cssRules)appliedGroupRules(record.cssRules,el,context,out);}return;}
    var inner=groupContext(record,kind,el,context);
    if(inner)appliedGroupRules(record.cssRules,el,inner,out);
  }
  function appliedGroupRules(list,el,context,out){
    for(var $ruleList=cssItems(list),$ruleAt=0,rule;$ruleAt<$ruleList.length&&((rule=$ruleList[$ruleAt]),true);$ruleAt++){
      var record=cssRuleRecord(rule);appliedGroupRulesStep(record,record.kind,el,context,out);
    }
  }
  function appliedSheetEntries(sheet,el,layers,out,seen){seen=seen||new Set();out=out||[];if($c.has(seen,sheet))return out;$c.add(seen,sheet);var record=sheetRecord(sheet);appliedGroupRules(record.rules,el,new AppliedContext(layers,seen,record.ownerNode,layers,0,null),out);return out;}
  // @scope (turbo-dom#273): is `el` in the scope of `frame` — an inclusive ancestor matches
  // <scope-start> (the owner node's parent for a prelude-less @scope) and no element from `el`
  // up to that root (exclusive) matches <scope-end>.
  var scopeParent=null;
  function cssParentElement(node){if(!scopeParent)scopeParent=__ns.domAccessor('parentElement',false,'Node');return cssApply(scopeParent,node,[]);}
  function cssScopeRootMatches(frame,node){return frame.root===null?!!frame.owner&&node===cssParentElement(frame.owner):computedNativeMatches(node,frame.root);}
  function cssScopeLimited(el,frame,root){if(frame.limit===null)return false;for(var node=el;node!==root;node=cssParentElement(node))if(computedNativeMatches(node,frame.limit))return true;return false;}
  function cssInScope(el,frame){
    for(var root=el;root;root=cssParentElement(root))if(cssScopeRootMatches(frame,root)&&!cssScopeLimited(el,frame,root))return true;
    return false;
  }
  // A selector list's complex selectors with their Selectors 4 specificity (turbo-dom#275),
  // from the native cascade parser: [text, a, b, c, ...], or null when the list is invalid.
  // Memoized per selector text (DOM-independent), cleared when it grows past 4096 entries.
  var cssSpecificityCache=$O.create(null),cssSpecificityCount=0;
  function cssSelectorItems(text){
    var items=cssSpecificityCache[text];
    if(items===undefined){if(cssSpecificityCount>=4096){cssSpecificityCache=$O.create(null);cssSpecificityCount=0;}items=cssNativeParse(text,'specificity')||null;cssSpecificityCache[text]=items;cssSpecificityCount++;}
    return items;
  }
  function cssSpecificityAt(items,index){return $Mmin(items[index+1],1023)*1048576+$Mmin(items[index+2],1023)*1024+$Mmin(items[index+3],1023);}
  // The specificity of the most specific selector of `selector` that `test(itemText)` accepts
  // (a one-selector list needs no test: the rule already matched), or -1.
  function cssMatchedSpecificity(selector,test){
    var items=cssSelectorItems(selector),best=-1;if(!items)return 0;
    if(items.length===4)return cssSpecificityAt(items,0);
    for(var index=0;index<items.length;index+=4){var value=cssSpecificityAt(items,index);if(value>best&&test(items[index]))best=value;}
    return best;
  }
  // Does the scoping root `root` select `el` by `item`, a selector with :scope (the root)?
  function cssScopeSelects(root,el,item){
    if(root===el)return computedNativeMatches(el,item);
    var list;try{list=cssApply(computedStyleNativeQueryAll,root,[item]);}catch(e){return false;}
    for(var index=0;index<list.length;index++)if(list[index]===el)return true;
    return false;
  }
  // The best [specificity, proximity] for which a selector of a scoped rule selects `el`
  // (turbo-dom#276), or null. A selector with :scope binds it to the scoping root; & stands
  // for :where(:scope); any other selector is relative to the root (its descendants, with no
  // added specificity). Proximity counts the generations from the root to `el`.
  // Does `item` select `el` from scoping root `root`? A `:scope`-naming selector uses the
  // scope-aware matcher; otherwise ordinary matches apply, but a bare `:scope` (root === el)
  // is not one of its descendants. Hoisted; returns a boolean — no per-call allocation.
  function cssScopedMatchItem(root, el, item){
    if ($c.test(/:scope\b/i, item)) return cssScopeSelects(root, el, item);
    return root !== el && computedNativeMatches(el, item);
  }
  function cssScopedMatchRoot(el,root,frame,items,best,hops){
    if(!cssScopeRootMatches(frame,root)||cssScopeLimited(el,frame,root))return best;
    for(var at=0;at<items.length;at+=4){
      var value=cssSpecificityAt(items,at);
      if(best&&value<=best[0])continue;
      if(cssScopedMatchItem(root,el,items[at]))best=[value,hops];
    }
    return best;
  }
  function cssScopedMatch(el,entry){
    var scopes=entry.scopes,frame=scopes[scopes.length-1],best=null;
    for(var index=0;index<scopes.length-1;index++)if(!cssInScope(el,scopes[index]))return null;
    var items=cssSelectorItems($c.replace(entry.rule.selectorText,/&/g,':where(:scope)'));if(!items)return null;
    for(var root=el,hops=0;root;root=cssParentElement(root),hops++)best=cssScopedMatchRoot(el,root,frame,items,best,hops);
    return best;
  }
  // turbo-dom#306: a child iframe document's tree is detached, so its document element (the
  // registered root, the only element there without a parent) is :root by that test.
  var frameRootPattern=/:root(?![\w-])/gi;
  function frameSelector(el,selector){
    if(!frameRootCount||!cssApply(cssZoomExec,/:root(?![\w-])/i,[selector])||computedFrameRoot(el)===null)return selector;
    return $c.replace(selector,frameRootPattern,':not(* *)');
  }
  // The cascade record of `entry`'s style rule when it selects `el` (`pseudo`: that
  // pseudo-element of it), or null. A rule under a conditional @media makes the computation
  // volatile when its selector matches.
  // Build the EntryMatch (or null) after a matched entry has been decided. The @media=2
  // entry is one that cannot match in any state (dropped); every other matched entry
  // under a conditional @media bumps computedVolatile. Allocation-free aside from the
  // EntryMatch, which existed on the hot path before the refactor.
  function appliedEntryMatchFinish(entry, matched, specificity, proximity, order){
    if (matched && entry.media && !computedMediaReads) computedVolatile = true;
    return matched && entry.media !== 2 ? new EntryMatch(entry, specificity, proximity, order) : null;
  }
  function appliedEntryMatchNormal(el,selector){var matched=false,specificity=0;try{matched=cssApply(computedStyleNativeMatches,el,[frameSelector(el,selector)]);}catch(e){}computedStateVolatile(el,selector,matched);if(matched)specificity=cssMatchedSpecificity(selector,function(item){return computedNativeMatches(el,frameSelector(el,item));});return [matched,specificity];}
  function appliedEntryMatch(el,entry,pseudo,order){
    if(noteRuleProbe)noteRuleProbe();
    var selector=entry.rule.effectiveSelector||entry.rule.selectorText,matched=false,specificity=0,proximity=Infinity;
    if(pseudo){try{matched=pseudoRuleMatches(el,selector,pseudo);}catch(e){}if(matched)specificity=cssMatchedSpecificity(selector,function(item){try{return pseudoRuleMatches(el,item,pseudo);}catch(e){return false;}});}
    else if(entry.scopes){var scoped=cssScopedMatch(el,entry);if(scoped){matched=true;specificity=scoped[0];proximity=scoped[1];}}
    else{var nm=appliedEntryMatchNormal(el,selector);matched=nm[0];specificity=nm[1];}
    return appliedEntryMatchFinish(entry, matched, specificity, proximity, order);
  }
  // Is `a` weaker than `b` once their layers tie: lower specificity, then farther from its
  // scoping root, then earlier.
  function cascadeWeaker(a,b){return a.specificity!==b.specificity?a.specificity<b.specificity:a.proximity!==b.proximity?a.proximity>b.proximity:a.order<b.order;}
  // Apply matched records weakest first, so the winner is applied last: normal declarations by
  // layer order (the unlayered ones last), then !important ones in reverse layer order (the
  // unlayered ones first); within a layer by specificity, @scope proximity, then document order.
  function cascadeEntries(records,layers,apply){
    cssLayerRanks(layers);
    var sorted=layerSorted(records,function(a,b){var x=cssLayerRank(a.entry.layer,layers),y=cssLayerRank(b.entry.layer,layers);return x!==y?x>y:!cascadeWeaker(a,b);});
    for(var index=0;index<sorted.length;index++)apply(sorted[index].entry,false);
    sorted=layerSorted(records,function(a,b){var x=cssLayerRank(a.entry.layer,layers),y=cssLayerRank(b.entry.layer,layers);return x!==y?x<y:!cascadeWeaker(a,b);});
    for(var index=0;index<sorted.length;index++)apply(sorted[index].entry,true);
  }
  // A stable insertion sort (`after(a, b)`: a goes after b).
  function layerSorted(entries,after){var out=[];for(var index=0;index<entries.length;index++){var at=out.length;while(at>0&&after(out[at-1],entries[index]))at--;$c.splice(out,at,0,entries[index]);}return out;}
  function readableSheet(sheet){var record=sheetRecord(sheet);if(!record.originClean)throw new __ns.DOMException('Cannot access rules of a cross-origin stylesheet','SecurityError');return record;}
  var cssStyleSheetRulesGetter=__ns.makeNativeFunction(function(){return readableSheet(this).rules;},'get cssRules',0);
  $O.defineProperty(sheetConstructor.prototype,'cssRules',{get:cssStyleSheetRulesGetter,enumerable:true,configurable:true});
  cssMember(sheetConstructor.prototype,'rules',function(){return readableSheet(this).rules;});

  function detachCssRule(rule,parent){var record=cssRuleRecord(rule);record.parentStyleSheet=null;record.parentRule=parent||null;if(record.cssRules)for(var $childList=cssItems(record.cssRules),$childAt=0;$childAt<$childList.length;$childAt++)detachCssRule($childList[$childAt],rule);}
  // `fresh`: the first rules of a sheet just made, which no computed style has read yet (a
  // sheet list it joins is compared on reread: computedOwnerSheets).
  function replaceSheetRules(sheet,record,text,keepImports,fresh){if(!fresh)invalidateComputedStyles();var parsed=cssItems(parseCssRules(cssString(text),sheet,null));if(!keepImports)parsed=$c.filter(parsed,function(rule){return cssRuleRecord(rule).type!==3;});var list=cssItems(record.rules);for(var $ruleAt=0;$ruleAt<list.length;$ruleAt++)detachCssRule(list[$ruleAt]);list.length=0;for(var $parsedAt=0;$parsedAt<parsed.length;$parsedAt++)$c.push(list,parsed[$parsedAt]);}
  function insertSheetRuleValidate(type,record,list,i){if(type===3&&record.constructed)throw new __ns.DOMException('Cannot insert an import into a constructed sheet','SyntaxError');if(type===3&&$c.some($c.slice(list,0,i),function(prior){return cssRuleRecord(prior).type!==3;}))throw new __ns.DOMException('Imports must precede style rules','HierarchyRequestError');if(type!==3&&$c.some($c.slice(list,i),function(next){return cssRuleRecord(next).type===3;}))throw new __ns.DOMException('Style rules must follow imports','HierarchyRequestError');}
  function insertSheetRule(sheet,text,index){invalidateComputedStyles();var record=readableSheet(sheet),list=cssItems(record.rules),i=index===undefined?0:Number(index)>>>0;if(record.replacing)throw new __ns.DOMException('The sheet is being replaced','NotAllowedError');if(i>list.length)throw new __ns.DOMException('The rule index is out of range','IndexSizeError');var source=cssNativeParse(cssString(text),false,true),parsed=source===null?cssList():parseCssRules(source,sheet,null,true);if(parsed.length!==1)throw new __ns.DOMException('Expected one CSS rule','SyntaxError');var rule=cssItems(parsed)[0],type=cssRuleRecord(rule).type;insertSheetRuleValidate(type,record,list,i);$c.splice(list,i,0,rule);return i;}
  function deleteSheetRule(sheet,index){invalidateComputedStyles();var record=readableSheet(sheet),list=cssItems(record.rules),i=Number(index)>>>0;if(record.replacing)throw new __ns.DOMException('The sheet is being replaced','NotAllowedError');if(i>=list.length)throw new __ns.DOMException('The rule index is out of range','IndexSizeError');detachCssRule(list[i]);$c.splice(list,i,1);}
  cssMethod(sheetConstructor.prototype,'insertRule',1,function(text,index){sheetRecord(this);if(!arguments.length)throw new TypeError('insertRule requires 1 argument');return insertSheetRule(this,text,index);});
  cssMethod(sheetConstructor.prototype,'deleteRule',1,function(index){sheetRecord(this);if(!arguments.length)throw new TypeError('deleteRule requires 1 argument');deleteSheetRule(this,index);});
  cssMethod(sheetConstructor.prototype,'addRule',0,function(selector,style,index){var record=sheetRecord(this);insertSheetRule(this,(selector===undefined?'undefined':cssString(selector))+' {'+(style===undefined?'undefined':cssString(style))+'}',index===undefined?cssItems(record.rules).length:index);return -1;});
  cssMethod(sheetConstructor.prototype,'removeRule',0,function(index){deleteSheetRule(this,index===undefined?0:index);});
  cssMethod(sheetConstructor.prototype,'replaceSync',1,function(text){var record=sheetRecord(this);if(!arguments.length)throw new TypeError('replaceSync requires 1 argument');if(!record.constructed||record.replacing)throw new __ns.DOMException('The sheet cannot be replaced','NotAllowedError');replaceSheetRules(this,record,text,false);});
  cssMethod(sheetConstructor.prototype,'replace',1,function(text){var record;try{record=sheetRecord(this);if(!arguments.length)throw new TypeError('replace requires 1 argument');if(!record.constructed||record.replacing)throw new __ns.DOMException('The sheet cannot be replaced','NotAllowedError');replaceSheetRules(this,record,text,false);return $promiseResolve(this);}catch(error){return $promiseReject(error);}});
  function linkedSheet(el,text,url){var sheet=$O.create(sheetConstructor.prototype),record=newSheetRecord();record.constructed=false;record.ownerNode=el;record.href=url||null;record.title=sheetNativeAttribute(el,'title')||null;record.media=makeMedia(sheetNativeAttribute(el,'media')||'',el);$c.set(sheetRecords,sheet,record);replaceSheetRules(sheet,record,text,true,true);return sheet;}
  function detachSheet(sheet){invalidateComputedStyles();if(sheet){var record=sheetRecord(sheet);if(record.ownerNode)record.title=sheetNativeAttribute(record.ownerNode,'title')||null;mediaRecord(record.media).owner=null;record.ownerNode=null;}}
  function attachLinkedStyleSheet(el,text,url,originClean){invalidateComputedStyles();var sheet=linkedSheet(el,text,url),old=elementSheets.get(el);sheetRecord(sheet).originClean=originClean!==false;if(old)detachSheet(old.sheet);elementSheets.set(el,{sheet:sheet,text:null});return sheet;};
  function attachImportStyleSheet(rule,sheet,url,text,originClean){invalidateComputedStyles();var parent=cssRuleRecord(rule,3),record=sheetRecord(sheet);record.constructed=false;record.ownerRule=rule;record.parentStyleSheet=parent.parentStyleSheet;record.href=url||null;record.originClean=originClean!==false;if(text!==undefined)replaceSheetRules(sheet,record,text,true);parent.styleSheet=sheet;};
  // Private live-layout export never reads page properties or invokes public
  // collection iteration. Imports retain their actual attached sheet graph.
  function cssHostArray(){var value=[];cssSetPrototype(value,null);return value;}
  function cssHostMedia(media){var record=mediaRecords.get(media);if(!record)throw new TypeError('Illegal invocation');if(record.owner){var text=sheetNativeAttribute(record.owner,'media')||'';if(text!==record.source)return cssNativeParse(text,'media');}var result='';for(var i=0;i<record.items.length;i++){if(i)result+=', ';result+=record.items[i];}return result;}
  function cssHostText(text,budget){budget.bytes-=text.length;if(budget.bytes<0)throw new RangeError('Live stylesheet text budget exceeded');return text;}
  function cssHostRule(rule,seen,budget){
    if(--budget.rules<0)throw new RangeError('Live stylesheet rule budget exceeded');
    var record=cssRuleRecord(rule),output={__proto__:null,type:record.type};
    if(record.cssRules&&cssItems(record.cssRules).length&&record.prelude!==undefined){output.prelude=cssHostText(record.prelude,budget);output.rules=cssHostRules(record.cssRules,seen,budget);output.cssText=record.prelude+' {';for(var index=0;index<output.rules.length;index++)output.cssText+=' '+output.rules[index].cssText;output.cssText+=' }';}else output.cssText=cssRuleText(record);
    output.cssText=cssHostText(output.cssText,budget);
    if(record.type===3){output.href=cssHostText(record.href,budget);output.media=cssHostText(cssHostMedia(record.media),budget);output.styleSheet=record.styleSheet?cssHostSheet(record.styleSheet,seen,budget):null;}
    return output;
  }
  function cssHostRules(list,seen,budget){var source=cssItems(list),output=cssHostArray();for(var i=0;i<source.length;i++)output[i]=cssHostRule(source[i],seen,budget);return output;}
  function cssHostSheet(sheet,seen,budget){
    for(var i=0;i<seen.length;i++)if(seen[i]===sheet)return {__proto__:null,cycle:true};
    if(--budget.sheets<0)throw new RangeError('Live stylesheet graph budget exceeded');
    seen[seen.length]=sheet;
    var record=sheetRecord(sheet),output={__proto__:null,disabled:record.disabled,media:cssHostText(cssHostMedia(record.media),budget),href:record.href===null?null:cssHostText(record.href,budget),originClean:record.originClean,rules:cssHostRules(record.rules,seen,budget)};
    seen.length--;return output;
  }
  function exportLiveSheet(sheet){return cssHostSheet(sheet,cssHostArray(),{__proto__:null,rules:20000,sheets:1024,bytes:2000000});}
  var documentCSSRecord=collectionGet(d)||{kind:'Document'};documentCSSRecord.cssHost={exportSheet:exportLiveSheet,looksLikeCSS:function(text){return !String(text).trim()||!!cssNativeParse(String(text)).trim();},sheet:function(element){return cssApply(currentSheetGetter,element,[]);},readRules:function(sheet){var source=cssItems(sheetRecord(sheet).rules),snapshot=[];cssSetPrototype(snapshot,null);for(var index=0;index<source.length;index++)snapshot[index]=source[index];return snapshot;},readRule:function(rule){var record=cssRuleRecord(rule);return {type:record.type,href:record.href||null};},attachLinked:attachLinkedStyleSheet,attachImport:attachImportStyleSheet};collectionSet(d,documentCSSRecord);
  // ObservableArray adoption has a stable identity and authoritative backing
  // state. Renderer exports never consult a page-defined document getter.
  var adoptionStates=cssMap('adoption'),adoptionDocumentCheck=__ns.domAccessor('nodeType',false,'Document'),adoptionNodeCheck=__ns.domAccessor('nodeType',false,'Node'),adoptionShadowCheck=__ns.domAccessor('__isShadowRoot',false,'Node'),adoptionOwner=__ns.domAccessor('ownerDocument',false,'Node'),adoptionDOMException=__ns.DOMException,adoptionDefine=Reflect.defineProperty,adoptionSet=Reflect.set,adoptionDelete=Reflect.deleteProperty,adoptionOwn=Object.hasOwn,adoptionProxy=Proxy,adoptionFloor=Math.floor;
  function adoptionRecord(value){var record=sheetRecords.get(value);if(!record)throw new TypeError('The value is not a CSSStyleSheet');return record;}
  function adoptionDocument(owner){var type=cssApply(adoptionNodeCheck,owner,[]);if(type===9)return owner;if(type===11&&cssApply(adoptionShadowCheck,owner,[]))return cssApply(adoptionOwner,owner,[]);throw new TypeError('Illegal invocation');}
  function adoptionAllowed(owner,value){var record=adoptionRecord(value);if(!record.constructed||record.constructorDocument!==adoptionDocument(owner))throw new adoptionDOMException('The stylesheet cannot be adopted','NotAllowedError');}
  function adoptionDefineRejected(descriptor){return adoptionOwn(descriptor,'get')||adoptionOwn(descriptor,'set')||adoptionOwn(descriptor,'configurable')&&!descriptor.configurable||adoptionOwn(descriptor,'enumerable')&&!descriptor.enumerable||adoptionOwn(descriptor,'writable')&&!descriptor.writable;}
  function adoptionDefineIndex(descriptor,i,index){
    if(adoptionDefineRejected(descriptor))return false;
    return adoptionOwn(descriptor,'value')?index(i,descriptor.value):true;
  }
  function adoptionDefineLength(descriptor,length){
    if(adoptionOwn(descriptor,'get')||adoptionOwn(descriptor,'set')||adoptionOwn(descriptor,'writable')&&!descriptor.writable)return false;
    return adoptionOwn(descriptor,'value')?length(descriptor.value):true;
  }
  function createAdoption(owner){
    var target=[],state={__proto__:null,target:target,proxy:null};
    function length(value){var n=+value;if(n<0||n>4294967295||n!==adoptionFloor(n))throw new RangeError('Invalid array length');if(n<target.length)target.length=n;return true;}
    function index(key,value){adoptionAllowed(owner,value);if(key>target.length)return true;adoptionDefine(target,String(key),{value:value,writable:true,enumerable:true,configurable:true});return true;}
    state.proxy=new adoptionProxy(target,{
      preventExtensions:function(){return false;},
      set:function(target,key,value,receiver){invalidateComputedStyles();var i=cssIndex(key);if(key==='length')return length(value);return i>=0&&i<4294967295?index(i,value):adoptionSet(target,key,value,receiver);},
      deleteProperty:function(target,key){invalidateComputedStyles();var i=cssIndex(key);if(i>=0&&i<target.length){target.length=i;return true;}return adoptionDelete(target,key);},
      defineProperty:function(target,key,descriptor){invalidateComputedStyles();var i=cssIndex(key);if(i>=0&&i<4294967295)return adoptionDefineIndex(descriptor,i,index);if(key==='length')return adoptionDefineLength(descriptor,length);return adoptionDefine(target,key,descriptor);}
    });
    return state;
  }
  function adoptionState(receiver){adoptionDocument(receiver);var state=$c.get(adoptionStates,receiver);if(!state){state=createAdoption(receiver);$c.set(adoptionStates,receiver,state);}return state;}
  function readAdoption(){cssApply(adoptionDocumentCheck,this,[]);return adoptionState(this).proxy;}
  function readShadowAdoption(){if(cssApply(adoptionNodeCheck,this,[])!==11||!cssApply(adoptionShadowCheck,this,[]))throw new TypeError('Illegal invocation');return adoptionState(this).proxy;}
  var adoptionNodeType=__ns.domAccessor('nodeType',false,'Node');
  function adoptionOwnerName(node){return cssApply(adoptionNodeType,node,[])===9?'Document':'ShadowRoot';}
  function replaceAdoption(input){invalidateComputedStyles();var state=adoptionState(this),converted=cssHostArray();if(input===null||(typeof input!=='object'&&typeof input!=='function'))throw new TypeError("Failed to set the 'adoptedStyleSheets' property on '"+(adoptionOwnerName(this))+"': The provided value cannot be converted to a sequence.");for(var value of input){adoptionRecord(value);converted[converted.length]=value;}state.target.length=0;for(var i=0;i<converted.length;i++){adoptionAllowed(this,converted[i]);adoptionDefine(state.target,String(i),{value:converted[i],writable:true,enumerable:true,configurable:true});}}
  var adoptionDescriptor={configurable:true,enumerable:true,get:__ns.makeNativeFunction(readAdoption,'get adoptedStyleSheets',0),set:__ns.makeNativeFunction(function(input){cssApply(adoptionDocumentCheck,this,[]);return cssApply(replaceAdoption,this,[input]);},'set adoptedStyleSheets',1)};
  var shadowAdoptionDescriptor={configurable:true,enumerable:true,get:__ns.makeNativeFunction(readShadowAdoption,'get adoptedStyleSheets',0),set:__ns.makeNativeFunction(function(input){cssApply(readShadowAdoption,this,[]);return cssApply(replaceAdoption,this,[input]);},'set adoptedStyleSheets',1)};
  Object.defineProperty(d,'adoptedStyleSheets',adoptionDescriptor);adoptionState(d);
  documentCSSRecord.cssHost.readAdopted=function(owner){var target=adoptionState(owner||d).target,snapshot=cssHostArray();for(var i=0;i<target.length;i++)snapshot[i]=target[i];return snapshot;};
  // document.title reflects the <title> element's text (jsdom semantics). The setter creates a
  // <title> in <head> on demand; the document never gets one it did not parse (Chrome:
  // `<!doctype html><body>` has an empty <head>; turbo-surf#36, computed_defaults_chrome156.json).
  // document_basics.js replaces this accessor with the spec-shaped Document.prototype.title.
  {
    var ensureTitle = function(){
      var el = d.querySelector && d.querySelector('title');
      if (!el) { el = d.createElement('title'); var head = (d.head) || (d.querySelector && d.querySelector('head')) || d.documentElement || d.body; if (head && head.appendChild) head.appendChild(el); }
      return el;
    };
    try { Object.defineProperty(d, 'title', { configurable: true,
      get: function(){ var el = d.querySelector && d.querySelector('title'); return el ? (el.textContent || '') : ''; },
      set: function(v){ var el = ensureTitle(); if (el) el.textContent = String(v == null ? '' : v); } }); } catch(e){}
  }
  // document.cookie jar. Defined on the document PROTOTYPE (not the instance) — code reads
  // `Object.getOwnPropertyDescriptor(Object.getPrototypeOf(document), 'cookie')` to wrap it. Setter
  // parses "k=v; attrs"; max-age<=0 deletes. Getter serializes the live pairs.
  function cookieIsDelete(parts){for(var i=1;i<parts.length;i++){var p=parts[i].trim();var m=/^max-age\s*=\s*(-?\d+)/i.exec(p);if(m&&parseInt(m[1],10)<=0)return true;if(/^expires\s*=/i.test(p)&&/1970/.test(p))return true;}return false;}
  (function(){
    var proto = Object.getPrototypeOf(d) || d;
    if (!Object.getOwnPropertyDescriptor(proto, 'cookie')) {
      var jar = {};
      try { Object.defineProperty(proto, 'cookie', {
        configurable: true,
        get: function(){ var out = []; for (var k in jar) out.push(k + '=' + jar[k]); return out.join('; '); },
        set: function(str){
          str = String(str == null ? '' : str);
          var parts = str.split(';'); var first = parts[0] || ''; var eq = first.indexOf('=');
          if (eq < 0) return; var name = first.slice(0, eq).trim(); var val = first.slice(eq + 1).trim();
          if (cookieIsDelete(parts)) { delete jar[name]; } else { jar[name] = val; }
        }
      }); } catch(e){}
    }
  })();
  // React's isEventSupported('input') checks `'oninput' in document`; make it true (+ keep
  // documentMode ABSENT) so React uses the modern input/change path, not the IE change polyfill —
  // otherwise fireEvent.change never fires onChange.
  d.oninput = null; d.onchange = null; d.onclick = null; d.onkeydown = null; d.onkeyup = null;
  function ensureDocumentHead(){
    if (!d.head) { try { d.head = d.createElement('head'); if (d.documentElement) d.documentElement.appendChild(d.head); } catch(e){} }
  }
  ensureDocumentHead();
  // NOTE: document.createElementNS uses the native binding (namespace-aware: SVG/MathML elements keep
  // their namespace + case-preserved attributes like viewBox). Previously aliased to createElement.
  // NOTE: document.addEventListener/removeEventListener/dispatchEvent use the native binding (the
  // document is a wrapped node) so document-level listeners — click-outside handlers, global
  // keydown — register and fire via the bubble path. (Previously stubbed to no-ops.)
  // CSSOM shim (<style>.sheet) + form-control value/checked. value/checked are defined BOTH as an
  // OWN accessor on each control element (testing-library's setNativeValue reads
  // getOwnPropertyDescriptor(element)) AND on the interface .prototype (React's value-tracker reads
  // node.constructor.prototype). No setPrototypeOf — avoids touching the native proto chain.
  var currentSheetGetter, currentSheetGetterStyle;
  var sheetTagName=__ns.domAccessor('tagName',false,'Element'),sheetConnected=__ns.domAccessor('isConnected',false,'Node'),sheetTextContent=__ns.domAccessor('textContent',false,'Node'),sheetGetAttribute=d.createElement('style').getAttribute;
  function sheetNativeAttribute(element,name){return cssApply(sheetGetAttribute,element,[name]);}
  function setupValueAsNumber(baseProto){if(!Object.getOwnPropertyDescriptor(baseProto,'valueAsNumber')){try{Object.defineProperty(baseProto,'valueAsNumber',{configurable:true,get:function(){var v=this.value;if(v==null||v==='')return NaN;var n=Number(v);return isNaN(n)?NaN:n;},set:function(n){this.value=(n==null||isNaN(n))?'':String(n);}});}catch(e){}}}
  function setupGetClientRects(baseProto){if(!baseProto.getClientRects)baseProto.getClientRects=function(){var r=$N.getBoundingClientRect?$call($N.getBoundingClientRect,this):{x:0,y:0,top:0,left:0,right:0,bottom:0,width:0,height:0};var list=[r];list.item=function(i){return list[i]||null;};return list;};}
  (function(){
    var mkSheet = function(el){return linkedSheet(el,cssApply(sheetTextContent,el,[])||'',null);};
    var orig = d.createElement;
    // value/checked live ONLY on the interface .prototype (NOT own — React's value-tracker bails on
    // node.hasOwnProperty('value')). Each control's actual proto is set to that interface prototype
    // so getPrototypeOf(el) === el.constructor.prototype has the descriptor (React + testing-library).
    // A `date`/`time` <input> only retains a value the browser can parse — an invalid/partial string
    // sets value to ''. userEvent.type relies on this (isValidDateOrTimeValue clones, assigns, and
    // checks the value stuck) to commit the typed value + fire onChange only once a full valid date
    // is entered. Without it, partials commit and the controlled reset mangles the result.
    // Null-prototype: the key is the page's `type` attribute (`constructor`, `__proto__`).
    var validForTypePatterns = { __proto__: null,
      date: /^\d{4}-\d{2}-\d{2}$/,
      time: /^\d{2}:\d{2}(:\d{2})?$/,
      month: /^\d{4}-\d{2}$/,
      'datetime-local': /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/,
      number: /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/
    };
    var validForType = function(el, v){
      var t = $c.toLowerCase($getAttr(el, 'type') || 'text');
      // Range, the date/time family and textarea newlines follow Chromium (input_members.js).
      var sanitized = __ns.sanitizeControlValue ? __ns.sanitizeControlValue(el, t, v) : undefined;
      if (sanitized !== undefined) return sanitized;
      if (t === 'number') return (v === '' || $c.test(validForTypePatterns.number, v)) ? v : '';
      var pattern = validForTypePatterns[t];
      if (pattern) return $c.test(pattern, v) ? v : '';
      return v;
    };
    // Live input/textarea values have a dirty flag, separate from their defaults.
    // Hidden/button values remain reflected attributes as required by HTML.
    var controlValues = new WeakMap();
    // The file state's filename value mode (file_inputs.js) registers once, privately.
    var fileControls = null;
    __ns.registerFileControls = function(hooks){ fileControls = hooks; delete __ns.registerFileControls; };
    var fileValueDelete = WeakMap.prototype.delete, fileValueApply = Reflect.apply;
    __ns.resetControlValue = function(element){ bumpStyleState(); fileValueApply(fileValueDelete, controlValues, [element]); };
    // Captured natives: the value paths never reach a page-replaced builtin (turbo-dom#199).
    // The binding's own native accessors, captured at install.
    var valueLocalName, valueNamespace, valueTextContent, valueTextContentSet;
    var valueNatives = function(){ if (!valueTextContent) { valueLocalName = __ns.domAccessor('localName', false, 'Element'); valueNamespace = __ns.domAccessor('namespaceURI', false, 'Element'); valueTextContent = __ns.domAccessor('textContent', false, 'Node'); valueTextContentSet = __ns.domAccessor('textContent', true, 'Node'); } };
    valueNatives();
    var valueName = function(el){ valueNatives(); return $apply(valueNamespace, el, []) === 'http://www.w3.org/1999/xhtml' ? $apply(valueLocalName, el, []) : ''; };
    var isTextarea = function(el){ return valueName(el) === 'textarea'; };
    // A button's type is always submit/reset/button; an input's reflected modes follow its type attribute.
    var reflectedValue = function(el){ var name = valueName(el); return name === 'button' || (name === 'input' && $c.test(/^(hidden|button|submit|reset|image|checkbox|radio)$/, $c.toLowerCase($getAttr(el, 'type') || ''))); };
    var defaultValueOf = function(el){ return isTextarea(el) ? $apply(valueTextContent, el, []) || '' : $getAttr(el, 'value') || ''; };
    var valDesc = { configurable: true,
      get: function(){
        var file = fileControls ? fileControls.value(this) : undefined;
        if (file !== undefined) return file;
        if ($c.has(controlValues,this) && !reflectedValue(this)) return validForType(this, $c.get(controlValues,this));
        var value = defaultValueOf(this);
        if (!isTextarea(this) && $c.test(/^(checkbox|radio)$/, $c.toLowerCase($getAttr(this, 'type') || '')) && !$hasAttr(this, 'value')) return 'on';
        return validForType(this, value);
      },
      set: function(v){
        if (fileControls && fileControls.setValue(this, v)) return;
        var oldValue=selectionApply(valDesc.get,this,[]);
        v = v == null ? '' : String(v);
        if (reflectedValue(this)) $setAttr(this, 'value', v);
        else { bumpStyleState(); $c.set(controlValues,this, validForType(this, v)); }
        selectionValueChanged(this,oldValue,selectionApply(valDesc.get,this,[]));
      }
    };
    var defaultValueDesc = { configurable: true, get: function(){ return defaultValueOf(this); }, set: function(v){
      if (isTextarea(this)) { valueNatives(); $apply(valueTextContentSet, this, [String(v)]); }
      else $setAttr(this, 'value', String(v));
    } };
    var checkedNative=g.__checkedNative,checkedSetAttribute=d.createElement('input').setAttribute,checkedRemoveAttribute=d.createElement('input').removeAttribute;delete g.__checkedNative;
    var checkedDesc = { configurable: true, get: function(){ return checkedNative(0,this); }, set: function(v){ checkedNative(1,this,!!v); } };
    var defaultCheckedDesc={configurable:true,get:function(){checkedNative(0,this);return sheetNativeAttribute(this,'checked')!==null;},set:function(value){checkedNative(0,this);if(value)cssApply(checkedSetAttribute,this,['checked','']);else cssApply(checkedRemoveAttribute,this,['checked']);}};
    __ns.formControlChecked=function(element){return checkedNative(0,element);};
    // `type` defaults to 'text' for <input> — React's isTextInputElement keys on it to pick the
    // input/change handling path; undefined would route changes to the select/checkbox path.
    // An <input> type outside the known states (e.g. 'file ') is the Text state.
    var inputTypeStates = /^(hidden|text|search|tel|url|email|password|date|month|week|time|datetime-local|number|range|color|checkbox|radio|file|submit|image|reset|button)$/;
    var mkTypeDesc = function(def){ return { configurable: true, get: function(){ var t = $c.toLowerCase($getAttr(this, 'type') || def); return def === 'text' && !$c.test(inputTypeStates, t) ? 'text' : t; }, set: function(v){ $setAttr(this, 'type', v); } }; };
    // `form` → the nearest ancestor <form> (userEvent fires submit on a submit-button's form).
    var formDesc = { configurable: true, get: function(){ return this.closest ? this.closest('form') : null; } };
    // selection.js owns the text-control selection model: it defines selectionStart/End/Direction,
    // select, setSelectionRange and setRangeText on both control prototypes during every install,
    // and value writes report to its controlValueChanged hook (absent only before it installs).
    var selectionApply=Reflect.apply;
    function selectionValueChanged(element,oldValue,newValue){if(__ns.controlValueChanged)__ns.controlValueChanged(element,oldValue,newValue);}
    var baseProto = compatibilityElementPrototype = Object.getPrototypeOf(orig.call(d,'span'));
    // Reflected IDL attributes on every element: setting el.src / el.href (script/img/link/a — a DOM
    // property) must show as the content attribute so attribute selectors like script[src*="maps"]
    // and getAttribute('src') see it.
    // Reflected string IDL attributes on every element. `rel` is load-bearing: React's
    // clearContainerSparingly does `node.rel.toLowerCase()` over a container's <link> children during
    // hydration — undefined there aborts the whole app mount. `as`/`media`/`type` ride the same
    // pattern (<link rel=preload as=script media=…>, <script type=…>). For inputs `type` is shadowed
    // per-instance by the text-defaulting control descriptor (applyControlProto), so this is the
    // fallback for non-control elements.
    ['src','href','rel','as','media','type'].forEach(function(a){ if (!Object.getOwnPropertyDescriptor(baseProto, a)) { try { Object.defineProperty(baseProto, a, { configurable: true, get: function(){ return $getAttr(this, a) || ''; }, set: function(v){ $setAttr(this, a, v == null ? '' : String(v)); } }); } catch(e){} } });
    // CharacterData mutation methods (insertData/deleteData/appendData/replaceData/substringData/
    // splitText) are now native rtdom Tree ops bound on every node — see browser_env.rs.
    // input.valueAsNumber — the numeric view of `value`, NaN when non-numeric/empty. MUI Slider's
    // hidden range input reads event.target.valueAsNumber in its change handler to derive the new
    // value; without it the value came through as null. (number/range inputs only; others give NaN.)
    setupValueAsNumber(baseProto);
    setupGetClientRects(baseProto);
    if (!baseProto.isEqualNode) {
      var __nodeAttrsEq = function(a, b){
        var aa = $call($N.nativeAttributes, a), ba = $call($N.nativeAttributes, b);
        if (aa.length !== ba.length) return false;
        for (var i = 0; i < aa.length; i++) {
          if ($call($N.getAttribute, b, $call($N['attr.name'], aa[i])) !== $call($N['attr.value'], aa[i])) return false;
        }
        return true;
      };
      var __nodeChildrenEq = function(a, b){
        var ac = $call($N.firstChild, a), bc = $call($N.firstChild, b);
        for (; ac !== null && bc !== null; ac = $call($N.nextSibling, ac), bc = $call($N.nextSibling, bc)) { if (!__nodeEq(ac, bc)) return false; }
        return ac === bc;
      };
      var __nodeEqData = function(a, b, type){
        if (type === 1) return __nodeAttrsEq(a, b);
        return ($call($N.nodeValue, a) || '') === ($call($N.nodeValue, b) || '');
      };
      var __nodeEq = function(a, b){
        if (a === b) return true;
        if (!a || !b) return false;
        var type = $call($N.nodeType, a);
        if (type !== $call($N.nodeType, b) || $call($N.nodeName, a) !== $call($N.nodeName, b)) return false;
        if (!__nodeEqData(a, b, type)) return false;
        return __nodeChildrenEq(a, b);
      };
      baseProto.isEqualNode = function(other){ return __nodeEq(this, other); };
      if (!baseProto.isSameNode) baseProto.isSameNode = function(other){ return this === other; };
    }
    // getElementsByTagName / getElementsByClassName / getElementsByName over querySelectorAll. The
    // native binding ships querySelector(All) only; libs (jQuery's load-time support probe does
    // el.getElementsByTagName('input')[0].checked) need these. Add to the shared element prototype
    // (guarded so a native impl wins if ever added).
    // Walk descendants via `children` (works on a DETACHED subtree, unlike querySelectorAll which
    // matches only connected nodes — same reason select.options walks children).
    var geWalk = function(node, match){ var out = []; (function visit(n){ var kids = n.children || []; for (var i=0;i<kids.length;i++){ if (match(kids[i])) out.push(kids[i]); visit(kids[i]); } })(node); out.item = function(i){ return out[i] || null; }; out.namedItem = function(nm){ for (var i=0;i<out.length;i++){ var e=out[i]; if (e && (e.id === nm || (e.getAttribute && e.getAttribute('name') === nm))) return e; } return null; }; return out; };
    var geByTag = function(t){ var want = String(t).toUpperCase(); return geWalk(this, function(e){ return want === '*' || String(e.tagName).toUpperCase() === want; }); };
    var geByClass = function(c){ var want = String(c).trim().split(/\s+/).filter(Boolean); return geWalk(this, function(e){ var cls = String(e.className || '').split(/\s+/); return want.every(function(w){ return cls.indexOf(w) >= 0; }); }); };
    var geByName = function(nm){ var want = String(nm); return geWalk(this, function(e){ return e.getAttribute && e.getAttribute('name') === want; }); };
    var addGetElems = function(proto){
      if (typeof proto.getElementsByTagName !== 'function') proto.getElementsByTagName = geByTag;
      if (typeof proto.getElementsByClassName !== 'function') proto.getElementsByClassName = geByClass;
      if (typeof proto.getElementsByName !== 'function') proto.getElementsByName = geByName;
    };
    addGetElems(baseProto);
    g.__addGetElems = addGetElems; // reused for document below
    // Traversal accessors the native binding lacks (it ships firstChild/nextSibling/children/
    // firstElementChild only). jQuery's clone-support probe reads clone.lastChild.checked; Sizzle
    // walks siblings. Derive from native childNodes/children/parentNode (all detached-safe).
    var defTrav = function(name, get){ if (baseProto && !Object.getOwnPropertyDescriptor(baseProto, name)) { try { Object.defineProperty(baseProto, name, { configurable: true, get: get }); } catch(e){} } };
    // The binding's native accessors, captured at install (turbo-dom#199).
    var travNext=__ns.domAccessor('nextSibling',false,'Node'),travType=__ns.domAccessor('nodeType',false,'Node');
    var travParent=__ns.domAccessor('parentNode',false,'Node'),travFirst=__ns.domAccessor('firstChild',false,'Node');
    var travLast=function(node,elements){ var last=null; for (var c=$call(travFirst,node); c; c=$call(travNext,c)) if (!elements || $call(travType,c)===1) last=c; return last; };
    var travPrevious=function(node,elements){ var p=$call(travParent,node); if (!p) return null; var previous=null; for (var c=$call(travFirst,p); c && c!==node; c=$call(travNext,c)) if (!elements || $call(travType,c)===1) previous=c; return previous; };
    defTrav('lastChild', function(){ return travLast(this,false); });
    defTrav('lastElementChild', function(){ return travLast(this,true); });
    defTrav('previousSibling', function(){ return travPrevious(this,false); });
    defTrav('previousElementSibling', function(){ return travPrevious(this,true); });
    defTrav('nextElementSibling', function(){ for (var node = $call(travNext, this); node; node = $call(travNext, node)) if ($call(travType, node) === 1) return node; return null; });
    // classList on the shared element prototype (every element inherits it), backed by className.
    if (baseProto && !Object.getOwnPropertyDescriptor(baseProto, 'classList')) {
      try { Object.defineProperty(baseProto, 'classList', { configurable: true, get: function(){
        var el = this;
        var parse = function(){ return String(el.className || '').split(/\s+/).filter(Boolean); };
        var write = function(a){ el.className = a.join(' '); };
        var dt = {
          add: function(){ var a = parse(); for (var i=0;i<arguments.length;i++) if (a.indexOf(arguments[i])<0) a.push(arguments[i]); write(a); },
          remove: function(){ var a = parse(); for (var i=0;i<arguments.length;i++){ var x=a.indexOf(arguments[i]); if (x>=0) a.splice(x,1); } write(a); },
          toggle: function(c, force){ var a = parse(); var has = a.indexOf(c)>=0; var on = force === undefined ? !has : !!force; if (on && !has) a.push(c); else if (!on && has) a.splice(a.indexOf(c),1); write(a); return on; },
          contains: function(c){ return parse().indexOf(c)>=0; },
          replace: function(o,n){ var a = parse(); var x = a.indexOf(o); if (x<0) return false; a[x]=n; write(a); return true; },
          item: function(i){ return parse()[i] || null; },
          forEach: function(cb, thisArg){ parse().forEach(cb, thisArg); },
          toString: function(){ return String(el.className || ''); }
        };
        Object.defineProperty(dt, 'length', { get: function(){ return parse().length; } });
        Object.defineProperty(dt, 'value', { get: function(){ return String(el.className || ''); } });
        return dt;
      } }); } catch(e){}
    }
    var protoFor = {};
    var defType = { HTMLInputElement:'text', HTMLButtonElement:'submit' };
    var controlStrings = { HTMLInputElement:['name','placeholder','accept','min','max','step','pattern','autocomplete'], HTMLTextAreaElement:['name','placeholder','autocomplete'], HTMLSelectElement:['name','autocomplete'], HTMLButtonElement:['name'] };
    var controlBooleans = { HTMLInputElement:[['disabled','disabled'],['multiple','multiple'],['required','required'],['readOnly','readonly']], HTMLTextAreaElement:[['disabled','disabled'],['required','required'],['readOnly','readonly']], HTMLSelectElement:[['disabled','disabled'],['multiple','multiple'],['required','required']], HTMLButtonElement:[['disabled','disabled']], HTMLOptionElement:[['disabled','disabled']] };
    var controlHasDefaultValue={__proto__:null,HTMLInputElement:true,HTMLTextAreaElement:true};
    ['HTMLInputElement','HTMLTextAreaElement','HTMLSelectElement','HTMLOptionElement','HTMLButtonElement','HTMLLabelElement','HTMLCanvasElement'].forEach(function(n){ if (typeof g[n] !== 'function') g[n] = function(){}; var p = Object.create(baseProto); try {
      var defineControl=function(name,descriptor){Object.defineProperty(p,name,Object.assign({},descriptor,{enumerable:true}));};
      if(n!=='HTMLLabelElement')defineControl('value',valDesc);if(n==='HTMLInputElement'){defineControl('checked',checkedDesc);defineControl('defaultChecked',defaultCheckedDesc);}defineControl('form',formDesc);
      if (defType[n]) defineControl('type',mkTypeDesc(defType[n]));
      (controlStrings[n]||[]).forEach(function(a){ Object.defineProperty(p, a, { configurable: true, enumerable:true, get: function(){ return $getAttr(this, a) || ''; }, set: function(v){ $setAttr(this, a, v == null ? '' : String(v)); } }); });
      (controlBooleans[n]||[]).forEach(function(pair){ Object.defineProperty(p, pair[0], { configurable: true, enumerable:true, get: function(){ return $hasAttr(this, pair[1]); }, set: function(v){ if (v) $setAttr(this, pair[1], ''); else $removeAttr(this, pair[1]); } }); });
      if (controlHasDefaultValue[n]) defineControl('defaultValue',defaultValueDesc);
    } catch(e){} g[n].prototype = p; protoFor[n] = p; });
    // The select's list of options (HTML's "option element nearest ancestor select", Chrome 156
    // measured in select_element_chrome156.json): option descendants in tree order, skipping nested
    // select/datalist/hr/option subtrees and optgroups inside an optgroup. A sibling walk also serves
    // DETACHED subtrees, which React-DOM's updateOptions reads during commit, and stays linear for
    // huge lists (indexing the live `children` collection recomputes it per access).
    var optionNs = 'http://www.w3.org/1999/xhtml';
    // The binding's native tree accessors, captured at install: the walk never reaches a page-replaced builtin (turbo-dom#199).
    var optionFirst, optionNext, optionKind, optionSpace, optionLocal, optionParent;
    var optionNatives = function(){ if (!optionFirst) { optionFirst = __ns.domAccessor('firstChild', false, 'Node'); optionNext = __ns.domAccessor('nextSibling', false, 'Node'); optionKind = __ns.domAccessor('nodeType', false, 'Node'); optionSpace = __ns.domAccessor('namespaceURI', false, 'Element'); optionLocal = __ns.domAccessor('localName', false, 'Element'); optionParent = __ns.domAccessor('parentNode', false, 'Node'); } };
    optionNatives();
    var optionName = function(n){ optionNatives(); return $apply(optionSpace, n, []) === optionNs ? $apply(optionLocal, n, []) : ''; };
    var optionsOf = function(sel){
      var a = [];
      optionNatives();
      (function walk(n, grouped){ for (var c = $apply(optionFirst, n, []); c; c = $apply(optionNext, c, [])){ if ($apply(optionKind, c, []) !== 1) continue; var name = optionName(c);
        if (name === 'option') $c.push(a, c);
        else if (name === 'optgroup') { if (!grouped) walk(c, true); }
        else if (name !== 'select' && name !== 'datalist' && name !== 'hr') walk(c, grouped); } })(sel, false);
      a.item = function(i){ return this[i] || null; }; return a;
    };
    var ownerSelect = function(n){ var grouped = false, p = $apply(optionParent, n, []); while (p && $apply(optionKind, p, []) === 1){ var name = optionName(p); if (name === 'select') return p; if (name === 'datalist' || name === 'hr' || name === 'option') return null; if (name === 'optgroup') { if (grouped) return null; grouped = true; } p = $apply(optionParent, p, []); } return null; };
    // <option>: `value` falls back to text content (HTML spec), `selected`/`defaultSelected` track
    // the live + attribute state. React-DOM's updateOptions reads option.value and writes
    // option.selected for every option of a controlled <select>.
    var selection;
    (function(){
      var op = protoFor.HTMLOptionElement;
      Object.defineProperty(op, 'value', { configurable: true, get: function(){ var v = $getAttr(this, 'value'); return v == null ? (this.textContent || '') : v; }, set: function(v){ $setAttr(this, 'value', v == null ? '' : String(v)); } });
      var optionStates = new WeakMap(), emptySelects = new WeakSet();
      var isSelected = function(o){ return $c.has(optionStates,o) ? $c.get(optionStates,o) : $hasAttr(o, 'selected'); };
      var selectNumber = Number;
      var optionGroupDisabled = function(opt){
        var group = $apply(optionParent, opt, []);
        return group && $apply(optionKind, group, []) === 1 && optionName(group) === 'optgroup' && $hasAttr(group, 'disabled');
      };
      var firstEnabledOption = function(os){
        for (var j = 0; j < os.length; j++) {
          if (!$hasAttr(os[j], 'disabled') && !optionGroupDisabled(os[j])) return os[j];
        }
        return null;
      };
      var selectionNeedsFallback = function(chosen,multiple,sel){return !chosen.length&&!multiple&&selectNumber($getAttr(sel,'size')||0)<=1&&!$c.has(emptySelects,sel);};
      var selectionOf = function(sel){
        var os = optionsOf(sel), chosen = [];
        for (var i=0;i<os.length;i++) if (isSelected(os[i])) $c.push(chosen, os[i]);
        var multiple = $hasAttr(sel, 'multiple');
        if (!multiple && chosen.length > 1) chosen = [chosen[chosen.length-1]];
        if (selectionNeedsFallback(chosen,multiple,sel)) {
          var fallback = firstEnabledOption(os);
          if (fallback) $c.push(chosen, fallback);
        }
        return chosen;
      };
      Object.defineProperty(op, 'selected', { configurable: true, get: function(){
        var sel = ownerSelect(this); return sel ? $c.indexOf(selectionOf(sel), this) >= 0 : isSelected(this);
      }, set: function(v){
        bumpStyleState(); v = !!v; $c.set(optionStates,this, v);
        var sel = ownerSelect(this);
        if (sel && v) {
          $c.delete(emptySelects,sel);
          if (!$hasAttr(sel, 'multiple')) $c.forEach(optionsOf(sel), function(o){ if (o !== this) $c.set(optionStates,o, false); }, this);
        }
      } });
      selection = { selectionOf: selectionOf, optionStates: optionStates, emptySelects: emptySelects, isSelected: isSelected, optionsOf: optionsOf, ownerSelect: ownerSelect };
      __ns.selectSelection = selection; // captured (and deleted) by select_element.js
      Object.defineProperty(op, 'defaultSelected', { configurable: true, get: function(){ return $hasAttr(this, 'selected'); }, set: function(v){ if (v) $setAttr(this, 'selected',''); else $removeAttr(this, 'selected'); } });
      Object.defineProperty(op, 'disabled', { configurable: true, get: function(){ return $hasAttr(this, 'disabled'); }, set: function(v){ if (v) $setAttr(this, 'disabled',''); else $removeAttr(this, 'disabled'); } });
      Object.defineProperty(op, 'text', { configurable: true, get: function(){ return this.textContent || ''; }, set: function(v){ this.textContent = v; } });
    })();
    // <select>: `options` (HTMLOptionsCollection — the crash: React-DOM reads node.options.length),
    // plus `value`/`selectedIndex` derived from the options' selected state.
    (function(){
      var sp = protoFor.HTMLSelectElement;
      var opts = optionsOf;
      Object.defineProperty(sp, 'options', { configurable: true, get: function(){ return opts(this); } });

      Object.defineProperty(sp, 'selectedOptions', { configurable: true, get: function(){ var list = selection.selectionOf(this); list.item = function(i){ return this[i] || null; }; return list; } });
      Object.defineProperty(sp, 'selectedIndex', { configurable: true,
        get: function(){ var chosen = selection.selectionOf(this); return chosen.length ? opts(this).indexOf(chosen[0]) : -1; },
        set: function(idx){ var o = opts(this), index = Number(idx) >> 0; bumpStyleState();
          $c.add(selection.emptySelects,this);
          for (var i=0;i<o.length;i++) { $c.set(selection.optionStates,o[i], i === index); }
          if (index >= 0 && index < o.length) $c.delete(selection.emptySelects,this);
        } });
      Object.defineProperty(sp, 'value', { configurable: true,
        get: function(){ var chosen = selection.selectionOf(this); return chosen.length ? chosen[0].value : ''; },
        set: function(v){ var o = opts(this), s = String(v), hit = false; bumpStyleState();
          $c.add(selection.emptySelects,this);
          for (var i=0;i<o.length;i++){ var m = !hit && o[i].value === s; $c.set(selection.optionStates,o[i], m); if (m) hit = true; }
          if (hit) $c.delete(selection.emptySelects,this);
        } });
    })();
    var formValueApply=Reflect.apply,formValueGet=WeakMap.prototype.get,formValueHas=WeakMap.prototype.has;
    var formValueText=__ns.domAccessor('textContent',false,'Node');
    var formControlIsReflected=/^(hidden|button|submit|reset|image|checkbox|radio)$/;
    var formControlSelectValue=function(element){var chosen=selection.selectionOf(element);return chosen.length?chosen[0].value:'';};
    __ns.formControlValue=function(element){
      var tag=formValueApply(sheetTagName,element,[]),type=formValueApply(sheetGetAttribute,element,['type'])||'text';
      var file=fileControls?fileControls.value(element):undefined;if(file!==undefined)return file;
      if(tag==='SELECT')return formControlSelectValue(element);
      if(formValueApply(formValueHas,controlValues,[element])&&!$c.test(formControlIsReflected,$c.toLowerCase(type)))return formValueApply(formValueGet,controlValues,[element]);
      return tag==='TEXTAREA'?formValueApply(formValueText,element,[]):formValueApply(sheetGetAttribute,element,['value'])||'';
    };
    var resetControlDelete=WeakMap.prototype.delete,resetSelectDelete=WeakSet.prototype.delete;
    __ns.formSelectedOptions=function(element){return selection.selectionOf(element);};
    __ns.formResetControl=function(element){
      bumpStyleState();
      formValueApply(resetControlDelete,controlValues,[element]);
      if(fileControls)fileControls.reset(element);
      if(formValueApply(sheetTagName,element,[])==='INPUT')checkedNative(2,element);
      if(formValueApply(sheetTagName,element,[])==='SELECT'){
        var options=optionsOf(element);for(var index=0;index<options.length;index++)formValueApply(resetControlDelete,selection.optionStates,[options[index]]);
        formValueApply(resetSelectDelete,selection.emptySelects,[element]);
      }
    };
    var cloneChildren=__ns.domAccessor('childNodes',false,'Node'),cloneNodeType=__ns.domAccessor('nodeType',false,'Node');
    var cloneApply=Reflect.apply,cloneDefine=Object.defineProperty;
    var cloneControlInput=function(source,clone){if(checkedNative(3,source))checkedNative(1,clone,checkedNative(0,source));checkedNative(4,clone);if(fileControls)fileControls.clone(source,clone);};
    var cloneControlPair=function(source,clone){
      if($c.has(controlValues,source))clone.value=$c.get(controlValues,source);
      if(cloneApply(cloneNodeType,source,[])===1&&formValueApply(sheetTagName,source,[])==='INPUT')cloneControlInput(source,clone);
      if($c.has(selection.optionStates,source))$c.set(selection.optionStates,clone,$c.get(selection.optionStates,source));
      if($c.has(selection.emptySelects,source))$c.add(selection.emptySelects,clone);
    };
    __ns.cloneControlState = function cloneControlState(source, clone, deep){
      var pending=[[source,clone]];
      for(var index=0;index<pending.length;index++){
        var pair=pending[index];cloneControlPair(pair[0],pair[1]);
        if(deep){
          var originals=cloneApply(cloneChildren,pair[0],[]),copies=cloneApply(cloneChildren,pair[1],[]);
          for(var i=0;i<originals.length&&i<copies.length;i++)cloneDefine(pending,pending.length,{__proto__:null,value:[originals[i],copies[i]],writable:true,configurable:true,enumerable:true});
        }
      }
    };
    // <label>: htmlFor + control resolution. testing-library's getByLabelText filters
    // querySelectorAll('label') by `label.control === element`, so without `control` NO label ever
    // associates and every form field fails with "no form control was found associated to that label".
    (function(){
      var lp = protoFor.HTMLLabelElement;
      var labelableTags={__proto__:null,BUTTON:true,SELECT:true,TEXTAREA:true,METER:true,OUTPUT:true,PROGRESS:true};
      var labelable = function(el){ if (!el || el.nodeType !== 1) return false; var tg = String(el.tagName).toUpperCase(); if (tg === 'INPUT') return (el.getAttribute('type') || 'text').toLowerCase() !== 'hidden'; return !!labelableTags[tg]; };
      Object.defineProperty(lp, 'htmlFor', { configurable: true, get: function(){ return $getAttr(this, 'for') || ''; }, set: function(v){ $setAttr(this, 'for', v == null ? '' : String(v)); } });
      Object.defineProperty(lp, 'control', { configurable: true, get: function(){
        var f = $getAttr(this, 'for');
        if (f) { var el = this.ownerDocument.getElementById(f); return labelable(el) ? el : null; }
        var list = this.querySelectorAll('input, button, select, textarea, meter, output, progress');
        for (var i=0;i<list.length;i++) if (labelable(list[i])) return list[i];
        return null;
      } });
    })();
    var CTRL = { input:'HTMLInputElement', textarea:'HTMLTextAreaElement', select:'HTMLSelectElement', option:'HTMLOptionElement', button:'HTMLButtonElement', label:'HTMLLabelElement', canvas:'HTMLCanvasElement' };
    // <a> interface prototype with a `click` that dispatches (NOT an own per-instance method), so
    // download flows that spy on HTMLAnchorElement.prototype.click (createObjectURL + a.click) work —
    // an own native click would shadow the spy. Scoped to anchors only (the global variant regressed
    // select-like components, whose own native click is load-bearing).
    if (typeof g.HTMLAnchorElement !== 'function') g.HTMLAnchorElement = function(){};
    var anchorProto = Object.create(baseProto);
    anchorProto.click = function(){ try { var C = g.MouseEvent || g.PointerEvent || g.Event; this.dispatchEvent(new C('click', { bubbles: true, cancelable: true })); } catch(e){} };
    // HTMLHyperlinkElementUtils — origin/protocol/host/hostname/port/pathname/search/hash decomposed
    // from href (resolved against the document base). React/Next read anchorEl.origin during render;
    // undefined → "Cannot read properties of undefined (reading 'origin')". href already reflects.
    (function(){
      var anchorBase = function(){ return (g.location && g.location.href) || 'http://localhost/'; };
      var parse = function(el){ try { return new g.URL(el.href || '', anchorBase()); } catch(e){ return null; } };
      ['origin','protocol','host','hostname','port','pathname','search','hash','username','password'].forEach(function(p){
        Object.defineProperty(anchorProto, p, { configurable: true,
          get: function(){ var u = parse(this); if (!u) return p === 'pathname' ? '/' : ''; var v = u[p]; return v == null ? '' : v; },
          set: function(v){ var u = parse(this); if (!u) return; try { u[p] = v; this.href = u.href; } catch(e){} } });
      });
    })();
    g.HTMLAnchorElement.prototype = anchorProto;
    // Apply the control interface prototype + own value/checked to an element by its tag. Used by
    // createElement AND cloneNode (clones must keep type/value/checked/selected accessors — e.g.
    // userEvent's isValidDateOrTimeValue clones a date input, assigns, and checks the value stuck).
    // The DOM value-change tracker: browsers expose an internal "is the IDL value dirty since the
    // framework last saw it" signal that React/Preact/etc. read via the conventional `_valueTracker`
    // slot (a tracker is the standard contract, not React-private). We attach a spec-shaped one whose
    // `currentValue` advances only when value is set through the OWN accessor (programmatic / a
    // controlled-component restore) — NOT through the prototype accessor that user-event drives.
    // So user typing reads as "changed" (per-char onChange) while a controlled restore to the same
    // value reads as "unchanged" (no spurious trailing onChange — e.g. date inputs commit once).
    var attachValueTracker = function(el, field, protoDesc){
      var current = '' + (protoDesc.get ? protoDesc.get.call(el) : '');
      Object.defineProperty(el, field, { configurable: true, enumerable: false,
        get: function(){ return protoDesc.get.call(this); },
        set: function(v){ current = '' + v; protoDesc.set.call(this, v); } });
      // The tracker tracks the `value` field (text inputs). For checkbox/radio — which track `checked`
      // — el_set_attribute('type', …) strips this `value`+`_valueTracker` once the type is known, so
      // a framework falls back to its own checked path.
      el._valueTracker = { getValue: function(){ return current; }, setValue: function(v){ current = '' + v; }, stopTracking: function(){ el._valueTracker = null; try { Object.defineProperty(el, field, protoDesc); } catch(e){} } };
    };
    var controlTagName=__ns.domAccessor('tagName',false,'Element');
    var controlTracked={__proto__:null,input:true,textarea:true};
    var controlClickDrop={__proto__:null,input:true,textarea:true,button:true};
    var applyControlCtrl=function(el,t){
      Object.setPrototypeOf(el,protoFor[CTRL[t]]);
      if(controlTracked[t]){attachValueTracker(el,'value',valDesc);Object.defineProperty(el,'checked',checkedDesc);}
      if(controlClickDrop[t]&&Object.prototype.hasOwnProperty.call(el,'click'))delete el.click;
    };
    var applyControlProto = function(el){
      try {
        var t = $call($lower, String($call(controlTagName, el) || ''));
        if (CTRL[t]) applyControlCtrl(el,t);
        else if (t === 'a') {
          Object.setPrototypeOf(el, anchorProto);
          if (Object.prototype.hasOwnProperty.call(el, 'click')) delete el.click;
        }
      } catch(e){}
    };
    // <canvas>.getContext('2d') — a 2D context that does NOT rasterize but RECORDS the draw
    // operations (with the paint state at each) so readback (toDataURL/getImageData/toBlob)
    // is DETERMINISTIC and CONTENT-DEPENDENT: identical draws always hash to identical bytes,
    // different draws to different bytes. An empty `data:image/png;base64,` or an all-zero
    // getImageData is a canvas-fingerprint dead tell (real canvases vary with content); a
    // stable content-derived hash reads like a real — if device-invariant — canvas. There is
    // no GPU/font rasterizer here, so the output is synthetic: honest about being computed
    // from the op log, not a specific device's pixels. Covers signature pads/charts too.
    var __cvHash = function(str){
      // FNV-1a (32-bit), avalanched at the end.
      var h = 0x811c9dc5;
      for (var i = 0; i < str.length; i++) { h ^= $c.charCodeAt(str, i); h = $imul(h, 0x01000193) >>> 0; }
      h ^= h >>> 16; h = $imul(h, 0x85ebca6b) >>> 0; h ^= h >>> 13; return h >>> 0;
    };
    var __CV_B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
    var __cvB64 = function(bytes){
      var out = '';
      for (var i = 0; i < bytes.length; i += 3) {
        var a = bytes[i], b = (i+1 < bytes.length) ? bytes[i+1] : 0, c = (i+2 < bytes.length) ? bytes[i+2] : 0;
        var n = (a << 16) | (b << 8) | c;
        out += __CV_B64[(n>>18)&63] + __CV_B64[(n>>12)&63] + ((i+1 < bytes.length) ? __CV_B64[(n>>6)&63] : '=') + ((i+2 < bytes.length) ? __CV_B64[n&63] : '=');
      }
      return out;
    };
    // Deterministic byte stream of length n from a 32-bit seed (an LCG).
    var __cvBytes = function(seed, n){
      var b = new Array(n), x = (seed ^ 0x9e3779b9) >>> 0;
      for (var i = 0; i < n; i++) { x = ($imul(x, 1664525) + 1013904223) >>> 0; b[i] = (x >>> 16) & 0xff; }
      return b;
    };
    var __cvSeed = function(canvas, ctx, extra){
      var w = (canvas && canvas.width) || 300, h = (canvas && canvas.height) || 150;
      var ops = (ctx && ctx._ops) ? ctx._ops : [];
      return __cvHash(w + 'x' + h + '|' + (extra || '') + '|' + $jsonStringify(ops));
    };
    // fillStyle / strokeStyle / shadowColor (turbo-dom#108, canvas_style_chrome156.json): a
    // string parses with turbo-dom's CSS colour parser and reads back in Chrome's
    // serialization; a value the canvas cannot parse is ignored. `currentcolor` resolves once,
    // at assignment, to the connected canvas's own inline `color` (Chrome 156 ignores
    // inherited and stylesheet colours), else black. The op log records the resolved colour.
    var __cvPaints = new WeakSet();
    var __cvColor = function(canvas, text){
      if (String(text).trim().toLowerCase() === 'currentcolor') {
        var connected = false, inline = '';
        try { connected = computedStyleNativeConnected.call(canvas); } catch(e){}
        if (connected) { try { inline = canvas.style.getPropertyValue('color'); } catch(e){} }
        return (inline && canvasColorNative(inline)) || '#000000';
      }
      return canvasColorNative(text);
    };
    var __cvStyles = function(ctx, canvas){
      var state = { fillStyle: '#000000', strokeStyle: '#000000', shadowColor: 'rgba(0, 0, 0, 0)' };
      Object.keys(state).forEach(function(name){
        var paints = name !== 'shadowColor';
        Object.defineProperty(ctx, name, { configurable: true, enumerable: true,
          get: function(){ return state[name]; },
          set: function(value){
            if (paints && value !== null && typeof value === 'object' && $c.has(__cvPaints, value)) { state[name] = value; return; }
            var color = __cvColor(canvas, '' + value);
            if (color != null) state[name] = color;
          } });
      });
    };
    // TextMetrics (turbo-dom#237, canvas_text_metrics_interface_chrome156.json): a global,
    // non-constructible interface whose fields are enumerable prototype getters over the
    // instance's measured values; they throw "Illegal invocation" on any other receiver.
    var __tmFields = ['width','actualBoundingBoxLeft','actualBoundingBoxRight','fontBoundingBoxAscent',
      'fontBoundingBoxDescent','actualBoundingBoxAscent','actualBoundingBoxDescent','hangingBaseline',
      'alphabeticBaseline','ideographicBaseline'];
    var __tmValues = new WeakMap();
    var TextMetrics = function TextMetrics(){
      throw new TypeError(new.target ? "Failed to construct 'TextMetrics': Illegal constructor" : 'Illegal constructor');
    };
    var __tmProto = TextMetrics.prototype;
    __tmFields.forEach(function(name, index){
      var holder = {};
      Object.defineProperty(holder, name, { get: function(){
        var values = __tmValues.get(this);
        if (!values) throw new TypeError('Illegal invocation');
        return values[index];
      } });
      var get = Object.getOwnPropertyDescriptor(holder, name).get;
      Object.defineProperty(get, 'name', { value: 'get ' + name });
      Object.defineProperty(__tmProto, name, { get: get, enumerable: true, configurable: true });
    });
    Object.defineProperty(__tmProto, 'constructor', { value: TextMetrics, writable: true, configurable: true });
    Object.defineProperty(__tmProto, Symbol.toStringTag, { value: 'TextMetrics', configurable: true });
    Object.defineProperty(TextMetrics, 'prototype', { writable: false, enumerable: false, configurable: false });
    Object.defineProperty(g, 'TextMetrics', { value: TextMetrics, writable: true, enumerable: false, configurable: true });
    var __textMetrics = function(values){
      var metrics = Object.create(__tmProto);
      __tmValues.set(metrics, values);
      return metrics;
    };
    // Text attributes (turbo-dom#237, canvas_text_attributes_chrome156.json): letterSpacing /
    // wordSpacing take a CSS length (any <length> unit Chrome parses; no percentage, unitless
    // number, calc() or padding) and
    // read back lowercased with the number reserialized; the enum attributes accept only
    // their exact values. An invalid assignment is ignored. measureText passes them on.
    var __cvLength = /^([+-]?(?:\d*\.\d+|\d+)(?:e[+-]?\d+)?)([a-z]+)$/i;
    var __cvEnums = {
      fontKerning: ['auto', 'normal', 'none'],
      fontStretch: ['ultra-condensed', 'extra-condensed', 'condensed', 'semi-condensed', 'normal',
        'semi-expanded', 'expanded', 'extra-expanded', 'ultra-expanded'],
      textRendering: ['auto', 'optimizeSpeed', 'optimizeLegibility', 'geometricPrecision']
    };
    var __cvText = function(ctx){
      var state = { letterSpacing: '0px', wordSpacing: '0px', fontKerning: 'auto', fontStretch: 'normal', textRendering: 'auto' };
      Object.keys(state).forEach(function(name){
        var allowed = __cvEnums[name];
        Object.defineProperty(ctx, name, { configurable: true, enumerable: true,
          get: function(){ return state[name]; },
          set: function(value){
            var text = String(value);
            if (allowed) { if (allowed.indexOf(text) >= 0) state[name] = text; return; }
            var length = __cvLength.exec(text);
            if (length && canvasLengthUnitNative(length[2])) state[name] = String(Number(length[1])) + length[2].toLowerCase();
          } });
      });
    };
    var mkCanvasCtx = function(canvas){
      var noop = function(){};
      var paint = function(){ var p = { addColorStop: noop }; $c.add(__cvPaints, p); return p; };
      var lineDash = [], stack = [];
      var ctx = {
        canvas: canvas,
        _ops: [],
        // A dash with a negative or non-finite entry is ignored; an odd count is doubled.
        setLineDash: function(segments){
          var dash = Array.prototype.map.call(segments, Number);
          for (var i = 0; i < dash.length; i++) if (!isFinite(dash[i]) || dash[i] < 0) return;
          lineDash = dash.length % 2 ? dash.concat(dash) : dash;
        },
        getLineDash: function(){ return lineDash.slice(); },
        createLinearGradient: paint,
        createRadialGradient: paint,
        createConicGradient: paint,
        createPattern: function(){ var p = {}; $c.add(__cvPaints, p); return p; },
        getImageData: function(x,y,w,h){
          w = w||0; h = h||0;
          var data = new Uint8ClampedArray(Math.max(0, w*h*4));
          // Content-dependent, deterministic pixels derived from the op log — not all-zero.
          if (data.length) {
            var src = __cvBytes(__cvSeed(canvas, ctx, 'getImageData:'+x+','+y+','+w+','+h), Math.min(data.length, 4096));
            for (var i = 0; i < data.length; i++) data[i] = src[i % src.length];
          }
          return { data: data, width: w, height: h, colorSpace: 'srgb' };
        },
        createImageData: function(w,h){ if (w && w.width != null) { h = w.height; w = w.width; } return { data: new Uint8ClampedArray(Math.max(0,(w||0)*(h||0)*4)), width: w||0, height: h||0, colorSpace: 'srgb' }; },
        // measureText (turbo-dom#237, canvas_measure_text_chrome156.json): turbo-dom-layout
        // measures the run in the host's fonts like Chrome 156; Chrome 156 leaves the
        // emHeight* fields undefined.
        measureText: function(s){
          var text = String(s);
          this._ops.push(['measureText', text, this.font]);
          var m = canvasMeasureTextNative(text, String(this.font), String(this.textAlign), String(this.textBaseline), String(this.direction),
            this.letterSpacing, this.wordSpacing, this.fontKerning, this.textRendering);
          // Native order -> the TextMetrics field order.
          return __textMetrics([m[0], m[1], m[2], m[5], m[6], m[3], m[4], m[7], m[8], m[9]]);
        },
        isPointInPath: function(){ return false; }, isPointInStroke: function(){ return false; },
        lineWidth: 1, lineCap: 'butt', lineJoin: 'miter',
        miterLimit: 10, lineDashOffset: 0, shadowBlur: 0, shadowOffsetX: 0, shadowOffsetY: 0,
        font: '10px sans-serif', textAlign: 'start', textBaseline: 'alphabetic', direction: 'inherit',
        globalAlpha: 1, globalCompositeOperation: 'source-over', imageSmoothingEnabled: true, imageSmoothingQuality: 'low'
      };
      __cvStyles(ctx, canvas);
      __cvText(ctx);
      // Record-only paint/path methods: log the call (args + relevant paint state) so the
      // readback hash reflects what was drawn, but perform no rasterization.
      var recorded = ['fillRect','clearRect','strokeRect','beginPath','closePath','moveTo','lineTo',
        'bezierCurveTo','quadraticCurveTo','arc','arcTo','rect','roundRect','ellipse','fill','stroke',
        'clip','save','restore','scale','rotate','translate','transform','setTransform','resetTransform',
        'drawImage','putImageData','fillText','strokeText','drawFocusIfNeeded'];
      recorded.forEach(function(name){
        ctx[name] = function(){
          var a = Array.prototype.slice.call(arguments).map(function(v){ return (v && typeof v === 'object') ? (v.tagName || v.nodeName || 'obj') : v; });
          this._ops.push([name, a, this.fillStyle, this.strokeStyle, this.font, this.textBaseline, this.textAlign, this.globalAlpha, this.globalCompositeOperation]);
        };
      });
      // The drawing-state stack (turbo-dom#110): save() snapshots every state attribute and the
      // line dash, restore() brings the last snapshot back (an empty stack is a no-op). Both
      // still log their call first, so the raster replay keeps its own matching stack.
      var drawingState = ['fillStyle','strokeStyle','shadowColor','globalAlpha','globalCompositeOperation',
        'lineWidth','lineCap','lineJoin','miterLimit','lineDashOffset','shadowBlur','shadowOffsetX',
        'shadowOffsetY','font','textAlign','textBaseline','direction','imageSmoothingEnabled','imageSmoothingQuality',
        'letterSpacing','wordSpacing','fontKerning','fontStretch','textRendering'];
      var recordSave = ctx.save, recordRestore = ctx.restore;
      ctx.save = function(){
        recordSave.apply(this, arguments);
        var saved = { lineDash: lineDash };
        for (var i = 0; i < drawingState.length; i++) saved[drawingState[i]] = ctx[drawingState[i]];
        stack.push(saved);
      };
      ctx.restore = function(){
        recordRestore.apply(this, arguments);
        var saved = stack.pop();
        if (!saved) return;
        for (var i = 0; i < drawingState.length; i++) ctx[drawingState[i]] = saved[drawingState[i]];
        lineDash = saved.lineDash;
      };
      return ctx;
    };
    // WebGL: a COHERENT headless-Chrome SwiftShader context (ANGLE over Vulkan). Headless
    // Chrome has NO GPU and renders WebGL through SwiftShader — a legitimate, ubiquitous real
    // fingerprint (every GPU-less Chrome reports exactly this), so emulating it is far less of
    // a tell than a null context (which reads as WebGL blocked/disabled). Values below are
    // SwiftShader's real ones: VENDOR/RENDERER + UNMASKED_* (via WEBGL_debug_renderer_info),
    // VERSION/GLSL strings as Chrome reports, the numeric limits, the extension list, and
    // shader precision. Readback (readPixels/toDataURL/toBlob) is DETERMINISTIC + content-
    // dependent (hashes the GL op log, like the 2D path) — synthetic, not a real rasterizer
    // (a real GL raster is a bigger Tier-2 job); honest about being computed from the op log.
    var GLC = {
      VENDOR:0x1F00, RENDERER:0x1F01, VERSION:0x1F02, SHADING_LANGUAGE_VERSION:0x8B8C,
      MAX_TEXTURE_SIZE:0x0D33, MAX_CUBE_MAP_TEXTURE_SIZE:0x851C, MAX_RENDERBUFFER_SIZE:0x84E8,
      MAX_VIEWPORT_DIMS:0x0D3A, MAX_TEXTURE_IMAGE_UNITS:0x8872, MAX_COMBINED_TEXTURE_IMAGE_UNITS:0x8B4D,
      MAX_VERTEX_TEXTURE_IMAGE_UNITS:0x8B4C, MAX_VERTEX_ATTRIBS:0x8869, MAX_VERTEX_UNIFORM_VECTORS:0x8DFB,
      MAX_FRAGMENT_UNIFORM_VECTORS:0x8DFD, MAX_VARYING_VECTORS:0x8DFC,
      ALIASED_LINE_WIDTH_RANGE:0x846E, ALIASED_POINT_SIZE_RANGE:0x846D,
      RED_BITS:0x0D52, GREEN_BITS:0x0D53, BLUE_BITS:0x0D54, ALPHA_BITS:0x0D55, DEPTH_BITS:0x0D56,
      STENCIL_BITS:0x0D57, SUBPIXEL_BITS:0x0D50, SAMPLES:0x80A9, SAMPLE_BUFFERS:0x80A8,
      MAX_TEXTURE_MAX_ANISOTROPY_EXT:0x84FF, TEXTURE_MAX_ANISOTROPY_EXT:0x84FE,
      UNMASKED_VENDOR_WEBGL:0x9245, UNMASKED_RENDERER_WEBGL:0x9246,
      HIGH_FLOAT:0x8DF2, MEDIUM_FLOAT:0x8DF1, LOW_FLOAT:0x8DF0, HIGH_INT:0x8DF5, MEDIUM_INT:0x8DF4, LOW_INT:0x8DF3,
      MAX_3D_TEXTURE_SIZE:0x8073, MAX_ARRAY_TEXTURE_LAYERS:0x88FF, MAX_DRAW_BUFFERS:0x8824,
      MAX_COLOR_ATTACHMENTS:0x8CDF, MAX_SAMPLES:0x8D57, MAX_UNIFORM_BUFFER_BINDINGS:0x8A2F,
      MAX_VERTEX_UNIFORM_BLOCKS:0x8A2B, MAX_FRAGMENT_UNIFORM_BLOCKS:0x8A2D,
      MAX_ELEMENTS_INDICES:0x80E9, MAX_ELEMENTS_VERTICES:0x80E8, VERTEX_ARRAY_BINDING_OES:0x85B5,
    };
    var GL_EXT1 = ['ANGLE_instanced_arrays','EXT_blend_minmax','EXT_color_buffer_half_float',
      'EXT_disjoint_timer_query','EXT_float_blend','EXT_frag_depth','EXT_shader_texture_lod','EXT_sRGB',
      'EXT_texture_compression_bptc','EXT_texture_compression_rgtc','EXT_texture_filter_anisotropic',
      'OES_element_index_uint','OES_fbo_render_mipmap','OES_standard_derivatives','OES_texture_float',
      'OES_texture_float_linear','OES_texture_half_float','OES_texture_half_float_linear',
      'OES_vertex_array_object','WEBGL_color_buffer_float','WEBGL_compressed_texture_s3tc',
      'WEBGL_compressed_texture_s3tc_srgb','WEBGL_debug_renderer_info','WEBGL_debug_shaders',
      'WEBGL_depth_texture','WEBGL_draw_buffers','WEBGL_lose_context','WEBGL_multi_draw'];
    var GL_EXT2 = ['EXT_color_buffer_float','EXT_color_buffer_half_float','EXT_disjoint_timer_query_webgl2',
      'EXT_float_blend','EXT_texture_compression_bptc','EXT_texture_compression_rgtc',
      'EXT_texture_filter_anisotropic','EXT_texture_norm16','KHR_parallel_shader_compile',
      'OES_draw_buffers_indexed','OES_texture_float_linear','WEBGL_compressed_texture_s3tc',
      'WEBGL_compressed_texture_s3tc_srgb','WEBGL_debug_renderer_info','WEBGL_debug_shaders',
      'WEBGL_lose_context','WEBGL_multi_draw','WEBGL_provoking_vertex'];
    // Build the WebGL `getParameter` table for the requested context version. Moving the
    // isGL2 branches here keeps mkWebGLCtx under the cc limit.
    var mkWebGLParams = function(isGL2){
      var C = GLC;
      var params = {};
      params[C.VENDOR] = 'WebKit'; params[C.RENDERER] = 'WebKit WebGL';
      params[C.VERSION] = isGL2 ? 'WebGL 2.0 (OpenGL ES 3.0 Chromium)' : 'WebGL 1.0 (OpenGL ES 2.0 Chromium)';
      params[C.SHADING_LANGUAGE_VERSION] = isGL2 ? 'WebGL GLSL ES 3.00 (OpenGL ES GLSL ES 3.0 Chromium)' : 'WebGL GLSL ES 1.0 (OpenGL ES GLSL ES 1.0 Chromium)';
      params[C.UNMASKED_VENDOR_WEBGL] = 'Google Inc. (Google)';
      params[C.UNMASKED_RENDERER_WEBGL] = 'ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)';
      params[C.MAX_TEXTURE_SIZE] = 8192; params[C.MAX_CUBE_MAP_TEXTURE_SIZE] = 8192;
      params[C.MAX_RENDERBUFFER_SIZE] = 8192; params[C.MAX_VIEWPORT_DIMS] = new Int32Array([8192, 8192]);
      params[C.MAX_TEXTURE_IMAGE_UNITS] = 16; params[C.MAX_COMBINED_TEXTURE_IMAGE_UNITS] = 32;
      params[C.MAX_VERTEX_TEXTURE_IMAGE_UNITS] = 16; params[C.MAX_VERTEX_ATTRIBS] = 16;
      params[C.MAX_VERTEX_UNIFORM_VECTORS] = 4096; params[C.MAX_FRAGMENT_UNIFORM_VECTORS] = 4096;
      params[C.MAX_VARYING_VECTORS] = 30;
      params[C.ALIASED_LINE_WIDTH_RANGE] = new Float32Array([1, 1]);
      params[C.ALIASED_POINT_SIZE_RANGE] = new Float32Array([1, 1023]);
      params[C.MAX_TEXTURE_MAX_ANISOTROPY_EXT] = 16;
      params[C.RED_BITS] = 8; params[C.GREEN_BITS] = 8; params[C.BLUE_BITS] = 8; params[C.ALPHA_BITS] = 8;
      params[C.DEPTH_BITS] = 24; params[C.STENCIL_BITS] = 0; params[C.SUBPIXEL_BITS] = 4;
      params[C.SAMPLES] = 0; params[C.SAMPLE_BUFFERS] = 0;
      if (isGL2) {
        params[C.MAX_3D_TEXTURE_SIZE] = 2048; params[C.MAX_ARRAY_TEXTURE_LAYERS] = 2048;
        params[C.MAX_DRAW_BUFFERS] = 8; params[C.MAX_COLOR_ATTACHMENTS] = 8; params[C.MAX_SAMPLES] = 4;
        params[C.MAX_UNIFORM_BUFFER_BINDINGS] = 72; params[C.MAX_VERTEX_UNIFORM_BLOCKS] = 12;
        params[C.MAX_FRAGMENT_UNIFORM_BLOCKS] = 12; params[C.MAX_ELEMENTS_INDICES] = 0x7FFFFFFF;
        params[C.MAX_ELEMENTS_VERTICES] = 0x7FFFFFFF;
      }
      return params;
    };
    function mkWebGLExtension(name, C, noop){
      if (name === 'WEBGL_debug_renderer_info') return { UNMASKED_VENDOR_WEBGL: C.UNMASKED_VENDOR_WEBGL, UNMASKED_RENDERER_WEBGL: C.UNMASKED_RENDERER_WEBGL };
      if (name === 'EXT_texture_filter_anisotropic') return { MAX_TEXTURE_MAX_ANISOTROPY_EXT: C.MAX_TEXTURE_MAX_ANISOTROPY_EXT, TEXTURE_MAX_ANISOTROPY_EXT: C.TEXTURE_MAX_ANISOTROPY_EXT };
      if (name === 'OES_vertex_array_object') return { createVertexArrayOES:function(){return{};}, deleteVertexArrayOES:noop, bindVertexArrayOES:noop, isVertexArrayOES:function(){return false;}, VERTEX_ARRAY_BINDING_OES:C.VERTEX_ARRAY_BINDING_OES };
      if (name === 'WEBGL_lose_context') return { loseContext:noop, restoreContext:noop };
      if (name === 'ANGLE_instanced_arrays') return { drawArraysInstancedANGLE:noop, drawElementsInstancedANGLE:noop, vertexAttribDivisorANGLE:noop, VERTEX_ATTRIB_ARRAY_DIVISOR_ANGLE:0x88FE };
      return {};
    }
    function mkWebGLShaderPrecision(precisionType, C){
      var isFloat = (precisionType === C.HIGH_FLOAT || precisionType === C.MEDIUM_FLOAT || precisionType === C.LOW_FLOAT);
      return isFloat ? { rangeMin:127, rangeMax:127, precision:23 } : { rangeMin:31, rangeMax:30, precision:0 };
    }
    var mkWebGLCtx = function(canvas, isGL2){
      var C = GLC, noop = function(){};
      var params = mkWebGLParams(isGL2);
      var EXT = isGL2 ? GL_EXT2 : GL_EXT1;
      var gl = { canvas: canvas, _ops: [],
        drawingBufferWidth: (canvas && canvas.width) || 300, drawingBufferHeight: (canvas && canvas.height) || 150,
        drawingBufferColorSpace: 'srgb', unpackColorSpace: 'srgb' };
      for (var k in C) gl[k] = C[k];
      gl.getParameter = function(p){ return (p in params) ? params[p] : null; };
      gl.getContextAttributes = function(){ return { alpha:true, antialias:true, depth:true, desynchronized:false,
        failIfMajorPerformanceCaveat:false, powerPreference:'default', premultipliedAlpha:true,
        preserveDrawingBuffer:false, stencil:false, xrCompatible:false }; };
      gl.getSupportedExtensions = function(){ return EXT.slice(); };
      gl.getExtension = function(name){
        if (EXT.indexOf(name) < 0) return null;
        return mkWebGLExtension(name, C, noop);
      };
      gl.getShaderPrecisionFormat = function(shaderType, precisionType){
        return mkWebGLShaderPrecision(precisionType, C);
      };
      // Record-only GL commands so readback reflects what was drawn (no rasterization).
      ['clear','clearColor','drawArrays','drawElements','drawArraysInstanced','drawElementsInstanced',
       'viewport','scissor','useProgram','bindBuffer','bufferData','bufferSubData','vertexAttribPointer',
       'enableVertexAttribArray','uniform1f','uniform2f','uniform3f','uniform4f','uniform1i','uniformMatrix4fv',
       'activeTexture','bindTexture','texImage2D','texSubImage2D','texParameteri','enable','disable',
       'blendFunc','depthFunc','flush','finish','readBuffer'].forEach(function(n){
        gl[n] = function(){ var a = Array.prototype.slice.call(arguments).map(function(v){ return (v && typeof v === 'object') ? (v.byteLength != null ? ('buf' + v.byteLength) : (v.length != null ? ('arr' + v.length) : 'obj')) : v; }); this._ops.push([n, a]); };
      });
      ['createBuffer','createProgram','createShader','createTexture','createFramebuffer','createRenderbuffer','createVertexArray','createSampler'].forEach(function(n){ gl[n] = function(){ return { __glObj:n }; }; });
      ['deleteBuffer','deleteProgram','deleteShader','deleteTexture','deleteFramebuffer','deleteRenderbuffer','deleteVertexArray','shaderSource','compileShader','attachShader','linkProgram','validateProgram','detachShader','generateMipmap','pixelStorei','framebufferTexture2D','bindFramebuffer','bindRenderbuffer','bindVertexArray','renderbufferStorage','colorMask','depthMask','frontFace','cullFace','lineWidth','hint','stencilFunc','stencilOp','blendEquation','blendFuncSeparate','texParameterf','clearDepth','clearStencil','depthRange','sampleCoverage','stencilMask'].forEach(function(n){ gl[n] = noop; });
      gl.getShaderParameter = function(){ return true; };
      gl.getProgramParameter = function(){ return true; };
      gl.getShaderInfoLog = function(){ return ''; };
      gl.getProgramInfoLog = function(){ return ''; };
      gl.getError = function(){ return 0; };
      gl.getAttribLocation = function(){ return 0; };
      gl.getUniformLocation = function(){ return { __loc:true }; };
      gl.isContextLost = function(){ return false; };
      gl.checkFramebufferStatus = function(){ return 0x8CD5; }; // FRAMEBUFFER_COMPLETE
      mkWebGLReadPixels(gl, canvas);
      mkWebGLSetProto(gl, isGL2);
      return gl;
    };
    function mkWebGLReadPixels(gl, canvas){
      gl.readPixels = function(x,y,w,h,fmt,type,pixels){
        if (pixels && pixels.length){
          var src = __cvBytes(__cvSeed(canvas, gl, 'readPixels:'+x+','+y+','+w+','+h+','+fmt+','+type), Math.min(pixels.length, 4096));
          for (var i = 0; i < pixels.length; i++) pixels[i] = src[i % src.length];
        }
      };
    }
    function mkWebGLSetProto(gl, isGL2){
      try { var P = isGL2 ? g.WebGL2RenderingContext : g.WebGLRenderingContext; if (P && P.prototype) Object.setPrototypeOf(gl, P.prototype); } catch(e){}
    }
    // <canvas> methods live on HTMLCanvasElement.prototype (not own) so tests can mock
    // HTMLCanvasElement.prototype.getContext / getBoundingClientRect (signature pads) and the mock
    // isn't shadowed by an own method.
    (function(){
      var cp = protoFor.HTMLCanvasElement;
      // The readback seed uses whichever context recorded ops (2D or WebGL).
      var activeCtx = function(cv){ return cv.__ctx2d || cv.__ctxgl2 || cv.__ctxgl || null; };
      cp.getContext = function(kind){
        var k = String(kind);
        if (k === '2d') { if (!this.__ctx2d) this.__ctx2d = mkCanvasCtx(this); return this.__ctx2d; }
        if (k === 'webgl' || k === 'experimental-webgl') { if (!this.__ctxgl) this.__ctxgl = mkWebGLCtx(this, false); return this.__ctxgl; }
        if (k === 'webgl2' || k === 'experimental-webgl2') { if (!this.__ctxgl2) this.__ctxgl2 = mkWebGLCtx(this, true); return this.__ctxgl2; }
        return null;
      };
      cp.toDataURL = function(type){
        var mime = (typeof type === 'string' && type) ? type : 'image/png';
        var seed = __cvSeed(this, activeCtx(this), 'toDataURL:'+mime);
        return 'data:' + mime + ';base64,' + __cvB64(__cvBytes(seed, 54));
      };
      cp.toBlob = function(cb){ var type = arguments[1]; // toBlob(callback, optional type, optional quality).length === 1
        if (!cb) return;
        var mime = (typeof type === 'string' && type) ? type : 'image/png';
        var bytes = __cvBytes(__cvSeed(this, activeCtx(this), 'toBlob:'+mime), 64);
        var B = g.Blob;
        if (typeof B === 'function') {
          var str = ''; for (var i = 0; i < bytes.length; i++) str += String.fromCharCode(bytes[i]);
          try { cb(new B([str], { type: mime })); return; } catch(e){}
        }
        cb(null);
      };
      cp.getBoundingClientRect = function(){ return { x:0, y:0, top:0, left:0, right:0, bottom:0, width:0, height:0, toJSON:function(){ return this; } }; };
    })();
    d.createElement = ({ createElement(tag){
      var el = $apply(orig,this,arguments); var t = $call($lower, String($call(createdLocalName, el)));
      applyControlProto(el);
      // canvas: drop the own native getBoundingClientRect so it resolves to the (mockable)
      // HTMLCanvasElement.prototype version; getContext/toDataURL/toBlob are on the proto too.
      if (t === 'canvas') { try { if (Object.prototype.hasOwnProperty.call(el, 'getBoundingClientRect')) delete el.getBoundingClientRect; } catch(e){} }
      return el;
    } }).createElement;
    // NOTE: cloneNode re-applies the control prototype natively (el_clone_node copies the source's
    // prototype to the clone), so a cloned <input>/<select> keeps its type/value/checked accessors
    // (userEvent's isValidDateOrTimeValue clones a date input, assigns, and checks the value stuck).
    void applyControlProto;
    var documentSheetsQuery=d.querySelectorAll,documentSheetOf=function(element){var owner=$gopd($O.getPrototypeOf(element),'sheet');return owner&&owner.get?$apply(owner.get,element,[]):element.sheet;};
    var documentSheets=cssList('StyleSheetList',function(){var list=[],els=$apply(documentSheetsQuery,d,['style,link']);for(var i=0;i<els.length;i++){var sheet=documentSheetOf(els[i]);if(sheet)$c.push(list,sheet);}return list;});
    documentCSSRecord.cssHost.readSheets=function(){return cssRecord(documentSheets).items;};
    // Private @keyframes export for CSS animations (turbo-dom#187): name -> [{selector, values:[[longhand, text]]}],
    // read from the rule records (never page-visible properties); a later rule of a name wins.
    __ns.cssKeyframes=function(){var out={__proto__:null},sheets=cssRecord(documentSheets).items;
      function frames(record){var list=[],kids=cssItems(record.cssRules);for(var k=0;k<kids.length;k++){var kid=cssRuleRecord(kids[k]);if(kid.type!==1)continue;var decl=declarationRecord(kid.style),values=[];for(var n=0;n<decl.names.length;n++)$c.push(values,[decl.names[n],cssMapGet(decl.values,decl.names[n]).text]);$c.push(list,{selector:kid.selectorText,values:values});}return list;}
      function visit(rules){var items=cssItems(rules);for(var i=0;i<items.length;i++){var record=cssRuleRecord(items[i]),name;if(record.type===7&&record.cssRules&&(name=$c.exec(/^@(?:-webkit-)?keyframes\s+["']?([^"']+?)["']?\s*$/i,record.prelude)))out[name[1]]=frames(record);}}
      for(var s=0;s<sheets.length;s++){var sheet=sheetRecord(sheets[s]);if(!sheet.disabled)visit(sheet.rules);}return out;};
    __ns.styleSheetList=function(refresh){return cssList('StyleSheetList',refresh);};
    try { Object.defineProperty(d, 'styleSheets', { configurable: true, get: function(){return documentSheets;} }); } catch(e){}

    // Alias the shim interface-constructor `.prototype`s onto the REAL element/document prototypes,
    // so a test patching HTMLElement.prototype.requestFullscreen / Document.prototype.exitFullscreen
    // (DataTableCore fullscreen) reaches our instances. Only affects methods that aren't own per
    // instance (own native methods still shadow). The genuine fullscreen API is installed by
    // user_activation_gates.js.
    function aliasShimDocumentProps(){
      try { Object.defineProperty(d, 'fullscreenElement', { configurable: true, writable: true, value: null }); } catch(e){}
      try { Object.defineProperty(d, 'readyState', { configurable: true, get: function(){ return 'complete'; } }); } catch(e){}
      try { Object.defineProperty(d, 'visibilityState', { configurable: true, get: function(){ return 'visible'; } }); } catch(e){}
      try { Object.defineProperty(d, 'hidden', { configurable: true, get: function(){ return false; } }); } catch(e){}
      d.elementFromPoint = function(){ return null; };
      d.elementsFromPoint = function(){ return []; };
    }
    function aliasShimPrototypes(){
      try {
        if (g.HTMLElement) g.HTMLElement.prototype = baseProto;
        if (g.Element) g.Element.prototype = baseProto;
        if (g.Node) g.Node.prototype = baseProto;
        // Lazy `<style>.sheet` for EVERY <style> — server-rendered ones (from the HTML
        // parse / innerHTML) never pass through `createElement`, yet emotion /
        // styled-components hydrate by calling `styleEl.sheet.insertRule(...)` on them.
        // Without a `.sheet` that throws and aborts the chunk, blanking the styling.
        // CSSOM mutations stay separate from the DOM text, just as in browsers.
        // Defined on the shared element prototype and
        // WITHOUT a descriptor guard (the NON_MASKING interceptor makes every lookup
        // report a descriptor, so a guard would wrongly skip). Caches per element.
        // `.sheet` resolution for a <style> element (<link> takes the ternary in the caller).
        // Returns null when inline sheets are disallowed; otherwise rebuilds the CSSOM sheet
        // on text change and keeps it otherwise.
        currentSheetGetterStyle=function(elt, cached){
          if (!cssApply(inlineSheetAllowed, elt, [])) {
            if (cached) detachSheet(cached.sheet);
            elementSheets.set(elt, { sheet: null, text: null });
            return null;
          }
          var text = cssApply(sheetTextContent, elt, []) || '';
          if (!cached || !cached.sheet || cached.text !== text) {
            if (cached) detachSheet(cached.sheet);
            cached = { sheet: mkSheet(elt), text: text };
            elementSheets.set(elt, cached);
          }
          return cached.sheet;
        };
        currentSheetGetter=function(){
          var tag = cssApply(sheetTagName, this, []);
          if (!tag) return null;
          tag = cssTagUpper(tag);
          if (tag !== 'STYLE' && tag !== 'LINK') return null;
          var cached = elementSheets.get(this);
          // A child iframe document's <style> / <link> has a sheet (turbo-dom#299).
          if (!computedConnected(this)) { if (cached) detachSheet(cached.sheet); elementSheets.delete(this); return null; }
          if (tag === 'LINK') return cached ? cached.sheet : null;
          return currentSheetGetterStyle(this, cached);
        };
        Object.defineProperty(baseProto, 'sheet', {configurable:true,get:currentSheetGetter});
        var docProto = Object.getPrototypeOf(d) || baseProto;
        // Documents without a browsing context list their own sheets too (turbo-dom#163).
        var otherDocumentSheets=new WeakMap(),otherSheetsGet=WeakMap.prototype.get,otherSheetsSet=WeakMap.prototype.set,sheetsQuery=d.querySelectorAll;
        __ns.documentSheetsDescriptor={configurable:true,enumerable:true,get:function(){
          if(this===d)return documentSheets;
          var doc=this,list=cssApply(otherSheetsGet,otherDocumentSheets,[doc]);
          if(!list){list=cssList('StyleSheetList',function(){var out=[],found=cssApply(sheetsQuery,doc,['style,link']);for(var i=0;i<found.length;i++){var sheet=cssApply(currentSheetGetter,found[i],[]);if(sheet)out.push(sheet);}return out;});cssApply(otherSheetsSet,otherDocumentSheets,[doc,list]);}
          return list;
        }};
        if (g.Document) g.Document.prototype = docProto;
        if (g.HTMLDocument) g.HTMLDocument.prototype = docProto;
        aliasShimDocumentProps();
      } catch(e){}
    }
    aliasShimPrototypes();
  })();
  // ---- Range + Selection (real model over rtdom) ----------------------------------------------
  // The no-op stubs were enough for libraries that merely construct a Range, but contenteditable
  // editors (Lexical) and userEvent's contenteditable typing path REQUIRE a working selection:
  // userEvent.click sets the caret (createRange → setStart/End → selection.addRange), then on each
  // keypress userEvent reads selection.getRangeAt(0) to decide whether to emit `beforeinput`. With
  // a dead selection getInputRange() returns nothing and no input events fire, so onChange never
  // runs. This implements the slice of Range/Selection those paths exercise.
  function __zeroRect(){ return { x:0,y:0,top:0,left:0,right:0,bottom:0,width:0,height:0,toJSON:function(){return this;} }; }
  function __nodeIndex(n){ var p=n.parentNode; if(!p) return 0; var i=0,c=p.firstChild; while(c&&c!==n){ i++; c=c.nextSibling; } return i; }
  function __ancestry(n){ var a=[]; while(n){ a.push(n); n=n.parentNode; } return a; } // node … root
  // Resolve two boundary points once a common ancestor at (A[i] === B[j]) is found.
  // Returns -1 (before) / 1 (after) / 0 (same child + offset).
  function __cmpAt(A, B, i, j, oa, ob){
    var childA = i > 0 ? A[i - 1] : null, childB = j > 0 ? B[j - 1] : null;
    if (childA === null) return oa <= __nodeIndex(childB) ? -1 : 1;     // ca IS the common ancestor
    if (childB === null) return __nodeIndex(childA) < ob ? -1 : 1;      // cb IS the common ancestor
    var ia = __nodeIndex(childA), ib = __nodeIndex(childB);
    return ia < ib ? -1 : ia > ib ? 1 : 0;
  }
  // Compare boundary points (ca,oa) vs (cb,ob) in document order: -1 before, 0 equal, 1 after.
  function __cmp(ca,oa,cb,ob){
    if(ca===cb) return oa<ob?-1:oa>ob?1:0;
    var A=__ancestry(ca), B=__ancestry(cb);
    // walk B looking for a node whose ancestry includes it → common ancestor
    for(var i=0;i<A.length;i++){ var j=B.indexOf(A[i]); if(j>=0) return __cmpAt(A, B, i, j, oa, ob); }
    return 0; // disconnected
  }
  function __commonAncestor(a,b){ var A=__ancestry(a); var n=b; while(n){ if(A.indexOf(n)>=0) return n; n=n.parentNode; } return d; }
  var rangeRecord = g.__nativeRangeRecord;
  delete g.__nativeRangeRecord;
  var rangeCreate=Object.create,rangeDefine=Object.defineProperties,rangeProto=Object.getPrototypeOf;
  function Range(){
    var state=rangeCreate(Range.prototype);
    rangeDefine(state,{startContainer:{value:d,writable:true},startOffset:{value:0,writable:true},endContainer:{value:d,writable:true},endOffset:{value:0,writable:true},document:{value:d,writable:true}});
    rangeRecord(state,state);return rangeRecord(undefined,state,rangeProto(this));
  }
  Object.defineProperties(Range.prototype, {
    collapsed:{ configurable:true, enumerable:true, get:function(){ return this.startContainer===this.endContainer && this.startOffset===this.endOffset; } },
    commonAncestorContainer:{ configurable:true, enumerable:true, get:function(){ return __commonAncestor(this.startContainer, this.endContainer); } },
  });
  Range.prototype.setStart=function(n,o){ this.startContainer=n; this.startOffset=o|0; if(__cmp(this.startContainer,this.startOffset,this.endContainer,this.endOffset)>0){ this.endContainer=n; this.endOffset=o|0; } };
  Range.prototype.setEnd=function(n,o){ this.endContainer=n; this.endOffset=o|0; if(__cmp(this.startContainer,this.startOffset,this.endContainer,this.endOffset)>0){ this.startContainer=n; this.startOffset=o|0; } };
  Range.prototype.setStartBefore=function(n){ this.setStart(n.parentNode, __nodeIndex(n)); };
  Range.prototype.setStartAfter=function(n){ this.setStart(n.parentNode, __nodeIndex(n)+1); };
  Range.prototype.setEndBefore=function(n){ this.setEnd(n.parentNode, __nodeIndex(n)); };
  Range.prototype.setEndAfter=function(n){ this.setEnd(n.parentNode, __nodeIndex(n)+1); };
  Range.prototype.collapse=function(toStart){ if(toStart){ this.endContainer=this.startContainer; this.endOffset=this.startOffset; } else { this.startContainer=this.endContainer; this.startOffset=this.endOffset; } };
  Range.prototype.selectNode=function(n){ this.setStartBefore(n); this.setEndAfter(n); };
  Range.prototype.selectNodeContents=function(n){ this.startContainer=n; this.startOffset=0; this.endContainer=n; this.endOffset=(n.nodeType===3?(n.data||'').length:(n.childNodes?n.childNodes.length:0)); };
  Range.prototype.cloneRange=function(){ var r=new Range(); var state=rangeRecord(r); state.startContainer=this.startContainer; state.startOffset=this.startOffset; state.endContainer=this.endContainer; state.endOffset=this.endOffset; state.document=this.document; return r; };
  Range.prototype.comparePoint=function(n,o){ if(__cmp(n,o,this.startContainer,this.startOffset)<0) return -1; if(__cmp(n,o,this.endContainer,this.endOffset)>0) return 1; return 0; };
  Range.prototype.isPointInRange=function(n,o){ return this.comparePoint(n,o)===0; };
  Range.prototype.compareBoundaryPoints=function(how,other){ return __cmp(this.startContainer,this.startOffset,other.startContainer,other.startOffset); };
  Range.prototype.insertNode=function(node){ var c=this.startContainer, o=this.startOffset; if(c.nodeType===3){ var txt=c.data||''; var after=c.splitText?c.splitText(o):null; c.parentNode.insertBefore(node, after||c.nextSibling); } else { var ref=c.childNodes?c.childNodes[o]:null; c.insertBefore(node, ref||null); } };
  // End offset of a child node (text length for CharacterData, child count for Element).
  function rangeChildEndOffset(k){ return k.nodeType === 3 ? (k.data || '').length : (k.childNodes ? k.childNodes.length : 0); }
  // Delete a text slice inside one Text node that owns both boundaries.
  function rangeDeleteTextSlice(range, sc){
    var t = sc.data || '';
    if (sc.deleteData) sc.deleteData(range.startOffset, range.endOffset - range.startOffset);
    else sc.data = t.slice(0, range.startOffset) + t.slice(range.endOffset);
    range.endOffset = range.startOffset; range.endContainer = sc;
  }
  Range.prototype.deleteContents=function(){
    if(this.collapsed) return;
    var sc=this.startContainer;
    if(sc===this.endContainer && sc.nodeType===3){ rangeDeleteTextSlice(this, sc); return; }
    var ca=this.commonAncestorContainer;
    var kids=ca.childNodes?Array.prototype.slice.call(ca.childNodes):[];
    for(var i=0;i<kids.length;i++){
      var k=kids[i];
      if(this.comparePoint(k,0)===0 && this.comparePoint(k, rangeChildEndOffset(k))===0 && k.parentNode){
        k.parentNode.removeChild(k);
      }
    }
    this.collapse(true);
  };
  Range.prototype.cloneContents=function(){ var f=this.document.createDocumentFragment(); return f; };
  Range.prototype.extractContents=function(){ var f=this.cloneContents(); this.deleteContents(); return f; };
  Range.prototype.surroundContents=function(node){ this.insertNode(node); };
  Range.prototype.getClientRects=function(){ return []; };
  Range.prototype.getBoundingClientRect=function(){ return __zeroRect(); };
  Range.prototype.createContextualFragment=function(html){ var f=this.document.createDocumentFragment(); var t=this.document.createElement('div'); t.innerHTML=html; while(t.firstChild) f.appendChild(t.firstChild); return f; };
  Range.prototype.detach=function(){};
  Range.prototype.toString=function(){ var sc=this.startContainer; if(sc===this.endContainer && sc.nodeType===3){ return (sc.data||'').slice(this.startOffset, this.endOffset); } return ''; };
  if (__ns.makeNativeFunction) {
    var rangeImplementation=Range;
    Range=__ns.makeNativeFunction(rangeImplementation,'Range',0,true);
    Range.prototype=rangeImplementation.prototype;
    Object.defineProperty(Range.prototype,'constructor',{value:Range,writable:true,configurable:true});
    Object.defineProperty(Range,'prototype',{writable:false});
  }
  ['startContainer','startOffset','endContainer','endOffset'].forEach(function(name){
    var getter=function(){return rangeRecord(this)[name];};
    if(__ns.makeNativeFunction)getter=__ns.makeNativeFunction(getter,'get '+name,0);
    Object.defineProperty(Range.prototype,name,{get:getter,enumerable:true,configurable:true});
  });
  Object.getOwnPropertyNames(Range.prototype).forEach(function(name){
    if(name==='constructor')return;
    var descriptor=Object.getOwnPropertyDescriptor(Range.prototype,name);
    ['value','get'].forEach(function(kind){
      var implementation=descriptor[kind];if(typeof implementation!=='function')return;
      var delegate=function(){return implementation.apply(rangeRecord(this),arguments);};
      descriptor[kind]=__ns.makeNativeFunction?__ns.makeNativeFunction(delegate,kind==='get'?'get '+name:name,implementation.length):delegate;
    });
    descriptor.enumerable=true;descriptor.configurable=true;
    if('value'in descriptor)descriptor.writable=true;
    Object.defineProperty(Range.prototype,name,descriptor);
  });
  Object.defineProperty(Range.prototype,Symbol.toStringTag,{value:'Range',configurable:true});
  d.createRange=function(){return new Range();};
  function installDocumentFallbacks(){
    if(g.Range===undefined) g.Range=Range;
    if (!d.getRootNode) d.getRootNode = function(){ return d; };
    // document.write / writeln: parse the written markup and append it to <body>. Real browsers splice
    // at the parser position; our scripts run post-parse, so appending is the faithful no-JS-engine
    // behavior (e.g. a script that writes one element per item).
    if (!d.write || !d.writeln) {
      var docWrite = function(){ var html = Array.prototype.join.call(arguments, ''); var tmp = d.createElement('div'); tmp.innerHTML = html; var target = d.body || d.documentElement; if (!target) return; var kids = Array.prototype.slice.call(tmp.childNodes); for (var i=0;i<kids.length;i++) target.appendChild(kids[i]); };
      if (!d.write) d.write = docWrite;
      if (!d.writeln) d.writeln = function(){ docWrite(Array.prototype.join.call(arguments, '') + '\n'); };
    }
    if (g.__addGetElems) { g.__addGetElems(d); try { delete g.__addGetElems; } catch(e){} }
  }
  installDocumentFallbacks();
  // Selection lives in selection.js (installed natively after the form-control modules).

  // ── <iframe> second realm (bridged, same-isolate) ───────────────────────────────
  // A real cross-origin iframe hosts its OWN window/document realm and talks to the
  // parent only over postMessage (reCAPTCHA's anchor↔bframe handshake works exactly
  // this way). A separate V8 context is not available inside this single-isolate
  // binding, so `iframe.contentWindow` is a BRIDGED child window: a distinct window
  // object with its own event registry, a lightweight child document (native elements,
  // detached from the parent tree), its own location/origin, and DIRECTIONAL postMessage:
  //   parent → child:  iframe.contentWindow.postMessage(d,o)  fires the CHILD's message
  //                     listeners with { source: <parent view>, origin: <parent origin> }.
  //   child  → parent:  window.parent.postMessage(d,o)  (or e.source.postMessage inside
  //                     the child) fires the PARENT's message listeners with
  //                     { source: iframe.contentWindow, origin: <child origin> }.
  // The event's `source` is what lets each side reply to the other (the protocol reads
  // e.source + e.origin and posts back through e.source), so the two directional views
  // form a coherent two-window message channel.
  //
  // Fidelity tradeoff: globals/prototypes are SHARED — no true origin isolation, no
  // separate realm identity, and the child document is a lightweight facade over native
  // element construction rather than a second live-rendered tree. A determined script can
  // still detect the shared object graph. It is faithful enough for the message-channel
  // semantics a cross-frame handshake relies on, which is what this unblocks.
  (function(){
    // A `message` event shaped like the fields cross-frame protocols read.
    var mkMsgEvent = function(data, origin, source, ports){
      var event=new g.MessageEvent('message',{data:data,origin:origin||'',source:source||null,ports:ports||[]});
      eventRecord(event).trusted=true;
      return event;
    };
    // Deliver a message event to a window-like target's `message` listeners + onmessage.
    // Async via setTimeout when present (matches the real message-loop ordering + the
    // render tier's virtual timer queue); synchronous fallback for a bare isolate.
    var deliver = function(win, ev){
      var st = g.setTimeout || function(f){ return f(); };
      st(function(){
        var state=eventRecord(ev);state.target=win;state.currentTarget=win;state.trusted=true;state.eventPhase=2;
        try { if (typeof win.onmessage === 'function') win.onmessage(ev); } catch(e){}
        try { if (typeof win.dispatchEvent === 'function') win.dispatchEvent(ev); } catch(e){}
        finally{endEventDispatch(state);}
      }, 0);
    };
    // Minimal child document: real (native) elements forming a subtree DETACHED from the
    // parent's live tree, so the child realm builds + queries its own DOM without
    // scribbling into the host page. Element construction delegates to the native binding.
    var makeChildDocument = function(){
      // The child's documentElement / head / body are real (detached) <html> / <head> /
      // <body> elements (turbo-dom#306): Chrome's element types, html.css values (body's 8px
      // margin) and html / body / :root rules of the child's own sheets (frameSelector).
      var root = d.createElement('html');
      var head = d.createElement('head');
      var body = d.createElement('body');
      try { root.appendChild(head); root.appendChild(body); } catch(e){}
      var dl = {};
      // Walk the child's OWN detached subtree (native element-scoped querySelector isn't
      // reliable on a subtree that never entered the live tree, so recurse childNodes).
      // Handles the id / tag / .class selectors the child code uses; anything fancier
      // falls back to the native element querySelector best-effort.
      var attr = function(n, name){ try { return n.getAttribute ? n.getAttribute(name) : null; } catch(e){ return null; } };
      var walk = function(node, pred){
        var kids = node && node.childNodes ? node.childNodes : null;
        if (!kids) return null;
        for (var i = 0; i < kids.length; i++){
          var k = kids[i];
          if (k && k.nodeType === 1){
            if (pred(k)) return k;
            var deep = walk(k, pred); if (deep) return deep;
          }
        }
        return null;
      };
      var walkAll = function(node, pred, out){
        var kids = node && node.childNodes ? node.childNodes : null;
        if (!kids) return out;
        for (var i = 0; i < kids.length; i++){
          var k = kids[i];
          if (k && k.nodeType === 1){ if (pred(k)) out.push(k); walkAll(k, pred, out); }
        }
        return out;
      };
      var predFor = function(sel){
        sel = String(sel || '').trim();
        if (sel.charAt(0) === '#'){ var id = sel.slice(1); return function(n){ return attr(n,'id') === id || n.id === id; }; }
        if (sel.charAt(0) === '.'){ var cls = sel.slice(1); return function(n){ var c = attr(n,'class') || n.className || ''; return (' '+c+' ').indexOf(' '+cls+' ') >= 0; }; }
        var tag = sel.toUpperCase(); return function(n){ return String(n.tagName).toUpperCase() === tag; };
      };
      // Queries see the document element itself, then its descendants.
      var selfOr = function(pred, out){ if (pred(root)) out.push(root); return walkAll(root, pred, out); };
      var q1 = function(s){
        var pred = predFor(s), r = pred(root) ? root : walk(root, pred);
        if (r == null){ try { r = root.querySelector(s); } catch(e){} }
        return r;
      };
      var cd = {
        nodeType: 9, __childDoc: true,
        documentElement: root, head: head, body: body,
        readyState: 'complete', contentType: 'text/html', characterSet: 'UTF-8',
        cookie: '', title: '', hidden: false, visibilityState: 'visible',
        createElement: function(t){ return d.createElement(t); },
        createElementNS: function(ns,t){ return d.createElementNS ? d.createElementNS(ns,t) : d.createElement(t); },
        createTextNode: function(t){ return d.createTextNode(t); },
        createComment: function(t){ return d.createComment ? d.createComment(t) : d.createTextNode(String(t)); },
        createDocumentFragment: function(){ return d.createDocumentFragment(); },
        getElementById: function(id){ return selfOr(function(n){ return attr(n,'id') === String(id) || n.id === String(id); }, [])[0] || null; },
        querySelector: q1,
        querySelectorAll: function(s){ return selfOr(predFor(s), []); },
        getElementsByTagName: function(s){ return selfOr(predFor(String(s)), []); },
        addEventListener: function(t,f){ if (typeof f==='function') (dl[t]=dl[t]||[]).push(f); },
        removeEventListener: function(t,f){ var a=dl[t]; if(a){ var i=a.indexOf(f); if(i>=0) a.splice(i,1); } },
        dispatchEvent: function(ev){ if(!ev) return true; var a=dl[ev.type]; if(a) a.slice().forEach(function(f){ try{f.call(cd,ev);}catch(e){} }); return !(ev&&ev.defaultPrevented); },
      };
      // The document element's parent is the document.
      try { Object.defineProperty(root, 'parentNode', { get: function(){ return cd; }, configurable: true }); } catch(e){}
      return cd;
    };
    // origin (scheme://host[:port]) of a URL string; 'null' when not absolute http(s).
    var originOf = function(u){ try { var m=/^(https?:\/\/[^\/]+)/i.exec(String(u||'')); return m?m[1]:'null'; } catch(e){ return 'null'; } };

    // An iframe's content box on `axis` ('width' / 'height'): its computed size (the cascade
    // maps the size attributes as presentational hints), less padding and border under
    // border-box sizing, rounded to whole pixels like Chrome (turbo-dom#288).
    var framePx = function(style, name){ var match = /^(-?\d+(?:\.\d+)?)px$/.exec(String(style.getPropertyValue(name) || '').trim()); return match ? +match[1] : null; };
    // Under border-box sizing the content box excludes padding and border.
    var frameBoxExtras = function(style, axis){
      if (String(style.getPropertyValue('box-sizing')).trim() !== 'border-box') return 0;
      var sides = axis === 'width' ? ['left', 'right'] : ['top', 'bottom'];
      return sides.reduce(function(sum, side){ return sum + (framePx(style, 'padding-' + side) || 0) + (framePx(style, 'border-' + side + '-width') || 0); }, 0);
    };
    var frameContentSize = function(hostEl, axis){
      var style = null;
      try { style = hostEl && g.getComputedStyle(hostEl); } catch(e){}
      var size = style ? framePx(style, axis) : null;
      // A detached iframe has no box: its window reports the 300x150 default.
      return size === null ? (axis === 'width' ? 300 : 150) : Math.max(0, Math.round(size - frameBoxExtras(style, axis)));
    };
    // A child window's viewport is its iframe's content box; its outer size is the top window's.
    var defineFrameViewport = function(childWin, hostEl, parentWin){
      [['innerWidth', function(){ return frameContentSize(hostEl, 'width'); }], ['innerHeight', function(){ return frameContentSize(hostEl, 'height'); }],
       ['outerWidth', function(){ return parentWin.outerWidth; }], ['outerHeight', function(){ return parentWin.outerHeight; }]].forEach(function(pair){
        try { Object.defineProperty(childWin, pair[0], { get: pair[1], set: function(value){ Object.defineProperty(childWin, pair[0], { value: value, writable: true, configurable: true, enumerable: true }); }, configurable: true, enumerable: true }); } catch(e){}
      });
    };

    // A child window's matchMedia evaluates against its own viewport (turbo-dom#288); the
    // binding's own matchMedia only (a replaced one is kept). getComputedStyle evaluates @media
    // against the read element's document's viewport (computeStyle, turbo-dom#299).
    var defineFrameMedia = function(childWin){
      if (typeof __ns.matchMedia !== 'function' || !__ns.withMediaView || mediaMode === 'jsdom') return;
      childWin.matchMedia = __ns.makeNativeFunction(function matchMedia(query){ return __ns.matchMedia(query, childWin); }, 'matchMedia', 1);
    };
    // The child document cascades its own sheets (turbo-dom#299); a srcdoc is its markup.
    var defineFrameDocument = function(childDoc, childWin, hostEl){
      __ns.registerFrameRoot(childDoc.documentElement, childWin);
      var srcdoc = hostEl && hostEl.getAttribute ? hostEl.getAttribute('srcdoc') : null;
      if (srcdoc != null) loadFrameMarkup(childDoc, String(srcdoc));
    };
    // A srcdoc is a whole document (turbo-dom#306): its head / body content and the html /
    // head / body attributes move into the child document's elements.
    // Document.parseHTMLUnsafe (installed after this bootstrap; read on the first srcdoc).
    var frameParse = null;
    var loadFrameMarkup = function(childDoc, markup){
      frameParse = frameParse || g.Document.parseHTMLUnsafe;
      var parsed = null;
      try { parsed = frameParse.call(g.Document, markup); } catch(e){}
      if (!parsed || !parsed.documentElement) { childDoc.body.innerHTML = markup; return; }
      [[parsed.documentElement, childDoc.documentElement], [parsed.head, childDoc.head], [parsed.body, childDoc.body]].forEach(function(pair){
        if (!pair[0]) return;
        Array.prototype.forEach.call(pair[0].attributes, function(a){ try { pair[1].setAttribute(a.name, a.value); } catch(e){} });
        if (pair[0] === parsed.documentElement) return;
        for (var child = pair[0].firstChild; child; child = pair[0].firstChild) { pair[1].appendChild(d.importNode(child, true)); pair[0].removeChild(child); }
      });
    };
    // An iframe resizes with the DOM (its attributes, styles, style sheets): once a child
    // window has a MediaQueryList, every DOM change re-evaluates the lists after it.
    var frameWatcher = null;
    __ns.watchFrameViewport = function(){
      if (frameWatcher || typeof g.MutationObserver !== 'function' || !__ns.mediaViewportChanged) return;
      frameWatcher = new g.MutationObserver(function(){ __ns.mediaViewportChanged(); });
      try { frameWatcher.observe(d, { subtree: true, childList: true, attributes: true, characterData: true }); } catch(e){ frameWatcher = null; }
    };

    // Build (once) a bridged child realm for the <iframe> `hostEl`. `parentWin` defaults
    // to the top window `g`. Returns the child window; also sets hostEl.contentWindow /
    // contentDocument. Reused by turbo-surf's render tier for the reCAPTCHA bframe.
    function frameSrcOf(hostEl){ return hostEl && (hostEl.src || (hostEl.getAttribute && hostEl.getAttribute('src'))); }
    function frameParentOrigin(parentWin){ return (parentWin.location && parentWin.location.origin) || originOf(parentWin.location && parentWin.location.href) || ''; }
    function frameProtocolOf(childOrigin){ return (childOrigin.split(':')[0] || 'about') + ':'; }
    function frameMsgPorts(transfer){ return Array.isArray(transfer)?transfer:(transfer&&transfer.length?Array.prototype.slice.call(transfer):[]); }
    function frameAssignProps(childWin, childDoc, hostEl, childOrigin, wl){
      var srcAttr = frameSrcOf(hostEl);
      Object.assign(childWin, {
        document: childDoc, closed: false, name: (hostEl && hostEl.name) || '',
        origin: childOrigin, length: 0, frames: [], devicePixelRatio: 1,
        innerWidth: 0, innerHeight: 0, outerWidth: 0, outerHeight: 0, screenX: 0, screenY: 0,
        location: { href: String(srcAttr || 'about:blank'), origin: childOrigin,
          protocol: frameProtocolOf(childOrigin),
          host: childOrigin.replace(/^https?:\/\//,''),
          hostname: childOrigin.replace(/^https?:\/\//,'').split(':')[0],
          port: '', pathname: '/', search: '', hash: '',
          assign:function(){}, replace:function(){}, reload:function(){}, toString:function(){ return this.href; } },
        navigator: g.navigator, screen: g.screen, history: g.history,
        onmessage: null, onerror: null, onload: null,
        setTimeout: g.setTimeout, clearTimeout: g.clearTimeout,
        setInterval: g.setInterval, clearInterval: g.clearInterval,
        requestAnimationFrame: g.requestAnimationFrame, cancelAnimationFrame: g.cancelAnimationFrame,
        getComputedStyle: g.getComputedStyle, matchMedia: g.matchMedia, MediaQueryList: g.MediaQueryList, MediaQueryListEvent: g.MediaQueryListEvent,
        atob: g.atob, btoa: g.btoa, crypto: g.crypto, performance: g.performance,
        addEventListener: function(t,f){ if(typeof f==='function') (wl[t]=wl[t]||[]).push(f); },
        removeEventListener: function(t,f){ var a=wl[t]; if(a){ var i=a.indexOf(f); if(i>=0) a.splice(i,1); } },
        dispatchEvent: function(ev){var state=beginEventDispatch(ev,childWin,[childWin]),a=wl[state.type];try{if(a)a.slice().forEach(function(f){try{f.call(childWin,ev);}catch(e){}});}finally{endEventDispatch(state);}return !state.defaultPrevented;},
        focus:function(){}, blur:function(){}, close:function(){ childWin.closed = true; },
      });
    }
    __ns.makeFrameRealm = function(hostEl, parentWin, createWindow){
      parentWin = parentWin || g;
      if (hostEl && hostEl.__realm) return hostEl.__realm;
      var childDoc = makeChildDocument();
      var childOrigin = originOf(frameSrcOf(hostEl));
      var parentOrigin = frameParentOrigin(parentWin);
      var wl = {};
      var childWin = typeof createWindow === 'function' ? createWindow() : {};
      var parentView;
      frameAssignProps(childWin, childDoc, hostEl, childOrigin, wl);
      childWin.postMessage = function(msg, targetOrigin, transfer){
        deliver(childWin, mkMsgEvent(msg, parentOrigin, parentView, frameMsgPorts(transfer)));
      };
      defineFrameViewport(childWin, hostEl, parentWin);
      defineFrameMedia(childWin);
      defineFrameDocument(childDoc, childWin, hostEl);
      childWin.window = childWin; childWin.self = childWin; childWin.globalThis = childWin;
      childDoc.defaultView = childWin;
      parentView = {
        postMessage: function(msg, targetOrigin, transfer){
          deliver(parentWin, mkMsgEvent(msg, childOrigin, childWin, frameMsgPorts(transfer)));
        },
        location: { origin: parentOrigin, href: (parentWin.location && parentWin.location.href) || '' },
      };
      childWin.parent = parentView; childWin.top = parentView; childWin.frameElement = hostEl || null;
      wireFrameHost(hostEl, childWin, childDoc);
      return childWin;
    };
    // Attach the realm to its host <iframe> element. Native elements may refuse the
    // contentWindow / contentDocument accessors (eager read-only), so the write is in try.
    function wireFrameHost(hostEl, childWin, childDoc){
      if (!hostEl) return;
      hostEl.__realm = childWin;
      try { hostEl.contentWindow = childWin; hostEl.contentDocument = childDoc; } catch(e){}
    }

    // Wire createElement('iframe') to expose a real contentWindow/contentDocument, built
    // lazily on first access (or when src is set). Kept general so any consumer gets it;
    // the lazy build means an unused iframe costs nothing.
    var origCreateEl = d.createElement;
    d.createElement = ({ createElement(tag){
      var el = $apply(origCreateEl,this,arguments);
      if (el && $call($lower, String($call(createdLocalName, el))) === 'iframe' && !el.__iframeWired) {
        el.__iframeWired = true;
        try {
          $def(el, 'contentWindow', { __proto__: null, configurable: true,
            get: function(){ return el.__realm || __ns.makeFrameRealm(el, g); } });
          $def(el, 'contentDocument', { __proto__: null, configurable: true,
            get: function(){ return (el.__realm || __ns.makeFrameRealm(el, g)).document; } });
        } catch(e){
          // Native element rejected the accessor → fall back to an eager realm.
          try { __ns.makeFrameRealm(el, g); } catch(e2){}
        }
      }
      return el;
    } }).createElement;
  })();
  // Native ObjectTemplate accessors expose value descriptors. Project through
  // real native getter/setter functions so values stay backed by the same tree.
  (function(){
    if (typeof __ns.domAccessor !== 'function') return;
    // Reinstallation keeps read-only constructor prototypes from the previous
    // document; compatibility descriptors belong to this bootstrap's raw nodes.
    var oldBase=compatibilityElementPrototype||g.HTMLElement.prototype,oldDocument=g.Document.prototype;
    var nodeConstructors=['Node','Element','HTMLElement','Document','HTMLDocument','CharacterData','Attr','DocumentType','NamedNodeMap','Text','Comment','CDATASection','ProcessingInstruction','DocumentFragment','ShadowRoot','SVGElement','SVGSVGElement','MathMLElement'];
    Object.getOwnPropertyNames(g).forEach(function(name){if(/^HTML.*Element$/.test(name)&&typeof g[name]==='function'&&nodeConstructors.indexOf(name)<0)nodeConstructors.push(name);});
    nodeConstructors.forEach(function(name){var previous=g[name];var ctor=__ns.makeNativeFunction(function(){throw new TypeError('Illegal constructor');},name,0,true);ctor.prototype=previous.prototype;Object.defineProperty(g,name,{value:ctor,writable:true,configurable:true,enumerable:false});});
    var prototypes={};
    function defineInterface(name,parent,existing){
      var ctor=g[name],proto=existing||Object.create(parent);
      if(existing)Object.setPrototypeOf(proto,parent);
      Object.defineProperty(proto,'constructor',{value:ctor,writable:true,configurable:true});
      Object.defineProperty(proto,Symbol.toStringTag,{value:name,configurable:true});
      ctor.prototype=proto;
      if(parent&&parent.constructor&&parent.constructor!==Object)Object.setPrototypeOf(ctor,parent.constructor);
      Object.defineProperty(ctor,'prototype',{writable:false});
      Object.defineProperty(g,name,{value:ctor,writable:true,configurable:true,enumerable:false});
      try{delete ctor[Symbol.hasInstance];}catch(e){}
      prototypes[name]=proto;
      return proto;
    }
    var eventTargetProto=g.EventTarget.prototype;
    function ensureNsSet(key){ if(!__ns[key])__ns[key]=new WeakSet(); return __ns[key]; }
    function ensureNsObj(key){ if(!__ns[key])__ns[key]={}; return __ns[key]; }
    var eventTargets=ensureNsSet('eventTargetInstances');
    var eventCallbacks=ensureNsObj('eventTargetCallbacks');
    var windowCallbacks=ensureNsObj('windowEventCallbacks');
    ['addEventListener','removeEventListener','dispatchEvent'].forEach(function(name){
      if(!eventCallbacks[name])eventCallbacks[name]=eventTargetProto[name];
      if(!windowCallbacks[name])windowCallbacks[name]=g[name];
    });
    var PreviousEventTarget=g.EventTarget;
    var eventTargetCtor=__ns.makeNativeFunction(class extends PreviousEventTarget{constructor(){super();$c.add(eventTargets,this);}},'EventTarget',0,true,true);
    eventTargetCtor.prototype=eventTargetProto;
    __ns.eventTargetConstructor=eventTargetCtor;
    Object.defineProperty(g,'EventTarget',{value:eventTargetCtor,writable:true,configurable:true,enumerable:false});
    Object.defineProperty(eventTargetProto,'constructor',{value:eventTargetCtor,writable:true,configurable:true});
    Object.defineProperty(eventTargetProto,Symbol.toStringTag,{value:'EventTarget',configurable:true});
    Object.defineProperty(g.EventTarget,'prototype',{writable:false});
    Object.defineProperty(g,'EventTarget',{value:g.EventTarget,writable:true,configurable:true,enumerable:false});
    var nodeProto=defineInterface('Node',g.EventTarget.prototype);
    var elementProto=defineInterface('Element',nodeProto);
    var htmlProto=defineInterface('HTMLElement',elementProto);
    var documentProto=defineInterface('Document',nodeProto);
    defineInterface('HTMLDocument',documentProto);
    var characterProto=defineInterface('CharacterData',nodeProto);
    defineInterface('Attr',nodeProto);
    defineInterface('DocumentType',nodeProto);
    defineInterface('Text',characterProto);
    defineInterface('Comment',characterProto);
    defineInterface('CDATASection',characterProto);
    defineInterface('ProcessingInstruction',characterProto);
    defineInterface('DocumentFragment',nodeProto);
    defineInterface('ShadowRoot',g.DocumentFragment.prototype);
    defineInterface('SVGElement',elementProto);
    defineInterface('SVGSVGElement',g.SVGElement.prototype);
    defineInterface('MathMLElement',elementProto);
    Object.getOwnPropertyNames(g).forEach(function(name){
      if(!/^HTML.*Element$/.test(name)||name==='HTMLElement'||typeof g[name]!=='function')return;
      var proto=g[name].prototype;
      defineInterface(name,htmlProto,proto===oldBase?null:proto);
    });
    ['HTMLAudioElement','HTMLVideoElement'].forEach(function(name){Object.setPrototypeOf(prototypes[name],prototypes.HTMLMediaElement);Object.setPrototypeOf(g[name],g.HTMLMediaElement);});
    // The native DOM has no HTML-media playback decoder. Query that empty
    // capability set without claiming codec support from unrelated offline DSP.
    delete prototypes.HTMLAnchorElement.click;
    var mediaTypes=new Set();
    var mediaNode=__ns.domAccessor('nodeType',false,'HTMLElement');
    Object.defineProperty(prototypes.HTMLMediaElement,'canPlayType',{value:__ns.makeNativeFunction(function(type){
      mediaNode.call(this);
      if($call(computedStyleNativeNamespace,this)!=='http://www.w3.org/1999/xhtml'||!$c.includes(['audio','video'],$call(createdLocalName,this)))throw new TypeError('Illegal invocation');
      if(arguments.length<1)throw new TypeError("Failed to execute 'canPlayType' on 'HTMLMediaElement': 1 argument required, but only 0 present.");
      if(typeof type==='symbol')throw new TypeError('Cannot convert a Symbol value to a string');
      var mime=$c.toLowerCase($c.trim($c.split(String(type),';',1)[0]));
      return $c.has(mediaTypes,mime)?'maybe':'';
    },'canPlayType',1),writable:true,enumerable:true,configurable:true});
    installAudioConstructor();
    function installAudioConstructor(){
      if (typeof g.Audio !== 'function') return;
      var audioConstructor = __ns.makeNativeFunction(function(src){ var element = d.createElement('audio'); if (src !== undefined) element.src = src; return element; }, 'Audio', 0, true, false, false);
      audioConstructor.prototype = prototypes.HTMLAudioElement;
      Object.defineProperty(audioConstructor, 'prototype', { writable: false });
      Object.defineProperty(g, 'Audio', { value: audioConstructor, writable: true, configurable: true });
    }

    // Keep existing compatibility behavior while separating the base interfaces;
    // the native members below replace these descriptors on their proper owner.
    Reflect.ownKeys(oldBase).forEach(function(key){
      if(key==='constructor'||key===Symbol.toStringTag)return;
      var descriptor=Object.getOwnPropertyDescriptor(oldBase,key);
      if(descriptor)Object.defineProperty(htmlProto,key,descriptor);
    });
    // Read-only constructor prototypes can leave the previous bootstrap's sheet getter in oldBase.
    if(currentSheetGetter)Object.defineProperty(htmlProto,'sheet',{configurable:true,get:currentSheetGetter});
    Reflect.ownKeys(oldDocument).forEach(function(key){
      if(key!=='exitFullscreen'&&key!=='cookie')return;
      var descriptor=Object.getOwnPropertyDescriptor(oldDocument,key);
      if(descriptor)Object.defineProperty(documentProto,key,descriptor);
    });
    var accessorOwners={
      Node:['nodeType','nodeName','isConnected','ownerDocument','parentNode','parentElement','childNodes','firstChild','nextSibling','nodeValue','textContent'],
      Element:['namespaceURI','prefix','localName','tagName','id','className','attributes','innerHTML','outerHTML','children','firstElementChild','shadowRoot'],
      HTMLElement:['style','dataset','contentEditable'],
      // ElementCSSInlineStyle: Chrome puts `style` on each of the three (`dataset`, from
      // HTMLOrSVGElement, is document_basics.js's).
      SVGElement:['style'],
      MathMLElement:['style'],
      CharacterData:['data'],
      Document:['activeElement','children','firstElementChild'],
      DocumentFragment:['children','firstElementChild'],
      ShadowRoot:['host','mode','innerHTML']
    };
    var writable={nodeValue:1,textContent:1,id:1,className:1,innerHTML:1,outerHTML:1,data:1};
    var nativeKeys=['constructor'];
    Object.keys(accessorOwners).forEach(function(owner){
      accessorOwners[owner].forEach(function(name){
        nativeKeys.push(name);
        var descriptor={enumerable:true,configurable:true,get:__ns.domAccessor(name,false,owner)};
        if(writable[name])descriptor.set=__ns.domAccessor(name,true,owner);
        Object.defineProperty(prototypes[owner],name,descriptor);
      });
    });
    [['name','nodeName'],['value','__attrValue'],['ownerElement','__attrOwner'],['specified','__attrSpecified'],['namespaceURI','namespaceURI'],['prefix','prefix'],['localName','localName']].forEach(function(pair){
      var name=pair[0],raw=pair[1],get=__ns.domAccessor(raw,false,'Attr');
      var descriptor={configurable:true,enumerable:true,get:raw===name?get:__ns.makeNativeFunction(function(){return get.call(this);},'get '+name,0)};
      if(name==='value'){var set=__ns.domAccessor(raw,true,'Attr');descriptor.set=__ns.makeNativeFunction(function(value){set.call(this,value);},'set value',1);}
      Object.defineProperty(prototypes.Attr,name,descriptor);
    });
    [htmlProto,prototypes.SVGElement,prototypes.MathMLElement].forEach(function(proto){var inlineStyle=Object.getOwnPropertyDescriptor(proto,'style');inlineStyle.set=__ns.makeNativeFunction(function(value){var style=inlineStyle.get.call(this);Object.getOwnPropertyDescriptor(declarationPrototype,'cssText').set.call(style,value);},'set style',1);Object.defineProperty(proto,'style',inlineStyle);});
    ['Element','Document','DocumentFragment'].forEach(function(owner){
      var children=Object.getOwnPropertyDescriptor(prototypes[owner],'children').get;
      ['childElementCount','lastElementChild'].forEach(function(name){
        var getter=__ns.makeNativeFunction(function(){var list=children.call(this);return name==='childElementCount'?list.length:list[list.length-1]||null;},'get '+name,0);
        Object.defineProperty(prototypes[owner],name,{get:getter,enumerable:true,configurable:true});
      });
    });
    var getType=Object.getOwnPropertyDescriptor(nodeProto,'nodeType').get;
    var getTag=Object.getOwnPropertyDescriptor(elementProto,'tagName').get;
    var getNamespace=Object.getOwnPropertyDescriptor(elementProto,'namespaceURI').get;
    var getLocalTag=Object.getOwnPropertyDescriptor(elementProto,'localName').get;
    var nativeAttributes=Object.getOwnPropertyDescriptor(elementProto,'attributes').get;$N.nativeAttributes=nativeAttributes;
    var mapRecords=new WeakMap(),mapsByElement=new WeakMap(),namedFactory=__ns.makeNamedObject;
    var namedMapCtor=defineInterface('NamedNodeMap',Object.prototype),namedMapProto=namedMapCtor;
    function mapOwner(map){var record=collectionGet?collectionGet(map):null,owner=record&&record.kind==='NamedNodeMap'?record.owner:collectionGet?null:mapRecords.get(map);if(!owner)throw new TypeError('Illegal invocation');return owner;}
    function mapEntries(map){return nativeAttributes.call(mapOwner(map));}
    function mapIndex(value){return Number(value)>>>0;}
    var mapMethods={
      item(index){return mapEntries(this)[mapIndex(index)]||null;},
      getNamedItem(name){name=attributeString(name);return $call($N.getAttributeNode,mapOwner(this),name);},
      getNamedItemNS(namespace,local){return $call($N.getAttributeNodeNS,mapOwner(this),namespace,local);},
      setNamedItem(attribute){return $call($N.setAttributeNode,mapOwner(this),attribute);},
      setNamedItemNS(attribute){return $call($N.setAttributeNodeNS,mapOwner(this),attribute);},
      removeNamedItem(name){var owner=mapOwner(this),attribute=$call($N.getAttributeNode,owner,attributeString(name));if(!attribute)throw new __ns.DOMException('The attribute is absent','NotFoundError');return $call($N.removeAttributeNode,owner,attribute);},
      removeNamedItemNS(namespace,local){var owner=mapOwner(this),attribute=$call($N.getAttributeNodeNS,owner,namespace,local);if(!attribute)throw new __ns.DOMException('The attribute is absent','NotFoundError');return $call($N.removeAttributeNode,owner,attribute);}
    };
    Object.keys(mapMethods).forEach(function(name){var delegate=mapMethods[name];Object.defineProperty(namedMapProto,name,{value:__ns.makeNativeFunction(function(){mapOwner(this);if(arguments.length<delegate.length)throw new TypeError('Not enough arguments');return delegate.apply(this,arguments);},name,delegate.length),writable:true,enumerable:true,configurable:true});});
    Object.defineProperty(namedMapProto,'length',{get:__ns.makeNativeFunction(function(){return mapEntries(this).length;},'get length',0),enumerable:true,configurable:true});
    Object.defineProperty(namedMapProto,Symbol.iterator,{value:__ns.makeNativeFunction(function(){var map=this;mapOwner(map);var index=0;return {next(){var entries=mapEntries(map);return index<entries.length?{value:entries[index++],done:false}:{value:undefined,done:true};},[Symbol.iterator](){return this;}};},'values',0),writable:true,configurable:true});
    // Operation dispatcher for the NamedNodeMap proxy. A `switch` lets the many kinds share
    // one helper; without it the proxy callback blew the cc <= 12 ceiling.
    function namedAttrsOpIndices(attribute, entries){ return Array.from(entries, function(_, i){ return i; }); }
    function namedAttrsOpKeys(attribute, entries){ return Array.from(new Set(Array.from(entries, function(entry){ return entry.name; }))); }
    function namedAttrsOpDescriptor(attribute, entries, index){ return attribute ? { value: attribute, writable: false, enumerable: index !== null, configurable: true } : undefined; }
    var namedAttrsOpTable = { __proto__: null,
      'indices': namedAttrsOpIndices,
      'keys': namedAttrsOpKeys,
      'get': function(attribute){ return attribute; },
      'has': function(attribute){ return !!attribute; },
      'descriptor': namedAttrsOpDescriptor,
      'set': function(attribute){ return attribute ? true : undefined; },
      'define': function(attribute){ return attribute ? true : undefined; },
      'delete': function(attribute){ return attribute ? false : undefined; }
    };
    function namedAttrsOp(operation, attribute, entries, index){
      var handler = namedAttrsOpTable[operation];
      return handler ? handler(attribute, entries, index) : undefined;
    }
    Object.defineProperty(elementProto,'attributes',{get:__ns.makeNativeFunction(function(){getTag.call(this);var owner=this,map=$call($wmGet,mapsByElement,owner);if(map)return map;map=namedFactory(namedMapProto,function(operation,key,value){var entries=nativeAttributes.call(owner),index=/^(0|[1-9]\d*)$/.test(key)?Number(key):null,attribute=index===null?Array.from(entries).find(function(entry){return entry.name===key;}):entries[index];return namedAttrsOp(operation,attribute,entries,index);},true);$c.set(mapRecords,map,owner);if(collectionSet)collectionSet(map,{kind:'NamedNodeMap',owner:owner});$c.set(mapsByElement,owner,map);return map;},'get attributes',0),enumerable:true,configurable:true});
    var isShadowRoot=__ns.domAccessor('__isShadowRoot',false,'Node');
    var probe=d.createElement('div');
    __ns.cascadeProp=Object.getOwnPropertyDescriptor(probe,'__cascadeProp').value;
    nativeKeys.push('__attrValue','__attrOwner','__attrSpecified','__cascadeProp','__isShadowRoot','host','mode','containsNode','activeElement');
    var constants={ELEMENT_NODE:1,ATTRIBUTE_NODE:2,TEXT_NODE:3,CDATA_SECTION_NODE:4,ENTITY_REFERENCE_NODE:5,ENTITY_NODE:6,PROCESSING_INSTRUCTION_NODE:7,COMMENT_NODE:8,DOCUMENT_NODE:9,DOCUMENT_TYPE_NODE:10,DOCUMENT_FRAGMENT_NODE:11,NOTATION_NODE:12,DOCUMENT_POSITION_DISCONNECTED:1,DOCUMENT_POSITION_PRECEDING:2,DOCUMENT_POSITION_FOLLOWING:4,DOCUMENT_POSITION_CONTAINS:8,DOCUMENT_POSITION_CONTAINED_BY:16,DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC:32};
    Object.keys(constants).forEach(function(name){
      nativeKeys.push(name);
      [g.Node,nodeProto].forEach(function(owner){Object.defineProperty(owner,name,{value:constants[name],enumerable:true,configurable:true});});
    });
    ['addEventListener','removeEventListener','dispatchEvent'].forEach(function(name){
      var native=Object.getOwnPropertyDescriptor(probe,name).value;
      var minimum=name==='dispatchEvent'?1:2;
      var method=__ns.makeNativeFunction(function(){
        if(arguments.length<minimum)throw new TypeError(name+' requires '+minimum+' arguments');
        if(this===g)return $apply(windowCallbacks[name],this,arguments);
        var node=false;try{$call(getType,this);node=true;}catch(e){}
        if(node)return $apply(native,this,arguments);
        if(__ns.eventTargetNativeBrand?!__ns.eventTargetNativeBrand(this):!$c.has(eventTargets,this))throw new TypeError('Illegal invocation');
        return eventCallbacks[name].apply(this,arguments);
      },name,minimum);
      Object.defineProperty(eventTargetProto,name,{value:method,writable:true,configurable:true,enumerable:true});
      nativeKeys.push(name);
    });
    var controls=['HTMLInputElement','HTMLTextAreaElement','HTMLSelectElement','HTMLButtonElement','HTMLFieldSetElement','HTMLObjectElement','HTMLOutputElement'];
    ['setCustomValidity','setSelectionRange','setRangeText','select'].forEach(function(name){
      var descriptor=Object.getOwnPropertyDescriptor(probe,name);
      nativeKeys.push(name);
      var owners=/^(setSelectionRange|setRangeText|select)$/.test(name)?controls.slice(0,2):controls;
      owners.forEach(function(owner){var proto=prototypes[owner];if(proto&&!Object.hasOwn(proto,name)&&descriptor)Object.defineProperty(proto,name,{value:descriptor.value,writable:true,configurable:true,enumerable:true});});
    });
    var methodOwners={
      Node:['appendChild','removeChild','insertBefore','replaceChild','contains','hasChildNodes','cloneNode','getRootNode'],
      Element:['setAttribute','getAttribute','getAttributeNode','getAttributeNodeNS','setAttributeNode','setAttributeNodeNS','removeAttributeNode','hasAttribute','removeAttribute','querySelector','querySelectorAll','matches','closest','append','prepend','remove','before','after','replaceWith','replaceChildren','insertAdjacentElement','insertAdjacentHTML','toggleAttribute','getAttributeNS','hasAttributeNS','setAttributeNS','removeAttributeNS','scrollIntoView','scrollTo','scrollBy','getBoundingClientRect','attachShadow'],
      HTMLElement:['focus','blur','click'],
      Text:['splitText']
    };
    Object.keys(methodOwners).forEach(function(owner){
      methodOwners[owner].forEach(function(name){
        var descriptor=Object.getOwnPropertyDescriptor(probe,name);
        if(descriptor&&typeof descriptor.value==='function'){
          Object.defineProperty(prototypes[owner],name,{value:descriptor.value,writable:true,configurable:true,enumerable:true});
          nativeKeys.push(name);
        }
      });
    });
    // The probe served only the reads above: closures of this scope outlive the install, and a
    // startup snapshot must hold no shell node wrapper (round-1 review #15).
    probe=null;
    // HTMLOrSVGElement (turbo-dom#87): focus() / blur() brand-check their interface. An SVG or
    // MathML element takes script focus when connected, rendered and given a valid tabindex (or,
    // in SVG, when it is an <a> with an href / xlink:href).
    var nativeFocus=htmlProto.focus,nativeBlur=htmlProto.blur,nativeClick=htmlProto.click;
    // Internal focusing steps (dialogs, focusgroup navigation) focus any element kind.
    __ns.focusElement=nativeFocus;__ns.blurElement=nativeBlur;
    var parsesTabIndex=function(element){var value=element.getAttribute('tabindex');return value!==null&&/^[\t\n\f\r ]*[+-]?[0-9]/.test(value);};
    // Rendering, as Chrome 156 measures it: an SVG element other than <svg> needs a non-HTML
    // parent, a non-SVG element under SVG needs a <foreignObject> parent, and no inclusive
    // ancestor may compute display: none.
    // SVG element (other than <svg> itself) whose parent is HTML (or has no namespace):
    // its rendering stops here per Chrome's foreign-rendered test.
    function foreignRenderedSvgNoHost(node, parentSpace){
      return node.namespaceURI === 'http://www.w3.org/2000/svg' && node.localName !== 'svg' && (parentSpace === null || parentSpace === 'http://www.w3.org/1999/xhtml');
    }
    // Non-SVG under SVG: only renders inside <foreignObject>.
    function foreignRenderedBadForeign(node, parentSpace, parent){
      return node.namespaceURI !== 'http://www.w3.org/2000/svg' && parentSpace === 'http://www.w3.org/2000/svg' && parent.localName !== 'foreignObject';
    }
    var foreignRendered=function(element){
      for (var node = element; node && node.nodeType === 1; node = node.parentNode) {
        var parent = node.parentNode, parentSpace = parent && parent.nodeType === 1 ? parent.namespaceURI : null;
        if (foreignRenderedSvgNoHost(node, parentSpace)) return false;
        if (foreignRenderedBadForeign(node, parentSpace, parent)) return false;
        if (g.getComputedStyle(node).display === 'none') return false;
      }
      return true;
    };
    var foreignFocusable=function(element,svg){
      if(!$call(computedStyleNativeConnected,element))return false;
      if(!parsesTabIndex(element)&&!(svg&&element.localName==='a'&&(element.hasAttribute('href')||element.hasAttributeNS('http://www.w3.org/1999/xlink','href'))))return false;
      return foreignRendered(element);
    };
    [['HTMLElement',htmlProto],['SVGElement',prototypes.SVGElement],['MathMLElement',prototypes.MathMLElement]].forEach(function(entry){
      var check=__ns.domAccessor('nodeType',false,entry[0]),foreign=entry[0]!=='HTMLElement',svg=entry[0]==='SVGElement';
      var operations={focus:foreign?function(){$call(check,this);if(foreignFocusable(this,svg))return $apply(nativeFocus,this,arguments);}:function(){$call(check,this);return $apply(nativeFocus,this,arguments);},blur:function(){$call(check,this);return $apply(nativeBlur,this,arguments);}};
      if(!foreign)operations.click=function(){$call(check,this);return $apply(nativeClick,this,arguments);};
      Object.keys(operations).forEach(function(name){Object.defineProperty(entry[1],name,{value:__ns.makeNativeFunction(operations[name],name,0),writable:true,configurable:true,enumerable:true});});
    });
    computedStyleNativeMatches=Object.getOwnPropertyDescriptor(prototypes.Element,'matches').value;
    computedStyleNativeQueryAll=Object.getOwnPropertyDescriptor(prototypes.Element,'querySelectorAll').value;
    computedStyleNativeDocumentQueryAll=computedOwnValue(d,elementProto,'querySelectorAll');
    computedStyleNativeGetAttribute=Object.getOwnPropertyDescriptor(prototypes.Element,'getAttribute').value;
    computedStyleNativeRoot=Object.getOwnPropertyDescriptor(prototypes.Node,'getRootNode').value;
    ['querySelector','querySelectorAll','append','prepend','replaceChildren'].forEach(function(name){
      var descriptor=Object.getOwnPropertyDescriptor(d,name)||Object.getOwnPropertyDescriptor(elementProto,name);
      if(descriptor)Object.defineProperty(documentProto,name,descriptor);
      if(name!=='getElementById')Object.defineProperty(prototypes.DocumentFragment,name,Object.getOwnPropertyDescriptor(elementProto,name));
    });
    // ChildNode members: Element's and CharacterData's own copies of the shared native operation
    // brand-check their interface (Chrome 156: TypeError, Illegal invocation).
    var childNodeApply=Reflect.apply;
    // DocumentType's copies (document_basics.js) wrap the same shared operations.
    __ns.childNodeOperations=Object.create(null);
    ['before','after','remove','replaceWith'].forEach(function(name){
      var native=Object.getOwnPropertyDescriptor(elementProto,name).value;
      __ns.childNodeOperations[name]=native;
      [['Element',elementProto],['CharacterData',characterProto]].forEach(function(owner){
        var check=__ns.domAccessor('nodeType',false,owner[0]);
        Object.defineProperty(owner[1],name,{value:__ns.makeNativeFunction(function(){childNodeApply(check,this,[]);return childNodeApply(native,this,arguments);},name,0),writable:true,enumerable:true,configurable:true});
      });
    });
    // CSSOM View scrolling operations return a promise of a ScrollResult (Chrome 156); a bad
    // receiver or argument rejects it instead of throwing. Headless scrolling never moves, so
    // the operation only converts its arguments (IDL order) and resolves.
    var scrollPromise=g.Promise,scrollResolve=g.Promise.resolve,scrollReject=g.Promise.reject,ScrollTypeError=TypeError,scrollString=String;
    var scrollEnums={behavior:['ScrollOptions','ScrollBehavior',['auto','instant','smooth']],block:['ScrollIntoViewOptions','ScrollLogicalPosition',['start','center','end','nearest']],container:['ScrollIntoViewOptions','ScrollContainer',['all','nearest']]};
    scrollEnums.inline=scrollEnums.block;
    var scrollMembers=function(operation,options,keys,dictionary){
      if(options==null)return;
      if(typeof options!=='object'&&typeof options!=='function')throw new ScrollTypeError("Failed to execute '"+operation+"' on 'Element': The provided value is not of type '"+dictionary+"'.");
      $c.forEach(keys,function(key){
        var value=options[key];if(value===undefined)return;
        var spec=scrollEnums[key],owner=spec?spec[0]:dictionary,read="Failed to execute '"+operation+"' on 'Element': Failed to read the '"+key+"' property from '"+owner+"': ";
        if(typeof value==='symbol')throw new ScrollTypeError(read+'Cannot convert a Symbol value to a '+(spec?'string':'number'));
        if(!spec){+value;return;}
        value=scrollString(value);
        if($c.indexOf(spec[2],value)<0)throw new ScrollTypeError(read+"The provided value '"+value+"' is not a valid enum value of type "+spec[1]+'.');
      });
    };
    var scrollArguments={
      scrollIntoView:function(args){var arg=args[0];if(arg!==null&&(typeof arg==='object'||typeof arg==='function'))scrollMembers('scrollIntoView',arg,['behavior','block','container','inline'],'ScrollIntoViewOptions');},
      scrollTo:function(args,operation){if(args.length>=2){+args[0];+args[1];return;}scrollMembers(operation,args[0],['behavior','left','top'],'ScrollToOptions');}
    };
    ['scrollIntoView','scrollTo','scrollBy'].forEach(function(name){
      var native=Object.getOwnPropertyDescriptor(elementProto,name).value,check=__ns.domAccessor('nodeType',false,'Element'),convert=scrollArguments[name==='scrollBy'?'scrollTo':name];
      Object.defineProperty(elementProto,name,{value:__ns.makeNativeFunction(function(){
        try{childNodeApply(check,this,[]);childNodeApply(native,this,[]);childNodeApply(convert,null,[arguments,name]);}catch(error){return childNodeApply(scrollReject,scrollPromise,[error]);}
        return childNodeApply(scrollResolve,scrollPromise,[{interrupted:false}]);
      },name,0),writable:true,enumerable:true,configurable:true});
    });
    function projectCompatibility(owner,names,source){
      var proto=prototypes[owner],check=__ns.domAccessor('nodeType',false,owner);
      names.forEach(function(name){
        var descriptor=Object.getOwnPropertyDescriptor(source,name);
        if(!descriptor)return;
        ['get','set','value'].forEach(function(key){
          var delegate=descriptor[key];if(typeof delegate!=='function')return;
          descriptor[key]=__ns.makeNativeFunction(function(){$call(check,this);return $apply(delegate,this,arguments);},key==='get'?'get '+name:key==='set'?'set '+name:name,delegate.length);
        });
        descriptor.enumerable=true;Object.defineProperty(proto,name,descriptor);
      });
    }
    projectCompatibility('Document',['getElementsByTagName','getElementsByClassName','getElementsByName'],htmlProto);
    Object.defineProperty(documentProto,'adoptedStyleSheets',adoptionDescriptor);if(__ns.documentSheetsDescriptor){Object.defineProperty(documentProto,'styleSheets',__ns.documentSheetsDescriptor);delete __ns.documentSheetsDescriptor;}Object.defineProperty(g.ShadowRoot.prototype,'adoptedStyleSheets',shadowAdoptionDescriptor);delete d.adoptedStyleSheets;
    projectCompatibility('Node',['isEqualNode','isSameNode','lastChild','previousSibling'],htmlProto);
    projectCompatibility('Element',['classList','getClientRects','getElementsByTagName','getElementsByClassName','nextElementSibling','previousElementSibling','requestFullscreen'],htmlProto);
    ['isEqualNode','isSameNode','lastChild','previousSibling','replaceChild','classList','getClientRects','getElementsByTagName','getElementsByClassName','getElementsByName','nextElementSibling','previousElementSibling','requestFullscreen','lastElementChild','cookie','exitFullscreen'].forEach(function(name){delete htmlProto[name];});
    ['createTextNode','createComment','createDocumentFragment','createElementNS','getElementById'].forEach(function(name){
      var descriptor=Object.getOwnPropertyDescriptor(d,name);
      if(descriptor&&typeof descriptor.value==='function'){
        Object.defineProperty(documentProto,name,{value:descriptor.value,writable:true,configurable:true,enumerable:true});
        nativeKeys.push(name);
      }
    });
    ['accept','autoFocus','autocomplete','checked','disabled','form','max','min','multiple','name','pattern','placeholder','readOnly','required','step','value','getBoundingClientRect'].forEach(function(name){delete prototypes.HTMLCanvasElement[name];});
    // Captured for the reflected members below: page-replaced builtins never run on these paths (miaskiewicz/turbo-dom#199).
    $getAttributeNative=elementProto.getAttribute;$setAttributeNative=elementProto.setAttribute;$removeAttributeNative=elementProto.removeAttribute;$hasAttributeNative=elementProto.hasAttribute;
    // Members another engine script defines after bootstrap (document_basics.js's
    // Document#documentElement) are captured again by captured_intrinsics.js, which also
    // fails installation if any entry is still not a function.
    function captureProtoMember(proto,name,getter){for(var o=proto;o;o=Object.getPrototypeOf(o)){var d=$gopd(o,name);if(d)return getter?d.get:d.value;}}
    function captureIntrinsicKey(proto,name){return proto===g.Attr.prototype?'attr.'+name:proto===elementProto?'element.'+name:name;}
    function captureNodeGetters(getters,missingOnly){for(var i=0;i<getters.length;i++)for(var k=0;k<getters[i][1].length;k++){var name=getters[i][1][k],key=captureIntrinsicKey(getters[i][0],name);if(!missingOnly||typeof $N[key]!=='function')$N[key]=captureProtoMember(getters[i][0],name,true);}}
    function captureNodeMethods(methods,missingOnly){for(var i=0;i<methods.length;i++)for(var k=0;k<methods[i][1].length;k++)if(!missingOnly||typeof $N[methods[i][1][k]]!=='function')$N[methods[i][1][k]]=captureProtoMember(methods[i][0],methods[i][1][k],false);}
    function captureNodeIntrinsics(missingOnly){
      var getters=[[g.Node.prototype,['nodeType','nodeName','nodeValue','parentNode','parentElement','firstChild','nextSibling','childNodes','ownerDocument']],[g.Document.prototype,['documentElement']],[g.Attr.prototype,['ownerElement','prefix','namespaceURI','value','localName','name']],[elementProto,['namespaceURI','prefix','attributes']]];
      captureNodeGetters(getters,missingOnly);
      var methods=[[g.Document.prototype,['createTextNode']],[g.Node.prototype,['getRootNode','contains','insertBefore','appendChild']],[elementProto,['getAttribute','getAttributeNode','getAttributeNodeNS','setAttributeNode','setAttributeNodeNS','removeAttributeNode','getBoundingClientRect']]];
      captureNodeMethods(methods,missingOnly);
    }
    captureNodeIntrinsics(false);
    __ns.completeNodeIntrinsics=function(){captureNodeIntrinsics(true);var missing=[];for(var key in $N)if(typeof $N[key]!=='function')missing[missing.length]=key;if(missing.length)throw new Error('uncaptured node intrinsics: '+missing.join(', '));return Object.keys($N).sort().join();};
    var rNodeType=$gopd(g.Node.prototype,'nodeType').get,rParentElement=$gopd(g.Node.prototype,'parentElement').get,rIsPrototypeOf=Object.prototype.isPrototypeOf;
    var rFormGetters=[];['HTMLInputElement','HTMLTextAreaElement','HTMLSelectElement','HTMLButtonElement','HTMLFieldSetElement','HTMLObjectElement','HTMLOutputElement','HTMLLabelElement','HTMLLegendElement','HTMLOptionElement','HTMLImageElement'].forEach(function(owner){var proto=prototypes[owner],d=proto&&$gopd(proto,'form');if(d&&d.get)rFormGetters[rFormGetters.length]=[proto,d.get];});
    function rForm(el){for(var i=0;i<rFormGetters.length;i++)if($apply(rIsPrototypeOf,rFormGetters[i][0],[el]))return $apply(rFormGetters[i][1],el,[]);return null;}
    function reflectAttribute(owner,name,attribute,nullable){
      var check=__ns.domAccessor('nodeType',false,owner);
      Object.defineProperty(prototypes[owner],name,{configurable:true,enumerable:true,
        get:__ns.makeNativeFunction(function(){$apply(check,this,[]);var value=$getAttr(this,attribute);return nullable?value:value||'';},'get '+name,0),
        set:__ns.makeNativeFunction(function(value){$apply(check,this,[]);if(nullable&&value==null){$removeAttr(this,attribute);return;}if(typeof value==='symbol')throw new TypeError('Cannot convert a Symbol to a string');$setAttr(this,attribute,String(value));},'set '+name,1)});
    }
    var metaApply=Reflect.apply,metaString=String,metaTypeError=TypeError;
    var metaGetAttribute=elementProto.getAttribute,metaSetAttribute=elementProto.setAttribute;
    function checkMeta(receiver){
      if(metaApply(getLocalTag,receiver,[])!=='meta'||metaApply(getNamespace,receiver,[])!=='http://www.w3.org/1999/xhtml')throw new metaTypeError('Illegal invocation');
    }
    [['name','name'],['httpEquiv','http-equiv'],['content','content'],['media','media'],['scheme','scheme']].forEach(function(pair){
      var name=pair[0],attribute=pair[1];
      Object.defineProperty(prototypes.HTMLMetaElement,name,{configurable:true,enumerable:true,
        get:__ns.makeNativeFunction(function(){checkMeta(this);return metaApply(metaGetAttribute,this,[attribute])||'';},'get '+name,0),
        set:__ns.makeNativeFunction(function(value){checkMeta(this);if(typeof value==='symbol')throw new metaTypeError('Cannot convert a Symbol to a string');metaApply(metaSetAttribute,this,[attribute,metaString(value)]);},'set '+name,1)});
    });
    var iframeRemoveAttribute=elementProto.removeAttribute;
    function checkIframe(receiver){
      if(metaApply(getLocalTag,receiver,[])!=='iframe'||metaApply(getNamespace,receiver,[])!=='http://www.w3.org/1999/xhtml')throw new metaTypeError('Illegal invocation');
    }
    Object.defineProperty(prototypes.HTMLIFrameElement,'credentialless',{configurable:true,enumerable:true,
      get:__ns.makeNativeFunction(function(){checkIframe(this);return metaApply(metaGetAttribute,this,['credentialless'])!==null;},'get credentialless',0),
      set:__ns.makeNativeFunction(function(value){checkIframe(this);if(value)metaApply(metaSetAttribute,this,['credentialless','']);else metaApply(iframeRemoveAttribute,this,['credentialless']);},'set credentialless',1)});
    ['title','lang','accessKey'].forEach(function(name){reflectAttribute('HTMLElement',name,name.toLowerCase(),false);});
    var ariaNames='Atomic AutoComplete BrailleLabel BrailleRoleDescription Busy Checked ColCount ColIndex ColIndexText ColSpan Current Description Disabled Expanded HasPopup Hidden Invalid KeyShortcuts Label Level Live Modal MultiLine MultiSelectable Orientation Placeholder PosInSet Pressed ReadOnly Relevant Required RoleDescription RowCount RowIndex RowIndexText RowSpan Selected SetSize Sort ValueMax ValueMin ValueNow ValueText'.split(' ');
    ariaNames.forEach(function(name){reflectAttribute('Element','aria'+name,'aria-'+name.toLowerCase(),true);});
    reflectAttribute('Element','role','role',true);
    ['slot','elementTiming'].forEach(function(name){reflectAttribute('Element',name,name.toLowerCase(),false);});
    function reflectedMember(name,get,set){var check=__ns.domAccessor('nodeType',false,'HTMLElement');Object.defineProperty(htmlProto,name,{configurable:true,enumerable:true,get:__ns.makeNativeFunction(function(){$apply(check,this,[]);return $apply(get,this,[]);},'get '+name,0),set:__ns.makeNativeFunction(function(value){$apply(check,this,[]);return $apply(set,this,[value]);},'set '+name,1)});}
    function attributeString(value){if(typeof value==='symbol')throw new TypeError('Cannot convert a Symbol to a string');return String(value);}
    function enumAttribute(name,keywords,fallback){var lowerName=name.toLowerCase();reflectedMember(name,function(){var value=$c.toLowerCase($getAttr(this,lowerName)||'');return $c.indexOf(keywords,value)>=0?value:fallback;},function(value){$setAttr(this,lowerName,attributeString(value));});}
    enumAttribute('dir',['ltr','rtl','auto'],'');
    enumAttribute('inputMode',['none','text','tel','url','email','numeric','decimal','search'],'');
    enumAttribute('enterKeyHint',['enter','done','go','next','previous','search','send'],'');
    enumAttribute('virtualKeyboardPolicy',['auto','manual'],'');
    ['autofocus','inert'].forEach(function(name){reflectedMember(name,function(){return $hasAttr(this,name);},function(value){if(value)$setAttr(this,name,'');else $removeAttr(this,name);});});
    ['SVGElement','MathMLElement'].forEach(function(owner){var check=__ns.domAccessor('nodeType',false,owner);Object.defineProperty(prototypes[owner],'autofocus',{configurable:true,enumerable:true,get:__ns.makeNativeFunction(function(){$apply(check,this,[]);return $hasAttr(this,'autofocus');},'get autofocus',0),set:__ns.makeNativeFunction(function(value){$apply(check,this,[]);if(value)$setAttr(this,'autofocus','');else $removeAttr(this,'autofocus');},'set autofocus',1)});});
    reflectedMember('hidden',function(){var value=$getAttr(this,'hidden');return value===null?false:$c.toLowerCase(value)==='until-found'?'until-found':true;},function(value){if(typeof value!=='boolean'&&value!=null){value=attributeString(value);if($c.toLowerCase(value)==='until-found'){$setAttr(this,'hidden','until-found');return;}}if(value)$setAttr(this,'hidden','');else $removeAttr(this,'hidden');});
    function inheritedAttribute(el,name,accepted){for(var node=el;node&&$apply(rNodeType,node,[])===1;node=$apply(rParentElement,node,[])){var value=$getAttr(node,name);if(value!==null&&$c.indexOf(accepted,$c.toLowerCase(value))>=0)return $c.toLowerCase(value);}return null;}
    reflectedMember('translate',function(){return inheritedAttribute(this,'translate',['','yes','no'])!=='no';},function(value){$setAttr(this,'translate',value?'yes':'no');});
    reflectedMember('spellcheck',function(){var value=inheritedAttribute(this,'spellcheck',['','true','false']);if(value!==null)return value!=='false';return $c.toUpperCase(String($apply(getTag,this,[])))!=='INPUT'||$c.toLowerCase($getAttr(this,'type')||'')!=='password';},function(value){$setAttr(this,'spellcheck',value?'true':'false');});
    reflectedMember('writingSuggestions',function(){var value=$getAttr(this,'writingsuggestions');if(value===null)value=inheritedAttribute($apply(rParentElement,this,[]),'writingsuggestions',['true','false']);return value!==null&&$c.toLowerCase(value)==='false'?'false':'true';},function(value){$setAttr(this,'writingsuggestions',attributeString(value));});
    reflectedMember('draggable',function(){var value=$c.toLowerCase($getAttr(this,'draggable')||'');if(value==='true'||value==='false')return value==='true';var tag=$c.toUpperCase(String($apply(getTag,this,[])));return tag==='IMG'||tag==='A'&&$hasAttr(this,'href');},function(value){$setAttr(this,'draggable',value?'true':'false');});
    reflectedMember('autocapitalize',function(){var value=$getAttr(this,'autocapitalize');if(value===null&&rForm(this))value=$getAttr(rForm(this),'autocapitalize');value=$c.toLowerCase(value||'');return value===''?'':value==='off'?'none':value==='on'?'sentences':$c.indexOf(['none','sentences','words','characters'],value)>=0?value:'sentences';},function(value){$setAttr(this,'autocapitalize',attributeString(value));});
    reflectedMember('autocorrect',function(){var value=$getAttr(this,'autocorrect');if(value===null&&rForm(this))value=$getAttr(rForm(this),'autocorrect');return $c.toLowerCase(value||'')!=='off';},function(value){$setAttr(this,'autocorrect',value?'on':'off');});
    var reflectedApply=Reflect.apply,reflectedSetHas=Set.prototype.has,reflectedGetAttribute=g.Element.prototype.getAttribute;
    // Compatibility control delegates still require genuine interface receivers.
    var controlInterfaces={HTMLInputElement:'input',HTMLTextAreaElement:'textarea',HTMLSelectElement:'select',HTMLOptionElement:'option',HTMLButtonElement:'button',HTMLLabelElement:'label',HTMLCanvasElement:'canvas'};
    Object.keys(controlInterfaces).forEach(function(owner){
      var proto=prototypes[owner],check=__ns.domAccessor('nodeType',false,'HTML:'+controlInterfaces[owner]);
      Object.getOwnPropertyNames(proto).forEach(function(name){
        if(name==='constructor')return;
        var descriptor=Object.getOwnPropertyDescriptor(proto,name);
        ['get','set','value'].forEach(function(kind){
          var delegate=descriptor[kind];if(typeof delegate!=='function')return;
          descriptor[kind]=__ns.makeNativeFunction(function(){reflectedApply(check,this,[]);return reflectedApply(delegate,this,arguments);},kind==='get'?'get '+name:kind==='set'?'set '+name:name,delegate.length);
        });
        descriptor.enumerable=true;Object.defineProperty(proto,name,descriptor);
      });
    });
    function reflectedResolveBaseURL(value,element){
      if(value&&typeof g.URL==='function'){try{return new g.URL(value,element.ownerDocument.URL||g.location.href).href;}catch(e){}}
      return value;
    }
    function reflectedResolveURL(value,element,name,owner){
      if(owner==='HTMLBaseElement')return reflectedResolveBaseURL(value,element);
      var raw=reflectedApply(reflectedGetAttribute,element,[name]);
      return raw===null?'':__ns.resolveDocumentURL(element,raw);
    }
    var reflectedOwners={src:['HTMLScriptElement','HTMLImageElement','HTMLIFrameElement','HTMLMediaElement','HTMLInputElement','HTMLEmbedElement','HTMLSourceElement','HTMLTrackElement'],href:['HTMLAnchorElement','HTMLAreaElement','HTMLBaseElement','HTMLLinkElement'],rel:['HTMLAnchorElement','HTMLAreaElement','HTMLLinkElement'],as:['HTMLLinkElement'],media:['HTMLLinkElement','HTMLStyleElement','HTMLSourceElement'],type:['HTMLInputElement','HTMLButtonElement','HTMLScriptElement','HTMLLinkElement','HTMLStyleElement','HTMLEmbedElement','HTMLObjectElement','HTMLSourceElement'],valueAsNumber:['HTMLInputElement'],sheet:['HTMLStyleElement','HTMLLinkElement']};
    Object.keys(reflectedOwners).forEach(function(name){
      var descriptor=Object.getOwnPropertyDescriptor(htmlProto,name);if(!descriptor)return;
      reflectedOwners[name].forEach(function(owner){
        var proto=prototypes[owner];if(!proto||name!=='sheet'&&Object.hasOwn(proto,name))return;
        var copy={configurable:true,enumerable:true},check=__ns.domAccessor('nodeType',false,'HTMLElement');
        var ownerTags=Object.keys(htmlByTag).filter(function(tag){return htmlByTag[tag]===owner;});
        var extraTags={HTMLInputElement:['INPUT'],HTMLButtonElement:['BUTTON'],HTMLAnchorElement:['A'],HTMLImageElement:['IMG'],HTMLIFrameElement:['IFRAME'],HTMLScriptElement:['SCRIPT'],HTMLStyleElement:['STYLE'],HTMLMediaElement:['AUDIO','VIDEO']};
        if(extraTags[owner])ownerTags=ownerTags.concat(extraTags[owner]);
        var ownerLocals=new Set(ownerTags.map(function(tag){return tag.toLowerCase();}));
        ['get','set'].forEach(function(kind){var delegate=descriptor[kind];if(!delegate)return;copy[kind]=__ns.makeNativeFunction(function(){reflectedApply(check,this,[]);if(!reflectedApply(reflectedSetHas,ownerLocals,[reflectedApply(getLocalTag,this,[])]))throw new TypeError('Illegal invocation');var value=reflectedApply(delegate,this,arguments);if(kind==='get'&&(name==='src'||name==='href'))return reflectedResolveURL(value,this,name,owner);return value;},kind+' '+name,kind==='get'?0:1);});
        Object.defineProperty(proto,name,copy);
      });
      delete htmlProto[name];
    });
    // Reflected content attributes the copies above do not cover: [interface, IDL name,
    // boolean?, content attribute]. Only interfaces lacking the member get it.
    var reflectedSetAttribute=g.Element.prototype.setAttribute,reflectedRemoveAttribute=g.Element.prototype.removeAttribute,reflectedHasAttribute=g.Element.prototype.hasAttribute;
    var reflectedTags={HTMLBaseElement:['base'],HTMLMediaElement:['audio','video'],HTMLVideoElement:['video'],HTMLImageElement:['img'],HTMLEmbedElement:['embed'],HTMLObjectElement:['object'],HTMLIFrameElement:['iframe'],HTMLMapElement:['map'],HTMLFieldSetElement:['fieldset'],HTMLAnchorElement:['a'],HTMLDetailsElement:['details'],HTMLParamElement:['param']};
    var reflectedContent=[];
    // turbo-dom#118: HTMLBaseElement.target.
    reflectedContent.push(['HTMLBaseElement','target',false,'target']);
    // turbo-dom#165: HTMLMediaElement's boolean attributes (muted through defaultMuted).
    reflectedContent.push(['HTMLMediaElement','controls',true,'controls'],['HTMLMediaElement','autoplay',true,'autoplay'],['HTMLMediaElement','loop',true,'loop'],['HTMLMediaElement','defaultMuted',true,'muted'],['HTMLMediaElement','disableRemotePlayback',true,'disableremoteplayback'],['HTMLVideoElement','playsInline',true,'playsinline']);
    // turbo-dom#169: the name attribute of every owner the form-control copies do not cover.
    ['HTMLImageElement','HTMLEmbedElement','HTMLObjectElement','HTMLIFrameElement','HTMLMapElement','HTMLFieldSetElement','HTMLAnchorElement','HTMLDetailsElement'].forEach(function(owner){reflectedContent.push([owner,'name',false,'name']);});
    // turbo-dom#288: HTMLIFrameElement's width / height (DOMString reflections).
    reflectedContent.push(['HTMLIFrameElement','width',false,'width'],['HTMLIFrameElement','height',false,'height']);
    // turbo-dom#174: HTMLParamElement.
    reflectedContent.push(['HTMLParamElement','name',false,'name'],['HTMLParamElement','value',false,'value'],['HTMLParamElement','type',false,'type'],['HTMLParamElement','valueType',false,'valuetype']);
    reflectedContent.forEach(function(entry){
      var owner=entry[0],name=entry[1],flag=entry[2],attribute=entry[3],proto=prototypes[owner];if(!proto||Object.hasOwn(proto,name))return;
      var check=__ns.domAccessor('nodeType',false,'HTMLElement'),locals=new Set(reflectedTags[owner]);
      var brand=function(receiver){reflectedApply(check,receiver,[]);if(!reflectedApply(reflectedSetHas,locals,[reflectedApply(getLocalTag,receiver,[])]))throw new TypeError('Illegal invocation');};
      Object.defineProperty(proto,name,{configurable:true,enumerable:true,
        get:__ns.makeNativeFunction(function(){brand(this);if(flag)return reflectedApply(reflectedHasAttribute,this,[attribute]);var value=reflectedApply(reflectedGetAttribute,this,[attribute]);return value===null?'':value;},'get '+name,0),
        set:__ns.makeNativeFunction(function(value){brand(this);if(arguments.length<1)throw new TypeError("Failed to set the '"+name+"' property on '"+owner+"': 1 argument required, but only 0 present.");if(!flag){reflectedApply(reflectedSetAttribute,this,[attribute,attributeString(value)]);return;}if(value)reflectedApply(reflectedSetAttribute,this,[attribute,'']);else reflectedApply(reflectedRemoveAttribute,this,[attribute]);},'set '+name,1)});
    });
    installMediaMuted();
    installTextWholeText();
    // The muted IDL attribute is media state, not a reflection: it starts false.
    function installMediaMuted(){
      if (!prototypes.HTMLMediaElement || Object.hasOwn(prototypes.HTMLMediaElement, 'muted')) return;
      var mediaMuted=new WeakMap(),mutedGet=WeakMap.prototype.get,mutedSet=WeakMap.prototype.set,mediaLocals=new Set(['audio','video']),mediaCheck=__ns.domAccessor('nodeType',false,'HTMLElement');
      var mediaBrand=function(receiver){reflectedApply(mediaCheck,receiver,[]);if(!reflectedApply(reflectedSetHas,mediaLocals,[reflectedApply(getLocalTag,receiver,[])]))throw new TypeError('Illegal invocation');};
      Object.defineProperty(prototypes.HTMLMediaElement,'muted',{configurable:true,enumerable:true,
        get:__ns.makeNativeFunction(function(){mediaBrand(this);return reflectedApply(mutedGet,mediaMuted,[this])===true;},'get muted',0),
        set:__ns.makeNativeFunction(function(value){mediaBrand(this);reflectedApply(mutedSet,mediaMuted,[this,!!value]);},'set muted',1)});
    }
    // Text.wholeText (turbo-dom#143): the data of the contiguous Text siblings, in tree order.
    function installTextWholeText(){
      if (!g.Text || Object.hasOwn(g.Text.prototype, 'wholeText')) return;
      var wholeCheck=__ns.domAccessor('nodeType',false,'Text'),wholeType=Object.getOwnPropertyDescriptor(nodeProto,'nodeType').get,wholePrevious=Object.getOwnPropertyDescriptor(nodeProto,'previousSibling').get,wholeNext=Object.getOwnPropertyDescriptor(nodeProto,'nextSibling').get,wholeData=Object.getOwnPropertyDescriptor(g.CharacterData.prototype,'data').get;
      var isText=function(node){if(node===null)return false;var type=reflectedApply(wholeType,node,[]);return type===3||type===4;};
      Object.defineProperty(g.Text.prototype,'wholeText',{configurable:true,enumerable:true,get:__ns.makeNativeFunction(function(){
        reflectedApply(wholeCheck,this,[]);var first=this,previous;while(isText(previous=reflectedApply(wholePrevious,first,[])))first=previous;
        var text='';for(var node=first;isText(node);node=reflectedApply(wholeNext,node,[]))text+=reflectedApply(wholeData,node,[]);return text;
      },'get wholeText',0)});
    }
    var elementOperations={getAttributeNames(){var entries=$call(nativeAttributes,this),names=[];for(var index=0;index<entries.length;index++)$c.push(names,$call($N['attr.name'],entries[index]));return names;},hasAttributes(){return $call(nativeAttributes,this).length>0;},insertAdjacentText(position,text){if(typeof position==='symbol'||typeof text==='symbol')throw new TypeError('Cannot convert a Symbol to a string');position=$c.toLowerCase(String(position));var node=$call($N.createTextNode,$call($N.ownerDocument,this),String(text)),parent=$call($N.parentNode,this);if(position==='beforebegin'){if(parent)$call($N.insertBefore,parent,node,this);}else if(position==='afterend'){if(parent)$call($N.insertBefore,parent,node,$call($N.nextSibling,this));}else if(position==='afterbegin')$call($N.insertBefore,this,node,$call($N.firstChild,this));else if(position==='beforeend')$call($N.appendChild,this,node);else throw new __ns.DOMException('Invalid adjacent position','SyntaxError');}};
    Object.keys(elementOperations).forEach(function(name){var delegate=elementOperations[name],check=__ns.domAccessor('nodeType',false,'Element');Object.defineProperty(elementProto,name,{value:__ns.makeNativeFunction(function(){check.call(this);if(arguments.length<delegate.length)throw new TypeError(name+' requires '+delegate.length+' arguments');return delegate.apply(this,arguments);},name,delegate.length),writable:true,enumerable:true,configurable:true});});
    // GlobalEventHandlers: HTMLElement, SVGElement and MathMLElement each brand-check (turbo-dom#87).
    var eventHandlers=new WeakMap(),handlerAdd=g.EventTarget.prototype.addEventListener,handlerRemove=g.EventTarget.prototype.removeEventListener;
    var handlerNames='abort animationcancel animationend animationiteration animationstart auxclick beforeinput beforematch beforetoggle beforexrselect blur cancel canplay canplaythrough change click close command contentvisibilityautostatechange contextlost contextmenu contextrestored copy cuechange cut dblclick drag dragend dragenter dragleave dragover dragstart drop durationchange emptied ended error focus formdata gotpointercapture input invalid keydown keypress keyup load loadeddata loadedmetadata loadstart lostpointercapture mousedown mouseenter mouseleave mousemove mouseout mouseover mouseup mousewheel paste pause play playing pointercancel pointerdown pointerenter pointerleave pointermove pointerout pointerover pointerrawupdate pointerup progress ratechange reset resize scroll scrollend scrollsnapchange scrollsnapchanging securitypolicyviolation seeked seeking select selectionchange selectstart slotchange stalled submit suspend timeupdate toggle transitioncancel transitionend transitionrun transitionstart volumechange waiting webkitanimationend webkitanimationiteration webkitanimationstart webkittransitionend wheel'.split(' ');
    function handlerToggle(element,type,entry,value){
      if(value&&!entry.value)$call(__ns.handlerAdd||handlerAdd,element,type,entry.callback);
      if(!value&&entry.value)$call(__ns.handlerRemove||handlerRemove,element,type,entry.callback);
    }
    function handlerEnsureMap(element){
      var handlers=$call($wmGet,eventHandlers,element);
      if(!handlers){handlers=$create(null);$call($wmSet,eventHandlers,element,handlers);}
      return handlers;
    }
    [['HTMLElement',htmlProto],['SVGElement',prototypes.SVGElement],['MathMLElement',prototypes.MathMLElement]].forEach(function(owner){var checkHandlerReceiver=__ns.domAccessor('nodeType',false,owner[0]);
    handlerNames.forEach(function(type){
      var name='on'+type;
      Object.defineProperty(owner[1],name,{configurable:true,enumerable:true,
        get:__ns.makeNativeFunction(function(){$call(checkHandlerReceiver,this);if(prepareInlineHandler)$call(prepareInlineHandler,this,name);var handlers=$call($wmGet,eventHandlers,this);return handlers&&handlers[name]&&handlers[name].value||null;},'get '+name,0),
        set:__ns.makeNativeFunction(function(value){
          $call(checkHandlerReceiver,this);if(noteInlineHandler)$call(noteInlineHandler,this,name);value=typeof value==='function'?value:null;
          var handlers=handlerEnsureMap(this);
          var entry=handlers[name];
          if(!entry){entry={value:null,callback:__ns.makeNativeFunction(function(event){var current=$call($wmGet,eventHandlers,this)[name].value;if(current&&$call(current,this,event)===false)event.preventDefault();},name,1)};handlers[name]=entry;}
          handlerToggle(this,type,entry,value);
          entry.value=value;
        },'set '+name,1)});
    });
    });
    // The lookup start of locate-a-namespace / locate-a-namespace-prefix, through captured natives.
    function nodeLookupStart(node){var type=$call(getType,node);return type===9?$call($N.documentElement,node):type===2?$call($N['attr.ownerElement'],node):type===1?node:$call($N.parentElement,node);}
    // The `xmlns` declaration that defines `prefix` on `element` (null for the default
    // namespace), returned as its value; undefined when the element declares no such thing.
    function lookupNamespaceURIDecl(element, prefix){
      var declaration = $call($N.getAttributeNodeNS, element, 'http://www.w3.org/2000/xmlns/', prefix === null ? 'xmlns' : prefix);
      if (!declaration) return undefined;
      var expectedAttrPrefix = prefix === null ? null : 'xmlns';
      if ($call($N['attr.prefix'], declaration) !== expectedAttrPrefix) return undefined;
      return $call($N['attr.value'], declaration) || null;
    }
    // Does this attribute declare `namespace` under some prefix (the attr's local name).
    function lookupPrefixAttrMatch(self, attribute, namespace){
      return $call($N['attr.namespaceURI'], attribute) === 'http://www.w3.org/2000/xmlns/' &&
        $call($N['attr.prefix'], attribute) === 'xmlns' &&
        $call($N['attr.value'], attribute) === namespace &&
        $call(nodeOperations.lookupNamespaceURI, self, $call($N['attr.localName'], attribute)) === namespace;
    }
    function lookupPrefixOnElement(self, element, namespace){
      var elementPrefix = $call($N['element.prefix'], element);
      if (elementPrefix && $call($N['element.namespaceURI'], element) === namespace && $call(nodeOperations.lookupNamespaceURI, self, elementPrefix) === namespace) return elementPrefix;
      var attributes = $call($N.nativeAttributes, element);
      for (var index = 0; index < attributes.length; index++) {
        var attribute = attributes[index];
        if (lookupPrefixAttrMatch(self, attribute, namespace)) return $call($N['attr.localName'], attribute);
      }
      return undefined;
    }
    // Cross-shadow-root order per Chrome 156: a stable WeakMap-keyed ordinal per root, OR'd
    // with the DOCUMENT_POSITION_DISCONNECTED | IMPLEMENTATION_SPECIFIC flags (0x21 = 33).
    function compareDocumentPositionCrossRoot(self, other){
      var order = __ns.nodeComparisonOrder || (__ns.nodeComparisonOrder = new WeakMap());
      var next = __ns.nodeComparisonNext || 1;
      if (!$c.has(order, self)) $c.set(order, self, next++);
      if (!$c.has(order, other)) $c.set(order, other, next++);
      __ns.nodeComparisonNext = next;
      return 33 | ($c.get(order, self) < $c.get(order, other) ? 4 : 2);
    }
    var nodeOperations={
      lookupNamespaceURI(prefix){
        prefix = prefix == null || prefix === '' ? null : attributeString(prefix);
        var element = nodeLookupStart(this);
        while (element) {
          if (prefix === 'xml') return 'http://www.w3.org/XML/1998/namespace';
          if (prefix === 'xmlns') return 'http://www.w3.org/2000/xmlns/';
          var elementNamespace = $call($N['element.namespaceURI'], element);
          if (elementNamespace && $call($N['element.prefix'], element) === prefix) return elementNamespace;
          var decl = lookupNamespaceURIDecl(element, prefix);
          if (decl !== undefined) return decl;
          element = $call($N.parentElement, element);
        }
        return null;
      },
      lookupPrefix(namespace){
        namespace = namespace == null || namespace === '' ? null : attributeString(namespace);
        if (namespace === null) return null;
        var element = nodeLookupStart(this);
        while (element) {
          var found = lookupPrefixOnElement(this, element, namespace);
          if (found !== undefined) return found;
          element = $call($N.parentElement, element);
        }
        return null;
      },
      isDefaultNamespace(namespace){namespace=namespace==null||namespace===''?null:String(namespace);return $call(nodeOperations.lookupNamespaceURI,this,null)===namespace;},
      normalize(){var nodes=this.childNodes;for(var index=0;index<nodes.length;){var node=nodes[index];if(node.nodeType!==3){node.normalize();index++;continue;}while(index+1<nodes.length&&nodes[index+1].nodeType===3){node.data+=nodes[index+1].data;this.removeChild(nodes[index+1]);nodes=this.childNodes;}if(!node.data){this.removeChild(node);nodes=this.childNodes;}else index++;}},
      compareDocumentPosition(other){
        getType.call(other);
        if (this === other) return 0;
        var root = $call($N.getRootNode, this), otherRoot = $call($N.getRootNode, other);
        if (root !== otherRoot) return compareDocumentPositionCrossRoot(this, other);
        if ($call($N.contains, this, other)) return 20;
        if ($call($N.contains, other, this)) return 10;
        var self = this, before = false, found = false;
        (function visit(node){
          if (found) return;
          if (node === self) before = true;
          if (node === other) { found = true; return; }
          for (var child = $call($N.firstChild, node); child !== null; child = $call($N.nextSibling, child)) visit(child);
        })(root);
        return before ? 4 : 2;
      }
    };
    Object.keys(nodeOperations).forEach(function(name){var delegate=nodeOperations[name];var fn=__ns.makeNativeFunction(function(){getType.call(this);if(arguments.length<delegate.length)throw new TypeError(name+' requires '+delegate.length+' arguments');return delegate.apply(this,arguments);},name,delegate.length);Object.defineProperty(nodeProto,name,{value:fn,writable:true,configurable:true,enumerable:true});});
    var tags={DIV:'HTMLDivElement',SPAN:'HTMLSpanElement',P:'HTMLParagraphElement',INPUT:'HTMLInputElement',TEXTAREA:'HTMLTextAreaElement',SELECT:'HTMLSelectElement',OPTION:'HTMLOptionElement',BUTTON:'HTMLButtonElement',LABEL:'HTMLLabelElement',CANVAS:'HTMLCanvasElement',A:'HTMLAnchorElement',FORM:'HTMLFormElement',STYLE:'HTMLStyleElement',SCRIPT:'HTMLScriptElement',IFRAME:'HTMLIFrameElement',IMG:'HTMLImageElement',UL:'HTMLUListElement',LI:'HTMLLIElement',TABLE:'HTMLTableElement'};
    Object.keys(htmlByTag).forEach(function(tag){tags[tag]=htmlByTag[tag];});
    Object.assign(tags,{TD:'HTMLTableCellElement',TH:'HTMLTableCellElement',THEAD:'HTMLTableSectionElement',TBODY:'HTMLTableSectionElement',TFOOT:'HTMLTableSectionElement',COLGROUP:'HTMLTableColElement',Q:'HTMLQuoteElement',BLOCKQUOTE:'HTMLQuoteElement'});
    for(var heading=1;heading<=6;heading++)tags['H'+heading]='HTMLHeadingElement';
    // HTML interfaces follow the exact namespace/local name, including XML-created nodes.
    var htmlInterfaces=Object.create(null);
    Object.keys(tags).forEach(function(tag){htmlInterfaces[tag.toLowerCase()]=tags[tag];});
    htmlInterfaces.listing=htmlInterfaces.xmp='HTMLPreElement';
    var genericHTML='abbr acronym address article aside b basefont bdi bdo big center code dd dfn dt em figcaption figure footer header hgroup i kbd main mark nav nobr noembed noframes noscript plaintext rb rp rt rtc ruby s samp search section small strike strong sub summary sup tt u var wbr'.split(' ');
    genericHTML.forEach(function(tag){htmlInterfaces[tag]='HTMLElement';});
    var customNames=/^[a-z][-._0-9a-z\u00b7\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u037d\u037f-\u1fff\u200c-\u200d\u203f-\u2040\u2070-\u218f\u2c00-\u2fef\u3001-\ud7ff\uf900-\ufdcf\ufdf0-\ufffd\u{10000}-\u{effff}]*-[-._0-9a-z\u00b7\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u037d\u037f-\u1fff\u200c-\u200d\u203f-\u2040\u2070-\u218f\u2c00-\u2fef\u3001-\ud7ff\uf900-\ufdcf\ufdf0-\ufffd\u{10000}-\u{effff}]*$/u;
    Object.defineProperty(customNames,'exec',{value:RegExp.prototype.exec});
    var reservedCustomNames=new Set(['annotation-xml','color-profile','font-face','font-face-src','font-face-uri','font-face-format','font-face-name','missing-glyph']);
    var projectionRegexTest=RegExp.prototype.test,projectionSetHas=Set.prototype.has;
    function htmlInterfaceName(tag){
      if(projectionApply(projectionHasOwn,htmlInterfaces,[tag]))return htmlInterfaces[tag];
      return projectionApply(projectionRegexTest,customNames,[tag])&&!projectionApply(projectionSetHas,reservedCustomNames,[tag])?'HTMLElement':'HTMLUnknownElement';
    }
    var projected=new WeakSet();
    // First wrapping can happen after page scripts replace the public builtins.
    var projectionApply=Reflect.apply,projectionSetPrototype=Object.setPrototypeOf;
    var projectionHasOwn=Object.prototype.hasOwnProperty,projectionHas=WeakSet.prototype.has,projectionAdd=WeakSet.prototype.add;
    var projectionSlice=String.prototype.slice;
    // Interface name for an Element (nodeType 1). SVG elements go to SVGSVGElement or
    // SVGElement depending on their (prefix-stripped) local name; xhtml / math / other
    // namespaces take their own interfaces.
    function projectNodeName1(tag, namespace){
      if (namespace === 'http://www.w3.org/2000/svg') {
        var localStart = 0;
        for (var index = 0; index < tag.length; index++) if (tag[index] === ':') localStart = index + 1;
        return projectionApply(projectionSlice, tag, [localStart]) === 'svg' ? 'SVGSVGElement' : 'SVGElement';
      }
      return namespace === 'http://www.w3.org/1999/xhtml' ? htmlInterfaceName(tag) : namespace === 'http://www.w3.org/1998/Math/MathML' ? 'MathMLElement' : 'Element';
    }
    // Interface name for every other node type (document, attr, text, fragment, …).
    var projectNodeTypeNames = { 2:'Attr', 10:'DocumentType', 3:'Text', 4:'CDATASection', 7:'ProcessingInstruction', 8:'Comment', 9:'HTMLDocument', 11:'DocumentFragment' };
    function projectNodeNameOther(obj, type){
      if (type === 11 && projectionApply(isShadowRoot, obj, [])) return 'ShadowRoot';
      return projectNodeTypeNames[type] || 'Node';
    }
    __ns.projectNativeNode=function(obj){
      if (projectionApply(projectionHas, projected, [obj])) return obj;
      var type = projectionApply(getType, obj, []), name;
      if (type === 1) {
        var tag = projectionApply(getLocalTag, obj, []), namespace = projectionApply(getNamespace, obj, []);
        name = projectNodeName1(tag, namespace);
      } else name = projectNodeNameOther(obj, type);
      projectionSetPrototype(obj, prototypes[name]);
      projectionApply(projectionAdd, projected, [obj]);
      for (var index = 0; index < nativeKeys.length; index++) { var key = nativeKeys[index]; if (projectionApply(projectionHasOwn, obj, [key])) delete obj[key]; }
      return obj;
    };
    __ns.projectNativeNode(d);
    var roots=[d.documentElement,d.head,d.body];
    roots.forEach(function(node){if(node)__ns.projectNativeNode(node);});
    var all=d.querySelectorAll('*');
    for(var i=0;i<all.length;i++)__ns.projectNativeNode(all[i]);
    __ns.projectNativeNode(probe);
  })();

})();
