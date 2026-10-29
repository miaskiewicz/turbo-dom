(function(){
  var g = globalThis;
  // Intrinsics captured at install time: the engine's load / insertion paths never call a
  // page-replaced builtin (miaskiewicz/turbo-dom#172).
  var $apply=Reflect.apply,$gopd=Object.getOwnPropertyDescriptor,$def=Object.defineProperty,$create=Object.create;
  var $wmGet=WeakMap.prototype.get,$wmSet=WeakMap.prototype.set,$lower=String.prototype.toLowerCase;
  var $jsonStringifyNative=JSON.stringify,$JSON=JSON,$jsonStringify=function(value){return $apply($jsonStringifyNative,$JSON,[value]);};
  var $fromCodePoint=String.fromCodePoint,$imul=Math.imul,$Mround=Math.round;
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
    var add=function(list,value){$def(list,list.length,{__proto__:null,value:value,writable:true,enumerable:true,configurable:true});};
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
    var split=function(text,pattern,limit){
      var out=[],max=limit===undefined?4294967295:limit>>>0;if(max===0)return out;
      var clone=globalClone(pattern),unicode=$apply(weakGet,cloneUnicode,[clone]),at=0,from=0,match;
      if(text===''){clone.lastIndex=0;match=$apply(exec,clone,[text]);if(!match)add(out,text);return out;}
      while(from<text.length){
        clone.lastIndex=from;match=$apply(exec,clone,[text]);
        if(!match||match.index>=text.length)break;
        var index=match.index,end=index+match[0].length;
        if(end===at){from=index+(unicode&&isPair(text,index)?2:1);continue;}
        add(out,$apply(strings.slice,text,[at,index]));if(out.length===max)return out;
        for(var i=1;i<match.length;i++){add(out,match[i]);if(out.length===max)return out;}
        at=end;from=at;
      }
      add(out,$apply(strings.slice,text,[at]));return out;
    };
    var dispatch=function(name){return function(receiver){var rest=[];for(var i=1;i<arguments.length;i++)add(rest,arguments[i]);var t=null;
      if(typeof receiver==='string'){if(name==='replace')return replace(receiver,rest[0],rest[1]);if(name==='split'&&rest[0] instanceof RegExpType)return split(receiver,rest[0],rest[1]);t=strings;}
      else if(typeof receiver==='function'){if(name==='call')return $apply(receiver,rest[0],$apply(arrays.slice,null,[rest,1]));if(name==='apply')return $apply(receiver,rest[0],rest[1]==null?[]:rest[1]);}
      else if(isArray(receiver)){if(arrays[name]){var args=[];add(args,receiver);for(var k=0;k<rest.length;k++)add(args,rest[k]);return $apply(arrays[name],null,args);}}
      else if(receiver instanceof MapType)t=maps;else if(receiver instanceof WeakMapType)t=weakMaps;else if(receiver instanceof SetType)t=sets;else if(receiver instanceof WeakSetType)t=weakSets;else if(receiver instanceof RegExpType){if(name==='test')return $apply(exec,receiver,rest)!==null;t=regexps;}
      if(t&&t[name])return $apply(t[name],receiver,rest);
      return $apply(receiver[name],receiver,rest);};};
    var out={__proto__:null},names=['get','set','has','delete','clear','add','slice','forEach','test','trim','push','indexOf','startsWith','splice','replace','toLowerCase','toUpperCase','map','concat','join','some','lastIndexOf','exec','filter','every','call','apply','endsWith','includes','split','charCodeAt','find'];
    for(var i=0;i<names.length;i++)out[names[i]]=dispatch(names[i]);
    return out;
  })();
  var compatibilityElementPrototype,computedStyleNativeMatches,computedStyleNativeGetAttribute,computedStyleNativeOwner,computedStyleNativeTag,computedStyleNativeNamespace,computedStyleNativeConnected,computedStyleNativeRoot;
  var normalizeComputedColor=g.__normalizeComputedColor;delete g.__normalizeComputedColor;
  var canvasColorNative=g.__canvasColorNative;delete g.__canvasColorNative;
  var computedFontContext=g.__computedFontContext;delete g.__computedFontContext;
  var computedStyleVersion=g.__computedStyleVersion;delete g.__computedStyleVersion;
  // Test builds only: counts fresh computed-style calculations (turbo-test#125 reuse tests).
  var noteCascadeRun=g.__noteCascadeRun;delete g.__noteCascadeRun;
  var cssomEpoch=g.__cssomEpoch,bumpCssomEpoch=g.__bumpCssomEpoch;delete g.__cssomEpoch;delete g.__bumpCssomEpoch;
  var cssFontNumber=Number,cssFontString=String,cssFontFinite=Number.isFinite,cssFontPrecision=Number.prototype.toPrecision,cssFontLower=String.prototype.toLowerCase;
  var prepareInlineHandler=g.__prepareInlineHandler,noteInlineHandler=g.__noteInlineHandler;delete g.__prepareInlineHandler;delete g.__noteInlineHandler;
  var collectionGet=g.__getCollectionRecord,collectionSet=g.__setCollectionRecord;
  var appliedInlineRead=g.__readAppliedInlineStyle,appliedInlineWrite=g.__writeAppliedInlineStyle,inlineSheetAllowed=g.__isInlineSheetAllowed;
  delete g.__readAppliedInlineStyle;delete g.__writeAppliedInlineStyle;delete g.__isInlineSheetAllowed;
  function cssMap(family){return {get:function(object){var shared=collectionGet(object);return shared&&shared.css&&shared.css[family];},set:function(object,value){var shared=collectionGet(object);if(!shared){shared={kind:'CSS'};collectionSet(object,shared);}if(!shared.css)shared.css=$create(null);shared.css[family]=value;},delete:function(object){var shared=collectionGet(object);if(shared&&shared.css)delete shared.css[family];}};}
  // turbo-surf shares this binding; its internal cross-context state lives on a Symbol-keyed
  // namespace (absent from getOwnPropertyNames(window) — real Chrome has no such __* globals),
  // not as `__*` string globals. Reachable from every realm via globalThis[Symbol.for('ts')].
  var __ns = g[Symbol.for('ts')] || (Object.defineProperty(g, Symbol.for('ts'), { value: {}, configurable: true, enumerable: false }), g[Symbol.for('ts')]);
  // The uncurried captured builtins, for the engine files that run after this bootstrap.
  __ns.intrinsics=$c;
  if (typeof g.__parseCssNative === 'function') { __ns.parseCssNative = g.__parseCssNative; delete g.__parseCssNative; }
  if (typeof g.__cssShorthandNative === 'function') { __ns.cssShorthandNative = g.__cssShorthandNative; delete g.__cssShorthandNative; }
  var cssShorthand = __ns.cssShorthandNative;
  var cssNativeParse=__ns.parseCssNative,cssSetPrototype=Object.setPrototypeOf,cssApply=Reflect.apply,cssMapGet=Function.prototype.call.bind(Map.prototype.get),cssTagUpper=Function.prototype.call.bind(String.prototype.toUpperCase);
  var computedSnapshotMap=WeakMap,computedSnapshotGet=Function.prototype.call.bind(WeakMap.prototype.get),computedSnapshotSet=Function.prototype.call.bind(WeakMap.prototype.set);
  if (typeof g.__domAccessor === 'function') { __ns.domAccessor = g.__domAccessor; delete g.__domAccessor; }
  computedStyleNativeOwner=__ns.domAccessor('ownerDocument',false,'Node');
  computedStyleNativeTag=__ns.domAccessor('tagName',false,'Element');
  computedStyleNativeNamespace=__ns.domAccessor('namespaceURI',false,'Element');
  computedStyleNativeConnected=__ns.domAccessor('isConnected',false,'Node');
  var createdLocalName=__ns.domAccessor('localName',false,'Element');
  if (typeof g.__nativeDOMFunction === 'function') { __ns.makeNativeFunction = g.__nativeDOMFunction; delete g.__nativeDOMFunction; }
  if (typeof g.__nativeNamedObject === 'function') { __ns.makeNamedObject = g.__nativeNamedObject; delete g.__nativeNamedObject; }
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
  function computedSnapshotValue(style,name){var record=declarationRecords.get(style),value=record&&cssMapGet(record.values,name);return value?value.text:'';}
  // Keep inherited unitless line-height as a factor: its public computed px value
  // alone cannot describe inheritance after a descendant changes font-size.
  function computedFonts(decl,element,parentStyle,connected,pseudo,parentSnapshot){
    var context=computedFontContext(element),parent=context&&context[2],root=context&&context[3];
    // A pseudo-element's parent is its originating element (or its parent pseudo-element).
    if(pseudo&&context){parent=element;root=null;}
    var initialFamily=context?context[0]:'',initialSize=context?context[1]:16;
    var inherited=parent?declarationRecords.get(parentSnapshot||parentStyle(parent)).fontState:null;
    var initial={__proto__:null,rootSize:initialSize,family:initialFamily,size:initialSize,style:'normal',weight:'400',stretch:'100%',caps:'normal',line:{__proto__:null,kind:'normal',value:0}};
    inherited=inherited||initial;
    function raw(name,inheritedValue,initialValue){var value=decl[name],key=typeof value==='string'?fontLower(value):value;return !value||$c.test(/^(inherit|unset|revert|revert-layer)$/, key)?inheritedValue:key==='initial'?initialValue:value;}
    function length(value,base,rem,typedLine){
      function term(text,scalar){
        var match=$c.exec(/^((?:\d*\.)?\d+)(px|em|rem|%|pt|pc|in|cm|mm|q)?$/i, text);if(!match)return null;
        var number=cssFontNumber(match[1]),unit=fontLower(match[2]||''),scale={__proto__:null,px:1,pt:96/72,pc:16,in:96,cm:96/2.54,mm:96/25.4,q:96/101.6};
        var resolved=unit==='em'?(base===null?null:number*base):unit==='rem'?(rem===null?null:number*rem):unit==='%'?(base===null?null:number*base/100):unit?number*scale[unit]:scalar||number===0?number:null;
        return resolved!==null&&cssFontFinite(resolved)?{__proto__:null,number:resolved,dimension:unit?1:0}:null;
      }
      function result(value){if(typedLine)return {__proto__:null,kind:value&&value.dimension===0?'factor':'length',value:value?value.number:null,text:source};return value&&(value.dimension===1||value.number===0)?value.number:null;}
      var source=value;
      if(!$c.test(/^calc\(/i, value))return result(term(value,false));
      var expression=$c.replace($c.replace(value, /calc\(/gi,'('), /\s+/g,''),tokens=[],cursor=0,tokenPattern=/(?:\d*\.)?\d+(?:[a-z]+|%)?|[()+*/-]/gi,tokenMatch;
      tokenPattern.lastIndex=0;while((tokenMatch=$c.exec(tokenPattern,expression))!==null)$c.push(tokens,tokenMatch[0]);
      if($c.join(tokens, '')!==expression)return result(null);
      function atom(){var token=tokens[cursor++];if(token==='+'||token==='-'){var signed=atom();if(signed&&token==='-')signed.number=-signed.number;return signed;}if(token==='('){var grouped=sum();return tokens[cursor++]===')'?grouped:null;}return term(token||'',true);}
      function product(){var value=atom();while(tokens[cursor]==='*'||tokens[cursor]==='/'){var operator=tokens[cursor++],right=atom();if(!value||!right||operator==='/'&&(right.number===0||right.dimension!==0)||operator==='*'&&value.dimension+right.dimension>1)return null;value={__proto__:null,number:operator==='*'?value.number*right.number:value.number/right.number,dimension:operator==='*'?value.dimension+right.dimension:value.dimension};}return value;}
      function sum(){var value=product();while(tokens[cursor]==='+'||tokens[cursor]==='-'){var operator=tokens[cursor++],right=product();if(!value||!right||value.dimension!==right.dimension)return null;value={__proto__:null,number:operator==='+'?value.number+right.number:value.number-right.number,dimension:value.dimension};}return value;}
      var calculated=sum();return result(cursor===tokens.length&&calculated&&cssFontFinite(calculated.number)?calculated:null);
    }
    function pixels(value){return cssFontString(cssFontNumber(cssApply(cssFontPrecision,value,[6])))+'px';}
    var rootSize=inherited.rootSize===undefined?initialSize:inherited.rootSize;
    var sizeValue=raw('font-size',inherited.size===null?inherited.sizeText:inherited.size,initialSize),size=typeof sizeValue==='number'?sizeValue:length(sizeValue,inherited.size,rootSize);
    var sizeKeywords={__proto__:null,'xx-small':9,'x-small':10,small:13,medium:16,large:18,'x-large':24,'xx-large':32,'xxx-large':48};
    var sizeKey=typeof sizeValue==='string'?fontLower(sizeValue):sizeValue;
    if(size===null){if(sizeKey==='smaller')size=inherited.size===null?null:inherited.size/1.2;else if(sizeKey==='larger')size=inherited.size===null?null:inherited.size*1.2;else if(sizeKeywords[sizeKey])size=sizeKeywords[sizeKey]*initialSize/16;}
    if(size!==null&&size<0)size=0;
    var family=raw('font-family',inherited.family,initial.family),style=raw('font-style',inherited.style,initial.style),weight=cssFontString(raw('font-weight',inherited.weight,initial.weight));
    var styleKey=fontLower(style);if($c.test(/^(normal|italic|oblique)$/, styleKey))style=styleKey;
    var weightKey=fontLower(weight);if($c.test(/^(normal|bold|bolder|lighter)$/, weightKey))weight=weightKey;
    family=$c.join($c.map(cssParts(family,','), function(part){var name=$c.trim(part),key=fontLower(name);return $c.test(/^(serif|sans-serif|monospace|cursive|fantasy|system-ui|ui-serif|ui-sans-serif|ui-monospace|ui-rounded|math|emoji|fangsong)$/, key)?key:name;}), ', ');
    if(weight==='normal')weight='400';else if(weight==='bold')weight='700';else if(weight==='bolder')weight=cssFontNumber(inherited.weight)<350?'400':cssFontNumber(inherited.weight)<550?'700':'900';else if(weight==='lighter')weight=cssFontNumber(inherited.weight)<550?'100':cssFontNumber(inherited.weight)<750?'400':'700';
    var stretch=raw('font-stretch',inherited.stretch,initial.stretch),stretchValues={__proto__:null,'ultra-condensed':'50%','extra-condensed':'62.5%',condensed:'75%','semi-condensed':'87.5%',normal:'100%','semi-expanded':'112.5%',expanded:'125%','extra-expanded':'150%','ultra-expanded':'200%'};
    stretch=stretchValues[fontLower(stretch)]||stretch;
    var caps=raw('font-variant-caps',raw('font-variant',inherited.caps,initial.caps),initial.caps);
    var capsKey=fontLower(caps);if($c.test(/^(normal|small-caps|all-small-caps|petite-caps|all-petite-caps|unicase|titling-caps)$/, capsKey))caps=capsKey;
    var lineValue=decl['line-height'],lineKey=typeof lineValue==='string'?fontLower(lineValue):lineValue,line;
    if(!lineValue||$c.test(/^(inherit|unset|revert|revert-layer)$/, lineKey))line=inherited.line;
    else if($c.test(/^(normal|initial)$/, lineKey))line=initial.line;
    else if($c.test(/^(?:\d*\.)?\d+$/, lineValue))line={__proto__:null,kind:'factor',value:cssFontNumber(lineValue)};
    else line=length(lineValue,size,rootSize,true);
    if(line.value!==null&&line.value<0)line={__proto__:null,kind:line.kind,value:0};
    var available=!!(connected&&context&&context[4]&&(parent?inherited.active:root===element));
    var state={__proto__:null,active:available,family:family,size:size,sizeText:cssFontString(sizeValue),style:style,weight:weight,stretch:stretch,caps:caps,line:line,rootSize:root===element?size:rootSize};
    var lineText=line.kind==='normal'?'normal':line.value===null?line.text:line.kind==='factor'&&size===null?cssFontString(line.value):pixels(line.kind==='factor'?line.value*size:line.value);
    var values={'font-family':family,'font-size':size===null?cssFontString(sizeValue):pixels(size),'font-style':style,'font-weight':weight,'font-stretch':stretch,'font-variant-caps':caps,'line-height':lineText};
    var shorthand=[];
    if(style!=='normal')$c.push(shorthand, style);if(caps!=='normal')$c.push(shorthand, caps);if(weight!=='400')$c.push(shorthand, weight);
    if(stretch!=='100%'){var stretchName=$c.find($O.keys(stretchValues),function(key){return stretchValues[key]===stretch;});$c.push(shorthand, stretchName||stretch);}
    $c.push(shorthand, values['font-size']);if(lineText!=='normal')$c.push(shorthand, '/ '+lineText);$c.push(shorthand, family);values.font=$c.join(shorthand, ' ');
    $c.forEach($O.keys(values), function(name){var value=available?values[name]:'';decl[name]=value;decl[$c.replace(name, /-([a-z])/g,function(_,letter){return $c.toUpperCase(letter);})]=value;});
    return state;
  }
  if (typeof g.getComputedStyle === 'undefined') g.getComputedStyle = function computeStyle(el){
    var pseudo=computedPseudoTarget(arguments[1]);
    // An invalid pseudo-element reads as an empty declaration (turbo-dom#47).
    if(pseudo===null)return __ns.makeComputedStyleDeclaration({},null,null);
    var snapshot=computeStyleSnapshot(el,new computedSnapshotMap(),undefined,pseudo||undefined);
    // A reused snapshot is shared: every call still returns its own declaration (Chrome).
    var entry=!pseudo&&el&&typeof el==='object'?computedSnapshotGet(computedStyleCache,el):null;
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
  // The CSS identifier at the start of `text` (escapes decoded, ASCII-lowercased) and its length.
  function cssIdentifierPrefix(text){
    var at=0,name='',start=true;
    function escape(){
      var hex=$c.exec(/^[0-9a-fA-F]{1,6}/,$c.slice(text,at+1,at+7));
      if(hex){var code=parseInt(hex[0],16);at+=1+hex[0].length;if($c.test(/[ \t\n\r\f]/,text[at]||''))at++;return code===0||code>0x10FFFF||code>=0xD800&&code<=0xDFFF?'\uFFFD':$apply($fromCodePoint,String,[code]);}
      var next=text[at+1];if(next===undefined||next==='\n')return null;at+=2;return next;
    }
    if(text[at]==='-'){name+='-';at++;if(text[at]==='-'){name+='-';at++;start=false;}}
    while(at<text.length){
      var c=text[at],part=null;
      if(c==='\\')part=escape();
      else if($c.test(/[A-Za-z_\u0080-\uFFFF]/,c)||!start&&$c.test(/[0-9-]/,c)){part=c;at++;}
      if(part===null)break;
      name+=part;start=false;
    }
    if(start)return null;
    return {name:$c.replace(name,/[A-Z]/g,function(c){return $c.toLowerCase(c);}),length:at};
  }
  // The pseudo-element a getComputedStyle(element, pseudoElt) read targets (turbo-dom#30, #47),
  // per Chrome 156: '' reads the element (no argument, a string without a leading colon that
  // is not a bare before / after, ::cue, unknown -webkit- / -internal- names), null is an
  // empty declaration (invalid, or a form Chrome computes no style for), else the
  // pseudo-element: its lowercase name, functional ones with their argument (`highlight(x)`,
  // `picker(select)`).
  function computedPseudoTarget(value){
    if(value===undefined||value===null)return '';
    var text=$c.replace(''+value,/\/\*[\s\S]*?(?:\*\/|$)/g,'');
    var colons=0;while(text[colons]===':')colons++;
    var rest=$c.slice(text,colons),ident=cssIdentifierPrefix(rest),name=ident&&ident.name;
    var bare=name==='before'||name==='after';
    if(colons===0){if(!ident)return '';if(ident.length<rest.length)return null;return bare?name:'';}
    if(colons>2||!ident)return null;
    if(ident.length<rest.length){
      var kind=computedPseudoFunctions[name];
      if(colons!==2||rest[ident.length]!=='('||rest[rest.length-1]!==')'||!kind)return null;
      var argument=$c.replace($c.slice(rest,ident.length+1,-1),/^[ \t\n\r\f]+|[ \t\n\r\f]+$/g,''),word=cssIdentifierPrefix(argument);
      if(kind==='direction'){var direction=argument==='*'?'*':word&&word.length===argument.length?word.name:'';return scrollButtonDirections[direction]?name+'('+direction+')':null;}
      if(!word||word.length!==argument.length)return null;
      if(kind==='select')return word.name==='select'?name+'(select)':null;
      return name+'('+argument+')';
    }
    if(colons===1)return bare||name==='first-line'||name==='first-letter'?name:null;
    if(computedPseudoNames[name])return name;
    return name==='cue'||$c.slice(name,0,8)==='-webkit-'||$c.slice(name,0,10)==='-internal-'?'':null;
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
  function computedUserAgentBox(el, layer){
    var namespace = '', tag = '';
    try { namespace = cssApply(computedStyleNativeNamespace, el, []); tag = cssApply(computedStyleNativeTag, el, []); } catch(e){}
    if (namespace === 'http://www.w3.org/1998/Math/MathML') {
      var parent = null; try { var context = computedFontContext(el); parent = context ? context[2] : null; } catch(e){}
      var parentNamespace = ''; try { if (parent) parentNamespace = cssApply(computedStyleNativeNamespace, parent, []); } catch(e){}
      var inMath = parentNamespace === namespace;
      return { display: inMath || computedNativeMatches(el, '[display=block i]') ? 'block math' : 'math', position: 'static' };
    }
    if (namespace !== 'http://www.w3.org/1999/xhtml') return { display: 'inline', position: 'static' };
    var name = cssApply(cssFontLower, String(tag), []), display = null;
    for (var i = 0; i < uaDisplayRules.length && display === null; i++) if (computedNativeMatches(el, uaDisplayRules[i][0])) display = uaDisplayRules[i][1];
    if (display === null) display = uaDisplayByTag[name] || 'inline';
    var position = computedNativeMatches(el, '[popover]') || layer === 'modal' ? 'fixed' : name === 'dialog' ? 'absolute' : 'static';
    return { display: display, position: position, topLayer: layer !== '' };
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
  var monospaceInputTypes = { __proto__: null, 'date': 1, 'time': 1, 'datetime-local': 1, 'month': 1, 'week': 1 };
  function formControlUserAgent(el, tag){
    if (tag === 'BUTTON') return { 'display': 'inline-block', 'font': '13.3333px Arial', 'margin': '0px', 'padding': '1px 6px', 'border-width': '2px', 'vertical-align': 'baseline' };
    if (tag === 'TEXTAREA') return { 'font': '13.3333px monospace' };
    if (tag === 'SELECT') return { 'font': '13.3333px Arial' };
    if (tag === 'OPTGROUP') return { 'font-weight': 'bolder' };
    if (tag === 'OPTION') return { 'font-weight': 'normal' };
    if (tag !== 'INPUT') return null;
    var type = ''; try { type = cssApply(cssFontLower, String(cssApply(computedStyleNativeGetAttribute, el, ['type']) || ''), []); } catch(e){}
    return { 'font': '13.3333px ' + (monospaceInputTypes[type] ? 'monospace' : 'Arial') };
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
  // Split a selector list at its top-level commas (outside (), [] and strings).
  function selectorListItems(text){
    var items = [], depth = 0, quote = '', start = 0;
    for (var i = 0; i < text.length; i++) {
      var c = text[i];
      if (c === '\\') { i++; continue; }
      if (quote) { if (c === quote) quote = ''; continue; }
      if (c === '"' || c === "'") quote = c;
      else if (c === '(' || c === '[') depth++;
      else if (c === ')' || c === ']') depth--;
      else if (c === ',' && depth === 0) { $c.push(items, $c.slice(text, start, i)); start = i + 1; }
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
  function pseudoRuleMatches(el, selectorText, pseudo){
    var items = selectorListItems(String(selectorText));
    var wanted = typeof pseudo === 'string' ? $c.exec(/^([a-z-]+)(?:\((.*)\))?$/, pseudo) : null;
    for (var i = 0; i < items.length; i++) {
      var item = $c.trim(items[i]), match = $c.exec(/(::?)([A-Za-z-]+)(?:\(\s*([^()]*?)\s*\))?$/, item);
      if (!match || match.index > 0 && item[match.index - 1] === '\\') continue;
      var name = $c.toLowerCase(match[2]), argument = match[3];
      name = pseudoSelectorAliases[name] || name;
      if (match[1] === ':' && !$c.test(/^(before|after|first-line|first-letter)$/, name)) continue;
      if (wanted) {
        if (name !== wanted[1] || (argument === undefined) !== (wanted[2] === undefined)) continue;
        if (argument !== undefined && argument !== '*' && wanted[2] !== '*') {
          var same = wanted[1] === 'picker' || wanted[1] === 'scroll-button' ? $c.toLowerCase(argument) === wanted[2] : argument === wanted[2];
          if (!same) continue;
        }
      } else if (!pseudo(name)) continue;
      var origin = $c.slice(item, 0, match.index);
      if (origin === '' || $c.test(/[\s>+~]$/, origin)) origin += '*';
      if (computedNativeMatches(el, origin)) return true;
    }
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
  function computedInheritedAuthored(el, name, parentStyle, fallback, own){
    var node = el;
    for (var depth = 0; node && depth < 256; depth++) {
      var value = '';
      try { value = computedSnapshotValue(parentStyle(node), name); } catch(e){}
      if (!value && own) value = own(node);
      if (value && !$c.test(/^(inherit|unset)$/i, value)) return $c.test(/^initial$/i, value) ? fallback : value;
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
  function computedPseudoInfo(el, pseudo, parentStyle){
    var kind = $c.replace(pseudo, /\(.*$/, ''), tag = computedHtmlTag(el);
    var info = { kind: kind, authored: true, accepts: null, ua: {}, important: null, display: 'inline', position: 'static', container: true, generated: false, highlight: false };
    if (kind === 'marker') {
      var inside = markerInside(el, parentStyle);
      info.display = inside ? 'inline' : 'inline-block';
      info.ua = { 'white-space': inside ? computedInheritedAuthored(el, 'white-space', parentStyle, 'normal') : 'pre', 'font-variant-numeric': 'tabular-nums', 'unicode-bidi': 'isolate', 'text-transform': 'none' };
    } else if (kind === 'first-line' || kind === 'first-letter') {
      info.container = kind === 'first-line';
    } else if (highlightPseudoNames[kind]) {
      info.highlight = true; info.container = false; kind = 'highlight';
      if (info.kind === 'spelling-error' || info.kind === 'grammar-error') info.ua = { 'text-decoration-line': info.kind };
    } else if (kind === 'placeholder') {
      var input = tag === 'INPUT' && placeholderInputTypes[computedInputType(el)];
      info.authored = (input || tag === 'TEXTAREA') && computedHasAttribute(el, 'placeholder');
      if (info.authored) {
        info.display = 'block';
        info.ua = { 'color': 'rgb(117, 117, 117)', 'unicode-bidi': 'isolate' };
        if (input) { info.ua['white-space'] = 'pre'; info.ua.overflow = 'hidden'; }
        info.important = { 'pointer-events': 'none' };
      }
    } else if (kind === 'file-selector-button') {
      info.authored = tag === 'INPUT' && computedInputType(el) === 'file';
      if (info.authored) {
        info.display = 'inline-block';
        info.ua = { 'background-color': 'rgb(239, 239, 239)', 'white-space': 'nowrap', 'padding': '1px 6px', 'margin': '0px 4px 0px 0px', 'border-width': '2px', 'border-style': 'outset', 'text-align': 'center', 'box-sizing': 'border-box', 'user-select': 'none' };
      }
    } else if (kind === 'details-content') {
      info.authored = tag === 'DETAILS';
      if (info.authored) { info.display = 'block'; info.ua = { 'content-visibility': computedHasAttribute(el, 'open') ? 'visible' : 'hidden' }; }
    } else if (kind === 'picker') {
      info.authored = tag === 'SELECT';
      if (info.authored) { info.display = 'none'; info.position = 'fixed'; info.ua = { 'overflow': 'auto', 'white-space': 'normal', 'unicode-bidi': 'isolate' }; }
    } else if (kind === 'checkmark' || kind === 'picker-icon') {
      info.generated = true;
      if (kind === 'picker-icon' && tag === 'SELECT') info.display = 'block';
    } else if (kind === 'scroll-marker-group') {
      info.display = 'block';
    } else if ($c.slice(kind, 0, 15) === 'view-transition' && tag) {
      info.display = 'block'; info.position = 'absolute';
      info.ua = kind === 'view-transition' || kind === 'view-transition-image-pair' ? { 'top': '0px', 'right': '0px', 'bottom': '0px', 'left': '0px' } : kind === 'view-transition-group' ? { 'top': '0px', 'right': 'auto', 'bottom': 'auto', 'left': '0px' } : { 'top': '0px', 'right': 'auto', 'bottom': 'auto', 'left': 'auto' };
    }
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
  function computedSheets(el){
    var owner=el?cssApply(computedStyleNativeOwner,el,[]):null;
    if(owner&&cssApply(computedStyleNativeRoot,el,[])!==owner&&cssApply(computedStyleNativeConnected,el,[]))return [];
    // A live list refreshes on every indexed/length read. Refresh its private
    // backing array once per computation; retained declarations still recompute.
    var ownerRecord=owner&&collectionGet(owner),host=ownerRecord&&ownerRecord.cssHost;
    var sheets = host ? host.readSheets() : [];
    // Document adoption follows ordinary sheets, but is not a shadow-tree scope.
    var adopted=adoptionStates&&$c.get(adoptionStates,owner);
    if(adopted&&cssApply(computedStyleNativeRoot,el,[])===owner){var active=[];cssSetPrototype(active,null);for(var index=0;index<sheets.length;index++)active[active.length]=sheets[index];for(var index=0;index<adopted.target.length;index++)active[active.length]=adopted.target[index];sheets=active;}
    return sheets;
  }
  // The selector text of every enabled style rule that can select `el`.
  function computedStyleRules(el){
    var out=[],sheets=[];try{sheets=computedSheets(el);}catch(e){}
    for (var si=0; si<sheets.length; si++) {
      if(sheetRecord(sheets[si]).disabled)continue;
      var rules = appliedSheetRuleRecords(sheets[si]);
      for (var ri=0; ri<rules.length; ri++) {
        var rule=rules[ri];
        if(rule.type===1){$c.push(out, rule.selectorText);continue;}
        var txt = String(cssRuleText(rule) || ''), br = $c.indexOf(txt, '{');
        if (br >= 0) $c.push(out, $c.trim($c.slice(txt, 0, br)));
      }
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
  // it read. A computation that read something outside the key is volatile and never reused:
  // a rule whose selector depends on live form / popover / dialog state and could match, an
  // animated element, the fullscreen element (viewport-sized), an @import's media query.
  var computedStyleCache=new computedSnapshotMap(),computedVolatile=false;
  function invalidateComputedStyles(){bumpCssomEpoch();}
  __ns.invalidateComputedStyles=invalidateComputedStyles;
  function computedCacheKey(el){
    var version;try{version=computedStyleVersion(el);}catch(e){return null;}
    if(typeof version!=='number')return null;
    var owner=null;try{owner=cssApply(computedStyleNativeOwner,el,[]);}catch(e){return null;}
    return {__proto__:null,epoch:cssomEpoch(),version:version,owner:owner};
  }
  // What the key cannot see: the element's top layer and font context.
  function computedCacheProbe(el){
    var connected=false;try{connected=cssApply(computedStyleNativeConnected,el,[]);}catch(e){}
    var layer='';try{if(connected)layer=computedTopLayer(el);}catch(e){}
    var context=null;try{context=computedFontContext(el);}catch(e){}
    return context?[layer,context[0],context[1],context[2],context[3],context[4]]:[layer];
  }
  function computedProbeEqual(a,b){if(a.length!==b.length)return false;for(var index=0;index<a.length;index++)if(a[index]!==b[index])return false;return true;}
  function computedCacheHit(el,key,snapshots){
    var entry=computedSnapshotGet(computedStyleCache,el);
    if(!entry||entry.epoch!==key.epoch||entry.version!==key.version||entry.owner!==key.owner)return null;
    if(!computedProbeEqual(entry.probe,computedCacheProbe(el)))return null;
    for(var index=0;index<entry.deps.length;index+=2)if(computeStyleSnapshot(entry.deps[index],snapshots)!==entry.deps[index+1])return null;
    return entry.snapshot;
  }
  function computeStyleSnapshot(el,snapshots,sink,pseudo){
    if(pseudo||sink||!el||typeof el!=='object')return computeStyleFresh(el,snapshots,sink,pseudo,null);
    var local=computedSnapshotGet(snapshots,el);if(local)return local;
    var key=computedCacheKey(el);
    if(key){var hit=computedCacheHit(el,key,snapshots);if(hit){computedSnapshotSet(snapshots,el,hit);return hit;}}
    var deps=[],outer=computedVolatile,snapshot,volatile;computedVolatile=false;
    try{snapshot=computeStyleFresh(el,snapshots,sink,pseudo,deps);volatile=computedVolatile;}finally{computedVolatile=outer;}
    if(key){
      var after=computedCacheKey(el);
      if(volatile||!after||after.epoch!==key.epoch||after.version!==key.version||after.owner!==key.owner)computedSnapshotSet(computedStyleCache,el,null);
      else computedSnapshotSet(computedStyleCache,el,{__proto__:null,snapshot:snapshot,epoch:key.epoch,version:key.version,owner:key.owner,probe:computedCacheProbe(el),deps:deps});
    }
    return snapshot;
  }
  // Selector-dependent volatility: the pseudo-classes whose truth is live binding state (form
  // control state, open popovers, modal dialogs, active view transitions, interest invokers)
  // rather than the tree. A rule using one makes
  // the computation volatile when it matches, or when it could match in another state: its
  // selector with every such pseudo-class (and every non-matching-list function containing one)
  // relaxed to `:is(*)` matches. Relaxing only widens a selector, so a rule that cannot match
  // even relaxed cannot match in any state.
  var computedStatePattern=/:(?:checked|indeterminate|default|placeholder-shown|read-only|read-write|in-range|out-of-range|valid|invalid|user-valid|user-invalid|enabled|disabled|required|optional|popover-open|modal|open|active-view-transition|active-view-transition-type|interest-source|interest-target)(?![\w-])/i;
  var computedStateName=/^(?:checked|indeterminate|default|placeholder-shown|read-only|read-write|in-range|out-of-range|valid|invalid|user-valid|user-invalid|enabled|disabled|required|optional|popover-open|modal|open|active-view-transition|active-view-transition-type|interest-source|interest-target)$/i;
  var computedListFunction=/^(?:is|where|has|-webkit-any|matches)$/i;
  var computedRelaxed={__proto__:null},computedRelaxedCount=0;
  function computedRelaxSelector(text){
    var out='',index=0;
    while(index<text.length){
      var c=text[index];
      if(c==='\\')return '*';
      if(c==='"'||c==="'"){var end=index+1;while(end<text.length&&text[end]!==c){if(text[end]==='\\')end++;end++;}out+=$c.slice(text, index,end+1);index=end+1;continue;}
      if(c!==':'||text[index+1]===':'){if(c===':'){out+='::';index+=2;continue;}out+=c;index++;continue;}
      var name=cssApply(cssZoomExec,/^[A-Za-z0-9_-]*/,[$c.slice(text, index+1)])[0],after=index+1+name.length;
      if(text[after]!=='('){out+=cssApply(cssZoomExec,computedStateName,[name])?':is(*)':':'+name;index=after;continue;}
      var depth=1,close=after+1;
      while(close<text.length&&depth){var d=text[close];if(d==='\\')return '*';if(d==='"'||d==="'"){var q=close+1;while(q<text.length&&text[q]!==d)q++;close=q+1;continue;}if(d==='(')depth++;else if(d===')')depth--;close++;}
      var inner=$c.slice(text, after+1,close-1);
      if(cssApply(cssZoomExec,computedStateName,[name]))out+=':is(*)';
      else if(cssApply(cssZoomExec,computedListFunction,[name]))out+=':'+name+'('+computedRelaxSelector(inner)+')';
      else out+=cssApply(cssZoomExec,computedStatePattern,[inner])?':is(*)':$c.slice(text, index,close);
      index=close;
    }
    return out;
  }
  function computedStateVolatile(el,selector,matched){
    if(computedVolatile||!cssApply(cssZoomExec,computedStatePattern,[selector]))return;
    if(matched){computedVolatile=true;return;}
    var relaxed=computedRelaxed[selector];
    if(relaxed===undefined){if(computedRelaxedCount>=4096){computedRelaxed={__proto__:null};computedRelaxedCount=0;}relaxed=computedRelaxed[selector]=computedRelaxSelector(selector);computedRelaxedCount++;}
    var could=true;try{could=!!cssApply(computedStyleNativeMatches,el,[relaxed]);}catch(e){}
    if(could)computedVolatile=true;
  }
  function computeStyleFresh(el,snapshots,sink,pseudo,deps){
    var cached=!pseudo&&el&&computedSnapshotGet(snapshots,el);if(cached)return cached;
    if(noteCascadeRun)noteCascadeRun();
    // Percentage lookup may visit several ancestors before font inheritance asks
    // for the immediate parent again. Reuse each resolved ancestor only in this read.
    function parentStyle(node){var style=computeStyleSnapshot(node,snapshots);if(deps&&deps[deps.length-2]!==node){deps[deps.length]=node;deps[deps.length]=style;}return style;}
    // A disconnected element has no style (turbo-dom#67): every value reads "" until it is
    // connected again (the declaration stays live). Typed OM's cascade read (`sink`) still
    // reports the cascaded values.
    if (!pseudo && !sink && el) {
      var attached = true;
      try { attached = __ns.domAccessor('isConnected',false,'Node').call(el); } catch(e) {}
      if (!attached) { var empty = __ns.makeComputedStyleDeclaration({},el,null); computedSnapshotSet(snapshots,el,empty); return empty; }
    }
    // A pseudo-element of a disconnected (or <video>/<audio> child) element has no style.
    if (pseudo) {
      var live = false;
      try { live = __ns.domAccessor('isConnected',false,'Node').call(el) && !computedMediaChild(el); } catch(e) {}
      if (!live) return __ns.makeComputedStyleDeclaration({},el,null,pseudo);
    }
    var camel = function(p){ return $c.replace(p,/-([a-z])/g, function(_,c){ return $c.toUpperCase(c); }); };
    var decl = { __proto__:null, getPropertyValue: function(p){ var v = this[p]; if (v == null) v = this[camel(p)]; return v == null ? '' : String(v); }, getPropertyPriority: function(){ return ''; }, length: 0 };
    // Minimal cascade: merge declarations from every injected stylesheet rule whose selector matches
    // `el` (emotion's `sx` -> `.css-xxx{...}`), then overlay inline declarations,
    // retaining per-longhand importance through shorthand expansion. Common authored
    // values remain observable before layout-dependent resolution.
    // Properties are exposed both kebab and camelCase.
    // Normalize comma-spacing ("a,b" -> "a, b"): minified emotion rules drop the space, but jest-dom
    // toHaveStyle's expected value is normalized by the browser's CSS parser (which inserts ", "),
    // so font-family / shorthand lists must match that form.
    // Normalize authored hex colors to the computed rgb()/rgba() spelling used by the
    // declaration records and style assertions.
    var normColor = function(v){
      var m = $c.exec(/^#([0-9a-fA-F]{3})$/, v); if (m){ var x=m[1]; return 'rgb('+parseInt(x[0]+x[0],16)+', '+parseInt(x[1]+x[1],16)+', '+parseInt(x[2]+x[2],16)+')'; }
      m = $c.exec(/^#([0-9a-fA-F]{6})$/, v); if (m){ var h=m[1]; return 'rgb('+parseInt($c.slice(h, 0,2),16)+', '+parseInt($c.slice(h, 2,4),16)+', '+parseInt($c.slice(h, 4,6),16)+')'; }
      m = $c.exec(/^#([0-9a-fA-F]{8})$/, v); if (m){ var h2=m[1]; return 'rgba('+parseInt($c.slice(h2, 0,2),16)+', '+parseInt($c.slice(h2, 2,4),16)+', '+parseInt($c.slice(h2, 4,6),16)+', '+($Mround(parseInt($c.slice(h2, 6,8),16)/255*100)/100)+')'; }
      return v;
    };
    // Length-valued properties resolve a bare zero to the computed "0px" spelling.
    var LEN_PROP = /(width|height|^top$|^right$|^bottom$|^left$|^inset|margin|padding|gap|^flex-basis$|radius|^font-size$|letter-spacing|word-spacing|text-indent|^outline-offset$|^column-(width|gap)$|size$)/;
    // Priority is private per computed snapshot; public CSSStyleDeclaration methods
    // and page overrides cannot change the importance of parsed declarations.
    var uaValues=null,priorities={__proto__:null};
    // A cascaded longhand outdates the shorthand entries an earlier declaration (a UA table)
    // stored; the shorthand being expanded keeps its own.
    var expandingShorthand = null;
    var setProp = function(name, val, important){ name = $c.trim(String(name)); if (!name) return; if(important!==undefined){if(priorities[name]&&!important)return;priorities[name]=important;} if(uaValues&&typeof val==='string'&&$c.test(/^revert$/i,$c.trim(val))&&uaValues[name]!==undefined)val=uaValues[name]; if (typeof val === 'string' && $c.slice(name,0, 2) !== '--'){ var t = $c.trim(val); if (t[0] === '#') val = normColor(t); else if (t === '0' && $c.test(LEN_PROP,name)) val = '0px'; else if ($c.indexOf(val,',') >= 0) val = cssPrelude(val,true); } decl[name] = val; decl[camel(name)] = val; $c.forEach(staleShorthands(name),function(shorthand){ if (shorthand !== expandingShorthand) { delete decl[shorthand]; delete decl[camel(shorthand)]; } }); if (sink) sink.order[name] = ++sink.count;
      if (typeof val === 'string') { var outer = expandingShorthand; expandingShorthand = name; try { expandShorthand(name, val, function(property,value){setProp(property,value,important);}); } finally { expandingShorthand = outer; } }
    };
    function applyDeclarations(record,accepts){for(var index=0;index<record.names.length;index++){var name=record.names[index],value=cssMapGet(record.values,name);if(accepts&&$c.slice(name,0,2)!=='--'&&!accepts(name))continue;setProp(name,value.pending?value.pending[1]:value.raw||value.text,value.priority==='important');}}
    var info=pseudo&&pseudo!=='backdrop'&&pseudo!=='before'&&pseudo!=='after'?computedPseudoInfo(el,pseudo,parentStyle):null;
    // Measured Chrome button defaults are UA declarations: every matching author
    // rule (including a universal selector) and inline value takes precedence.
    var ua = null, layer = '';
    try { if(pseudo||cssApply(computedStyleNativeConnected,el,[]))layer=computedTopLayer(el); } catch(e){}
    try { if(pseudo==='backdrop')ua=backdropUserAgent(layer);else if(info)ua=info.ua;else if(!pseudo&&layer==='modal')ua=modalDialogUserAgent;else if(!pseudo&&cssApply(computedStyleNativeNamespace,el,[])==='http://www.w3.org/1999/xhtml'&&cssApply(computedStyleNativeConnected,el,[])){var fontContext=computedFontContext(el);if(fontContext&&fontContext[4])ua=formControlUserAgent(el,cssApply(computedStyleNativeTag,el,[]));}
      if(!pseudo&&layer==='fullscreen')ua=$O.assign({},ua||{},fullscreenUserAgent); } catch(e){}
    if(ua){$c.forEach($O.keys(ua), function(name){setProp(name,ua[name]);});uaValues=$O.create(null);$c.forEach($O.keys(decl), function(name){if(typeof decl[name]==='string')uaValues[name]=decl[name];});}
    try {
      // Element-backed pseudo-elements the element does not have see no author rules.
      var sheets = info && !info.authored ? [] : computedSheets(el);
      for (var si=0; si<sheets.length; si++) {
        if(sheetRecord(sheets[si]).disabled)continue;
        var rules = appliedSheetRuleRecords(sheets[si]);
        for (var ri=0; ri<rules.length; ri++) {
          var rule=rules[ri],matched=false;
          if(pseudo&&rule.type===1){
            try{matched=pseudoRuleMatches(el,rule.selectorText,pseudo);}catch(e){}
          }else if(rule.type===1){
            // Match the private selector list before touching declarations. Native
            // parsing preserves quoted delimiters that serialized text splitting loses.
            try{matched=cssApply(computedStyleNativeMatches,el,[rule.selectorText]);}catch(e){}
            if(!pseudo)computedStateVolatile(el,rule.selectorText,matched);
          }else{
            var txt = String(cssRuleText(rule) || ''); var br = $c.indexOf(txt, '{'); if (br < 0) continue;
            var sel = $c.trim($c.slice(txt, 0, br)),parts = $c.split(sel, ',');
            if(pseudo){try{matched=pseudoRuleMatches(el,sel,pseudo);}catch(e){}parts=[];}
            for (var pi=0; pi<parts.length; pi++){ var s = $c.trim(parts[pi]); if (!s) continue; try { if (cssApply(computedStyleNativeMatches,el,[s])) { matched = true; } } catch(e){} computedStateVolatile(el,s,matched); if (matched) break; }
          }
          if (!matched) continue;
          applyDeclarations(declarationRecord(rule.style),info&&info.accepts);
        }
      }
    } catch(e){}
    // overlay inline style (own props React/components set: background, height, display, ...)
    var st = !pseudo && el && __ns.makeInlineStyleDeclaration(el);
    if (st) applyDeclarations(declarationRecord(st));
    // !important UA declarations (::placeholder pointer-events, the fullscreen element's box)
    // beat every author one.
    var important = info ? info.important : !pseudo && layer === 'fullscreen' ? fullscreenImportant : null;
    if (important) $c.forEach($O.keys(important), function(name){ setProp(name, important[name], true); });
    if (sink) { var cascaded = { __proto__: null }; $c.forEach($O.keys(decl),function(name){ if (typeof decl[name] === 'string' && ($c.slice(name,0, 2) === '--' || name === $c.toLowerCase(name))) cascaded[name] = decl[name]; }); sink.values = cascaded; return null; }
    // Inherited values come from the composed parent's snapshot in this read, so
    // CSSOM edits to document sheets reach descendants. Native cascade only adds
    // shadow-scoped rules (shadow <style>, :host, ::slotted) the JS cascade lacks.
    var inheritParent = null;
    try { var inheritContext = computedFontContext(el); inheritParent = pseudo ? el : inheritContext ? inheritContext[2] : null; } catch(e) {}
    // A fresh internal result is already resolved. Reading its public live getter
    // would compute the same ancestors again and double recursion at every level.
    // A view-transition pseudo-element inherits from its parent in the pseudo-element tree.
    var inheritSnapshot = info && info.parentPseudo ? computeStyleSnapshot(el, snapshots, undefined, info.parentPseudo) : null;
    function inheritedValue(name){ return inheritSnapshot ? computedSnapshotValue(inheritSnapshot,name) : inheritParent ? computedSnapshotValue(parentStyle(inheritParent),name) : ''; }
    // Web Animations compose over cascaded and inline values; !important declarations win.
    if(!pseudo&&el&&__ns.hasAnimations&&__ns.hasAnimations(el))computedVolatile=true;
    var animated=!pseudo&&el&&__ns.animatedStyle?__ns.animatedStyle(el,function(name){var value=decl[name];return value==null?'':String(value);},inheritedValue):null;
    if(animated)for(var ai=0;ai<animated.length;ai++)if(!priorities[animated[ai][0]])setProp(animated[ai][0],animated[ai][1]);
    var color = decl.color || '';
    if (!color && !pseudo) { try { color = __ns.cascadeProp.call(el, 'color'); } catch(e) {} }
    // A highlight in highlight-inheritance mode inherits color / background-color from the
    // parent's highlight, or takes the initial values (turbo-dom#47).
    var highlightParent = undefined;
    if (info && info.highlight) {
      var highlightMode = false; try { highlightMode = computedHighlightMode(el, pseudo); } catch(e){}
      if (highlightMode) {
        var highlightContext = null; try { highlightContext = computedFontContext(el); } catch(e){}
        var parentElement = highlightContext ? highlightContext[2] : null;
        highlightParent = parentElement && computedHighlightMode(parentElement, pseudo) ? computeStyleSnapshot(parentElement, snapshots, undefined, pseudo) : null;
        var inheritsKey = function(value){ return !value || $c.test(/^(inherit|unset)$/i, $c.trim(String(value))); };
        if (inheritsKey(color)) color = highlightParent ? computedSnapshotValue(highlightParent, 'color') : 'rgb(0, 0, 0)';
        if (inheritsKey(decl['background-color'])) setProp('background-color', highlightParent ? computedSnapshotValue(highlightParent, 'background-color') : 'rgba(0, 0, 0, 0)');
      } else if (decl['background-color'] == null) setProp('background-color', 'rgba(0, 0, 0, 0)');
    }
    var colorKeyword = $c.toLowerCase(String(color || ''));
    if (colorKeyword === 'initial' || colorKeyword === 'canvastext') color = 'rgb(0, 0, 0)';
    else if (!color || colorKeyword === 'inherit' || colorKeyword === 'unset' || colorKeyword === 'currentcolor') {
      color = inheritParent ? inheritedValue('color') : 'rgb(0, 0, 0)';
    }
    var connected = false;
    try { connected = __ns.domAccessor('isConnected',false,'Node').call(el); } catch(e) {}
    // Children of <video> / <audio> are outside the flat tree: Chrome computes no style for them.
    if (connected && computedMediaChild(el)) connected = false;
    setProp('color', connected ? normalizeComputedColor(color) : '');
    // UA and initial values for display / position / visibility (turbo-surf#36), measured in
    // computed_defaults_chrome156.json: an author / inline / shadow-scoped declaration wins, the
    // CSS-wide keywords resolve (inherit -> the composed parent's value, initial / unset -> the
    // initial value, revert -> the UA value), otherwise Chrome's html.css value; display is then
    // blockified for floats, absolutely positioned elements and flex / grid items. Disconnected
    // elements keep "" like every other default above.
    var cascadedValue = function(name){
      var value = decl[name];
      if ((value == null || value === '') && !pseudo && el && typeof __ns.cascadeProp === 'function') { try { value = __ns.cascadeProp.call(el, name) || ''; } catch(e){ value = ''; } }
      return value == null ? '' : $c.trim(String(value));
    };
    var keyword = function(value){ var key = $c.toLowerCase(value); return $c.test(/^(inherit|initial|unset|revert|revert-layer)$/, key) ? key : ''; };
    if (connected) {
      var ua = pseudo === 'backdrop' ? { display: 'block', position: layer ? 'fixed' : 'absolute', topLayer: layer !== '' } : pseudo ? { display: info ? info.display : 'inline', position: info ? info.position : 'static', topLayer: false } : computedUserAgentBox(el, layer);
      var displayValue = cascadedValue('display'), displayKey = keyword(displayValue);
      if (!displayValue || displayKey === 'revert' || displayKey === 'revert-layer') displayValue = ua.display;
      else if (displayKey === 'inherit') displayValue = inheritedValue('display') || 'inline';
      else if (displayKey) displayValue = 'inline';
      var positionValue = cascadedValue('position'), positionKey = keyword(positionValue);
      if (!positionValue || positionKey === 'revert' || positionKey === 'revert-layer') positionValue = ua.position;
      else if (positionKey === 'inherit') positionValue = inheritedValue('position') || 'static';
      else if (positionKey) positionValue = 'static';
      positionValue = $c.toLowerCase(positionValue);
      if (ua.topLayer) positionValue = topLayerPosition(positionValue);
      var floatValue = $c.toLowerCase($c.trim(String(decl['float'] == null ? '' : decl['float'])));
      var floated = floatValue !== '' && floatValue !== 'none' && !keyword(floatValue);
      if (positionValue === 'absolute' || positionValue === 'fixed' || floated || (pseudo ? pseudo !== 'backdrop' && (!info || info.container) && computedPseudoInContainer(el, parentStyle) : computedFlexOrGridItem(el, parentStyle)))
        displayValue = blockifiedDisplay(displayValue);
      decl.display = displayValue;
      decl.position = positionValue;
    } else if (decl.display == null) decl.display = '';
    // `visibility` is INHERITED: a child with no explicit visibility takes the parent's computed
    // value. testing-library / dom-accessibility-api rely on this — they treat an element as
    // inaccessible when getComputedStyle(el).visibility === 'hidden', expecting a hidden ancestor to
    // have propagated down. The JS cascade above only sees this element's own rules, so we defer to
    // rtdom's native cascade (cascade::computed_style) for shadow-scoped rules; the composed
    // parent's snapshot supplies inheritance and the root's value is the initial `visible`.
    var visibilityValue = connected ? cascadedValue('visibility') : String(decl.visibility == null ? '' : decl.visibility);
    var visibilityKey = keyword(visibilityValue);
    if (visibilityKey === 'initial') visibilityValue = 'visible';
    else if (connected && (visibilityValue === '' || visibilityKey)) { try { visibilityValue = inheritedValue('visibility'); } catch(e){ visibilityValue = ''; } }
    if (!visibilityValue && connected) visibilityValue = 'visible';
    // Highlights never take the element's visibility (turbo-dom#47).
    if (connected && info && info.highlight) visibilityValue = 'visible';
    decl.visibility = visibilityValue || '';
    // `content` computes to normal, and to none on ::before / ::after / ::checkmark /
    // ::picker-icon; strings serialize double-quoted (turbo-dom#47).
    var contentValue = decl.content == null ? '' : $c.trim(String(decl.content));
    if (pseudo && contentValue) contentValue = $c.replace(contentValue, /'((?:[^'\\\n]|\\[\s\S])*)'/g, function(_, inner){ return '"' + $c.replace($c.replace(inner, /\\'/g, "'"), /"/g, '\\"') + '"'; });
    if (pseudo === 'before' || pseudo === 'after' || info && info.generated) decl.content = contentValue === '' || $c.test(/^normal$/i, contentValue) ? 'none' : contentValue;
    else if (pseudo && contentValue) decl.content = contentValue;
    else if (decl.content == null) decl.content = connected ? 'normal' : '';
    // Initial computed values of properties Web Animations commonly animate.
    if (decl.opacity == null) decl.opacity = connected ? '1' : '';
    if (decl.transform == null) { decl.transform = connected ? 'none' : ''; }
    // `zoom` is not inherited; it computes to its factor, with normal and 0 as 1.
    decl.zoom = connected ? computedZoom(decl.zoom, function(){ return inheritedValue('zoom'); }) : '';
    // Used-value resolution for percentage width/height. A no-layout env returns a raw "50%"
    // verbatim, but a containing-block percentage has a well-defined used value in normal flow
    // (pct * the containing block's content width/height), which a real browser reports in px.
    // Resolve against the nearest ancestor with a px (or, for width, auto=fill) measure so
    // getComputedStyle(child).width matches a browser without a full layout engine.
    // A modal dialog's overflow longhands recompose the shorthand.
    if (connected && !pseudo && layer === 'modal' && typeof decl['overflow-x'] === 'string' && typeof decl['overflow-y'] === 'string')
      decl.overflow = decl['overflow-x'] === decl['overflow-y'] ? decl['overflow-x'] : decl['overflow-x'] + ' ' + decl['overflow-y'];
    if (pseudo && typeof decl['background-color'] === 'string') decl['background-color'] = decl.backgroundColor = normalizeComputedColor(decl['background-color']);
    var usedPx = function(node, axis, depth){
      if (!node || depth > 32) return null;
      var raw;try{raw=node===el?decl[axis]:computedSnapshotValue(parentStyle(node),axis);}catch(e){return null;}
      raw = raw == null ? '' : String(raw);
      var mpx = $c.exec(/^(-?\d+(?:\.\d+)?)px$/, raw); if (mpx) return parseFloat(mpx[1]);
      var parent = node.parentElement || (node.parentNode && node.parentNode.nodeType === 1 ? node.parentNode : null);
      var mpc = $c.exec(/^(-?\d+(?:\.\d+)?)%$/, raw);
      if (mpc) { var base = usedPx(parent, axis, depth + 1); return base == null ? null : (parseFloat(mpc[1]) / 100) * base; }
      // width:auto in normal flow fills the containing block (block box); height:auto is content-driven (unknown here).
      if ((raw === '' || raw === 'auto') && axis === 'width') return usedPx(parent, axis, depth + 1);
      return null;
    };
    // The fullscreen element's width / height (100% !important of the viewport, its
    // containing block as a fixed box) are the viewport's innerWidth / innerHeight in px
    // (turbo-dom#58); without a viewport they stay 100%.
    if (!pseudo && layer === 'fullscreen') {
      computedVolatile = true;
      [['width', g.innerWidth], ['height', g.innerHeight]].forEach(function(pair){
        var size = +pair[1];
        if (size > 0 && cssFontFinite(size)) setProp(pair[0], ($Mround(size * 1000) / 1000) + 'px', true);
      });
    }
    if (!pseudo && layer !== 'fullscreen') try {
      $c.forEach(['width', 'height'], function(axis){
        var raw = decl[axis];
        if (typeof raw === 'string' && $c.test(/^-?\d+(?:\.\d+)?%$/, $c.trim(raw))) {
          var px = usedPx(el, axis, 0);
          if (px != null && isFinite(px)) setProp(axis, ($Mround(px * 1000) / 1000) + 'px');
        }
      });
    } catch(e){}
    var fontState=computedFonts(decl,el,parentStyle,connected,pseudo,inheritSnapshot);
    // dialog:modal max-width / max-height: calc(100% - 6px - 2em) at the dialog's font size.
    if (connected && !pseudo && layer === 'modal' && fontState && typeof fontState.size === 'number') {
      var inset = 'calc(100% - ' + cssFontString(cssFontNumber(cssApply(cssFontPrecision, 2 * fontState.size + 6, [6]))) + 'px)';
      $c.forEach(['max-width', 'max-height'], function(name){ if (decl[name] == null) decl[name] = decl[camel(name)] = inset; });
    }
    var snapshot=__ns.makeComputedStyleDeclaration(decl,el,fontState,pseudo);
    if(el&&!pseudo)computedSnapshotSet(snapshots,el,snapshot);
    return snapshot;
  }
  if (typeof g.requestAnimationFrame === 'undefined') g.requestAnimationFrame = function(cb){ return setTimeout(function(){ cb(Date.now()); }, 0); };
  if (typeof g.cancelAnimationFrame === 'undefined') g.cancelAnimationFrame = function(id){ clearTimeout(id); };
  if (typeof g.matchMedia === 'undefined') g.matchMedia = function(q){ return { matches:false, media:q, addListener:function(){}, removeListener:function(){}, addEventListener:function(){}, removeEventListener:function(){}, dispatchEvent:function(){return false;} }; };
  if (typeof g.scrollTo === 'undefined') g.scrollTo = function(){};
  // `Image(width, height)` is the native legacy factory in html_image.js (turbo-dom#23); image
  // loading (load / error events, natural size) is the host's (record.imageHost).
  if (typeof g.location === 'undefined' || g.location == null) {
    g.location = { href: 'http://localhost/', origin: 'http://localhost', protocol: 'http:', host: 'localhost', hostname: 'localhost', port: '', pathname: '/', search: '', hash: '', assign: function(){}, replace: function(){}, reload: function(){}, toString: function(){ return this.href; } };
  }
  // DOM interface constructors (for `instanceof` / global presence). Stubs; identity not enforced.
  // Event records stay private: readonly Web-IDL getters and native dispatch
  // share this state without exposing writable bookkeeping to page scripts.
  var eventRecords = new WeakMap();
  var getEventRecord = WeakMap.prototype.get.bind(eventRecords);
  var setEventRecord = WeakMap.prototype.set.bind(eventRecords);
  var eventClockOrigin = Date.now();
  var eventDefineProperty = Object.defineProperty;
  function copyEventPath(path) {
    var result = [];
    for (var index=0; index<path.length; index++) eventDefineProperty(result,index,{__proto__:null,value:path[index],writable:true,configurable:true,enumerable:true});
    return result;
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
    function Event(type, init = {}) {
      if (!new.target && !(this instanceof Event)) throw new TypeError('Event requires new');
      if (!arguments.length) throw new TypeError('Event requires a type');
      if (init != null && typeof init !== 'object' && typeof init !== 'function') throw new TypeError('EventInit must be a dictionary');
      init = init || {};
      var state = {__proto__:null,type:eventString(type), bubbles:!!init.bubbles, cancelable:!!init.cancelable,
        composed:!!init.composed, defaultPrevented:false, target:null, currentTarget:null,
        stop:false, stopImmediate:false, eventPhase:0, timeStamp:g.performance&&typeof g.performance.now==='function'?g.performance.now():Date.now()-eventClockOrigin,
        trusted:false, dispatching:false, passive:false, path:[]};
      setEventRecord(this,state);
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
    member(MouseEvent.prototype,'initMouseEvent',{value:function initMouseEvent(type,bubbles=false,cancelable=false,view=null,detail=0,screenX=0,screenY=0,clientX=0,clientY=0,ctrlKey=false,altKey=false,shiftKey=false,metaKey=false,button=0,relatedTarget=null){stateOf(mouseStates,this);if(!arguments.length)throw new TypeError('A type is required');if(eventRecord(this).dispatching)return;$call(initUIEvent,this,type,bubbles,cancelable,view,detail);$call($wmSet,mouseStates,this,mouseState({screenX,screenY,clientX,clientY,ctrlKey,altKey,shiftKey,metaKey,button,relatedTarget}));relatedRecord(this,$call($wmGet,mouseStates,this).relatedTarget);}});
    tagEvent(MouseEvent,'MouseEvent');g.MouseEvent=MouseEvent;
    class KeyboardEvent extends UIEvent {
      constructor(type,init={}){if(!arguments.length)throw new TypeError('A type is required');super(type,init);init=init||{};var state=modifiers(init);state.key=stringValue(init.key);state.code=stringValue(init.code);state.location=intValue(init.location)>>>0;state.repeat=!!init.repeat;state.isComposing=!!init.isComposing;state.charCode=intValue(init.charCode)>>>0;state.keyCode=intValue(init.keyCode)>>>0;$call($wmSet,keyStates,this,state);}
    }
    readonly(KeyboardEvent.prototype,keyStates,['key','code','location','ctrlKey','shiftKey','altKey','metaKey','repeat','isComposing','charCode','keyCode']);
    member(KeyboardEvent.prototype,'getModifierState',{value:function getModifierState(key){if(!arguments.length)throw new TypeError('A modifier is required');return modifier(keyStates,this,key);}});
    member(KeyboardEvent.prototype,'initKeyboardEvent',{value:function initKeyboardEvent(type,bubbles=false,cancelable=false,view=null,key='',location=0,ctrlKey=false,altKey=false,shiftKey=false,metaKey=false){var state=stateOf(keyStates,this);if(!arguments.length)throw new TypeError('A type is required');if(eventRecord(this).dispatching)return;$call(initUIEvent,this,type,bubbles,cancelable,view,0);state.key=eventString(key);state.location=intValue(location)>>>0;state.ctrlKey=!!ctrlKey;state.altKey=!!altKey;state.shiftKey=!!shiftKey;state.metaKey=!!metaKey;}});
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
    g.document.createEvent = function(){ return new Event('', {}); };
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
  if (typeof g.Audio === 'undefined') {
    g.Audio = function Audio(src){
      var el = d.createElement('audio');
      if (src != null) { try { el.src = String(src); } catch(e){} }
      return el;
    };
    try { g.Audio.prototype = (g.HTMLAudioElement && g.HTMLAudioElement.prototype) || {}; } catch(e){}
  }
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
  if (typeof g.Worker === 'undefined') {
    g.Worker = function Worker(url){ this.url = String(url || ''); this.onmessage = null; this.onmessageerror = null; this.onerror = null; this._l = {}; };
    g.Worker.prototype.postMessage = function(){};
    g.Worker.prototype.terminate = function(){};
    g.Worker.prototype.addEventListener = function(t, f){ (this._l[t] = this._l[t] || []).push(f); };
    g.Worker.prototype.removeEventListener = function(t, f){ var a = this._l[t]; if (a){ var i = a.indexOf(f); if (i >= 0) a.splice(i, 1); } };
    g.Worker.prototype.dispatchEvent = function(){ return true; };
    if (typeof g.SharedWorker === 'undefined') { g.SharedWorker = function SharedWorker(url){ this.url = String(url || ''); this.port = { postMessage: function(){}, start: function(){}, close: function(){}, addEventListener: function(){}, removeEventListener: function(){}, onmessage: null }; }; }
  }
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
    try{if(list)for(var listener of list.slice()){
      if(state.stopImmediate)break;
      try{typeof listener==='function'?listener.call(target,event):listener.handleEvent(event);}catch(e){}
    }}finally{if(!legacyWindow)endEventDispatch(state);}
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
  var cssParts = function(text, separator){
    var out=[], start=0, depth=0, quote='', escaped=false, comment=false;
    for (var i=0;i<text.length;i++) {
      var c=text[i], next=i+1<text.length?text[i+1]:undefined;
      if (comment) { if(c==='*'&&next==='/'){comment=false;i++;} continue; }
      if (quote) { if(escaped) escaped=false; else if(c==='\\') escaped=true; else if(c===quote) quote=''; continue; }
      if(c==='/'&&next==='*'){comment=true;i++;continue;} if(c==='"'||c==="'"){quote=c;continue;}
      if(c==='('||c==='[') depth++; else if(c===')'||c===']') depth--;
      if(c===separator&&depth===0){$c.push(out,$c.slice(text,start,i));start=i+1;}
    }
    $c.push(out,$c.slice(text,start)); return out;
  };
  function cssPrelude(text,commas){
    var out='',quote='',escaped=false,comment=false;
    for(var i=0;i<text.length;i++){
      var c=text[i],next=i+1<text.length?text[i+1]:undefined;
      if(comment){if(c==='*'&&next==='/'){comment=false;i++;}continue;}
      if(quote){out+=c;if(escaped)escaped=false;else if(c==='\\')escaped=true;else if(c===quote)quote='';continue;}
      if(c==='\\'){out+=c;if(next!==undefined)out+=text[++i];continue;}
      if(c==='/'&&next==='*'){comment=true;i++;continue;}
      if(c==='"'||c==="'"){quote=c;out+=c;continue;}
      if(commas&&c===','){out=$c.replace(out,/\s*$/,'')+', ';while(i+1<text.length&&$c.test(/\s/,text[i+1]))i++;}
      else out+=c;
    }
    return $c.trim(out);
  }
  var declarationRecords=cssMap('declaration'),inlineDeclarations=cssMap('inlineDeclaration'),cssPropertyCache=new Map();
  function declarationRecord(style){var record=$c.get(declarationRecords,style);if(!record)throw new TypeError('Illegal invocation');if(record.owner){var text=$c.call(appliedInlineRead,record.owner);if(text!==record.source){record.source=text;parseDeclaration(record,text);}}if(record.refresh){var fresh=$c.get(declarationRecords,record.refresh());if(fresh){record.values=fresh.values;record.names=fresh.names;record.fontState=fresh.fontState;}}return record;}
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
  function colorText(name,text){if(!$c.test(/(?:^color$|color$)/,name))return text;var match=$c.exec(/^#([\da-f]{3}|[\da-f]{6})$/i,text);if(!match)return text;var hex=match[1];if(hex.length===3)hex=$c.replace(hex,/./g,function(c){return c+c;});return 'rgb('+parseInt($c.slice(hex,0,2),16)+', '+parseInt($c.slice(hex,2,4),16)+', '+parseInt($c.slice(hex,4,6),16)+')';}
  // `raw` is the stylesheet grammar's text of a longhand set directly: the computed cascade
  // reads it (full number precision), CSSOM reads the Chrome spelling in `text`.
  function storeDeclaration(record,name,text,priority,pending,raw){if(!$c.has(record.values,name))$c.push(record.names,name);$c.set(record.values,name,{text:pending?'':colorText(name,text),priority:priority||'',pending:pending||null,raw:raw||null});}
  // cssText parsing: a repeated property moves to the end; an !important one keeps out a later normal one.
  function parsedDeclaration(record,name,text,priority,pending,raw){var previous=$c.get(record.values,name);if(previous){if(previous.priority&&!priority)return;$c.delete(record.values,name);$c.splice(record.names,$c.indexOf(record.names,name),1);}storeDeclaration(record,name,text,priority,pending,raw);}
  // A longhand's value as Chrome spells it (turbo-dom#97): [text, raw] or null. rtdom's grammar
  // admits (or rejects) and spells the longhands it knows; any other value the stylesheet
  // grammar admits, rtdom spells context-free. `raw` is the stylesheet grammar's text.
  function longhandText(name,value){var key=name+'\u0000'+value,cached=$c.get(longhandTexts,key);if(cached!==undefined)return cached;var raw=parseLonghandText(name,value),result=raw===null?null:[raw,raw];if(cssShorthand&&$c.slice(name,0,2)!=='--'){var spelled=cssShorthand('longhand',name,value);if(spelled[0]==='valid')result=[spelled[1],raw===null?spelled[1]:raw];else if(spelled[0]==='invalid')result=null;else if(raw!==null)result=[spelled[1],raw];else if(spelled[1]!==value&&parseLonghandText(name,spelled[1])!==null)result=[spelled[1],spelled[1]];}if($mapSize(longhandTexts)>=4096)$c.clear(longhandTexts);$c.set(longhandTexts,key,result);return result;}
  // A var() / env() value is accepted at parse time once the native parser kept it; Typed OM
  // has no typed reading of it (turbo-dom#162).
  function deferredCssText(text){return $c.test(/(^|[^-\w])(var|env)\(/i,text);}
  function parseLonghandText(name,value){if(!$c.startsWith(name,'--')&&!knownProperty(name))return null;var parsed=typeof cssNativeParse==='function'?cssNativeParse('x { '+name+': '+value+' }'):'x { '+name+': '+value+' }';var body=$c.slice(parsed,$c.indexOf(parsed,'{')+1,$c.lastIndexOf(parsed,'}')),accepted=null;$c.forEach(cssParts(body,';'),function(part){var index=$c.indexOf(part,':');if(index<1||$c.trim($c.slice(part,0,index))!==name)return;var text=$c.trim($c.slice(part,index+1));if($c.test(/\s*!important\s*$/i,text))return;if(!$c.startsWith(name,'--')&&!deferredCssText(text)&&__ns.typedOmValid&&!__ns.typedOmValid(name,text))return;accepted=text;});return accepted;}
  // The [[name, text, pending, raw], ...] a declaration sets, or null when it is invalid.
  function declarationEntries(name,value){name=propertyAlias(name);if(shorthandName(name))return expandShorthandValue(name,value);var text=longhandText(name,value);return text===null?null:[[name,text[0],null,text[1]]];}
  function parseDeclaration(record,text){if(cssShorthand&&!record.fontFace){record.names=[];record.values=new Map();var list=cssShorthand('declarations',text);for(var index=0;index<list.length;index+=3){var name=list[index];if($c.slice(name,0,2)!=='--')name=$c.toLowerCase(name);var entries=declarationEntries(name,list[index+1]);if(entries)$c.forEach(entries,function(entry){parsedDeclaration(record,entry[0],entry[1],list[index+2]?'important':'',entry[2],entry[3]);});}return;}parseLegacyDeclaration(record,text);}
  function parseLegacyDeclaration(record,text){record.names=[];record.values=new Map();var selector=record.fontFace?'@font-face':'x',parsed=typeof cssNativeParse==='function'?cssNativeParse(selector+' { '+text+' }'):selector+' { '+text+' }';var body=$c.slice(parsed,$c.indexOf(parsed,'{')+1,$c.lastIndexOf(parsed,'}'));$c.forEach(cssParts(body,';'),function(part){var index=$c.indexOf(part,':');if(index<1)return;var name=$c.trim($c.slice(part,0,index));if(!record.fontFace&&!$c.startsWith(name,'--')&&!knownProperty(name))return;var value=$c.trim($c.slice(part,index+1)),important=$c.test(/\s*!important\s*$/i,value);if(important)value=$c.trim($c.replace(value,/\s*!important\s*$/i,''));if(!record.fontFace&&!$c.startsWith(name,'--')&&__ns.typedOmValid&&!__ns.typedOmValid(name,value))return;var previous=$c.get(record.values,name);if(previous&&previous.priority&&!important)return;storeDeclaration(record,name,value,important?'important':'');});}
  function writableDeclaration(record){if(record.readOnly)throw new __ns.DOMException('The declaration is read-only','NoModificationAllowedError');invalidateComputedStyles();}
  function commitDeclaration(record){if(record.owner){record.source=declarationText(record);$c.call(appliedInlineWrite,record.owner,record.source);}}
  // removeProperty: a shorthand removes its longhands and returns '' (Chrome).
  function removeDeclaration(record,name){if(!record.fontFace){name=propertyAlias(name);var longhands=shorthandName(name)&&cssShorthand('longhands',name);if(longhands){var removed=false;$c.forEach(longhands,function(longhand){if($c.delete(record.values,longhand)){$c.splice(record.names,$c.indexOf(record.names,longhand),1);removed=true;}});if(removed)commitDeclaration(record);return '';}}var previous=$c.get(record.values,name);if(!previous)return '';$c.delete(record.values,name);$c.splice(record.names,$c.indexOf(record.names,name),1);commitDeclaration(record);return previous.text;}
  function setDeclaration(record,name,value,priority){writableDeclaration(record);name=cssString(name);value=value===null?'':cssString(value);priority=priority==null?'':$c.toLowerCase(cssString(priority));if(!value){removeDeclaration(record,name);return;}if(cssShorthand&&!record.fontFace){if(priority&&priority!=='important')return;if($c.slice(name,0,2)!=='--')name=$c.toLowerCase(name);var entries=declarationEntries(name,value);if(!entries)return;$c.forEach(entries,function(entry){storeDeclaration(record,entry[0],entry[1],priority,entry[2],entry[3]);});commitDeclaration(record);return;}if(priority&&priority!=='important'||!record.fontFace&&!$c.startsWith(name,'--')&&!knownProperty(name))return;var parsed={names:[],values:new Map(),fontFace:record.fontFace};parseDeclaration(parsed,name+': '+value+(priority?' !important':'')+';');var accepted=$c.get(parsed.values,name);if(!accepted&&record.fontFace&&knownProperty(name)){parsed.fontFace=false;parseDeclaration(parsed,name+': '+value+(priority?' !important':'')+';');accepted=$c.get(parsed.values,name);}if(!accepted)return;storeDeclaration(record,name,accepted.text,accepted.priority);commitDeclaration(record);}
  // Typed OM writes (StylePropertyMap.set / append): Chrome stores the CSSStyleValue, so a
  // longhand keeps the value's own serialization (calc() sums in their order, products,
  // percentages) instead of the specified-value spelling setProperty applies (turbo-dom#97).
  __ns.setDeclarationAuthored=function(style,name,value){var record=declarationRecord(style);writableDeclaration(record);name=propertyAlias(cssString(name));value=cssString(value);if(record.fontFace||!cssShorthand||$c.slice(name,0,2)==='--'||shorthandName(name)){setDeclaration(record,name,value,'');return;}var raw=parseLonghandText(name,value);if(raw===null)return;storeDeclaration(record,name,raw,'',null,raw);commitDeclaration(record);};
  // Typed OM reads reify the value as parsed (Chrome's expression tree: calc() terms in their
  // authored order), not the specified-value spelling getPropertyValue returns.
  __ns.declarationAuthoredValue=function(style,name){var record=declarationRecord(style);if(record.fontFace||!cssShorthand)return propertyValue(record,name);name=propertyAlias(cssString(name));var value=$c.get(record.values,name);if(value)return value.pending||!value.raw?value.text:colorText(name,value.raw);return shorthandName(name)?shorthandValue(record,name):'';};
  // getPropertyValue: a shorthand serializes from its longhands (a computed declaration's own
  // shorthand entry wins); a pending longhand reads ''.
  function propertyValue(record,name){if(!record.fontFace&&cssShorthand){name=propertyAlias(name);var value=$c.get(record.values,name);if(value)return value.text;return shorthandName(name)?shorthandValue(record,name):'';}var value=$c.get(record.values,name);return value?value.text:'';}
  var declarationConstructor=__ns.makeNativeFunction(function(){throw new TypeError('Illegal constructor');},'CSSStyleDeclaration',0,true);
  declarationConstructor.prototype={};$O.defineProperty(declarationConstructor.prototype,'constructor',{value:declarationConstructor,writable:true,configurable:true});$O.defineProperty(declarationConstructor.prototype,Symbol.toStringTag,{value:'CSSStyleDeclaration',configurable:true});$O.defineProperty(declarationConstructor,'prototype',{writable:false});$O.defineProperty(g,'CSSStyleDeclaration',{value:declarationConstructor,writable:true,configurable:true,enumerable:false});
  var declarationPrototype=declarationConstructor.prototype;
  function declarationMember(name,get,set){var descriptor={enumerable:true,configurable:true,get:__ns.makeNativeFunction(get,'get '+name,0)};if(set)descriptor.set=__ns.makeNativeFunction(set,'set '+name,1);$O.defineProperty(declarationPrototype,name,descriptor);}
  declarationMember('length',function(){return declarationRecord(this).names.length;});
  declarationMember('parentRule',function(){return declarationRecord(this).parentRule||null;});
  declarationMember('cssFloat',function(){var value=$c.get(declarationRecord(this).values,'float');return value?value.text:'';},function(value){setDeclaration(declarationRecord(this),'float',value,'');});
  declarationMember('cssText',function(){var record=declarationRecord(this);return record.computed?'':declarationText(record);},function(value){var record=declarationRecord(this);writableDeclaration(record);parseDeclaration(record,cssString(value));commitDeclaration(record);});
  var declarationMethods={getPropertyValue:function(name){if(!arguments.length)throw new TypeError('getPropertyValue requires 1 argument');if(!$c.get(declarationRecords,this))throw new TypeError('Illegal invocation');name=cssString(name);var record=declarationRecord(this);return propertyValue(record,name);},getPropertyPriority:function(name){if(!arguments.length)throw new TypeError('getPropertyPriority requires 1 argument');var record=declarationRecord(this);name=cssString(name);if(!record.fontFace&&cssShorthand){name=propertyAlias(name);var longhands=shorthandName(name)&&cssShorthand('longhands',name);if(longhands)return $c.every(longhands,function(longhand){var value=$c.get(record.values,longhand);return value&&value.priority==='important';})?'important':'';}var value=$c.get(record.values,name);return value?value.priority:'';},setProperty:function(name,value,priority){if(arguments.length<2)throw new TypeError('setProperty requires 2 arguments');setDeclaration(declarationRecord(this),name,value,priority);},removeProperty:function(name){if(!arguments.length)throw new TypeError('removeProperty requires 1 argument');var record=declarationRecord(this);writableDeclaration(record);return removeDeclaration(record,cssString(name));},item:function(index){if(!arguments.length)throw new TypeError('item requires 1 argument');return declarationRecord(this).names[Number(index)>>>0]||'';}};
  $c.forEach($O.keys(declarationMethods),function(name){$O.defineProperty(declarationPrototype,name,{value:__ns.makeNativeFunction(declarationMethods[name],name,name==='setProperty'?2:1),writable:true,enumerable:true,configurable:true});});
  $O.defineProperty(declarationPrototype,Symbol.iterator,{value:Array.prototype.values,writable:true,configurable:true});
  function declarationProperty(name,record){if(typeof name!=='string'||$c.startsWith(name,'--'))return null;var cssName=cssPropertyName(name);return knownProperty(cssName)||record&&record.fontFace&&$c.indexOf(['src','font-display','unicode-range','size-adjust','ascent-override','descent-override','line-gap-override'],cssName)>=0?cssName:null;}
  var cssDeclaration=function(text,owner,parentRule,readOnly,fontFace){var target=$O.create(declarationPrototype),record={__proto__:null,fontFace:!!fontFace,refresh:null,names:[],values:new Map(),owner:owner||null,parentRule:parentRule||null,source:String(text||''),readOnly:!!readOnly},proxy;parseDeclaration(record,record.source);proxy=new Proxy(target,{get:function(obj,key,receiver){var index=cssIndex(key);if(index>=0)return declarationRecord(proxy).names[index];if($R.has(obj,key))return $R.get(obj,key,receiver);var name=declarationProperty(key,record);return name?propertyValue(declarationRecord(proxy),name):undefined;},set:function(obj,key,value,receiver){if(cssIndex(key)>=0)return false;if($R.has(obj,key))return $R.set(obj,key,value,receiver);var name=declarationProperty(key,record);if(name){setDeclaration(declarationRecord(proxy),name,value,'');return true;}return $R.set(obj,key,value,receiver);},has:function(obj,key){return cssIndex(key)>=0?cssIndex(key)<declarationRecord(proxy).names.length:$R.has(obj,key)||!!declarationProperty(key,record);},ownKeys:function(){return $c.concat($c.map(declarationRecord(proxy).names,function(_,i){return String(i);}),$R.ownKeys(target));},getOwnPropertyDescriptor:function(obj,key){var index=cssIndex(key),names=declarationRecord(proxy).names;if(index>=0&&index<names.length)return {value:names[index],writable:false,enumerable:true,configurable:true};var descriptor=$R.getOwnPropertyDescriptor(obj,key);if(descriptor)return descriptor;if($R.has(declarationPrototype,key))return undefined;var name=declarationProperty(key,record);return name?{value:propertyValue(declarationRecord(proxy),name),writable:true,enumerable:true,configurable:true}:undefined;},deleteProperty:function(obj,key){return cssIndex(key)<0&&$R.deleteProperty(obj,key);}});$c.set(declarationRecords,proxy,record);return proxy;};
  var inlineDeclarationReceiver=__ns.domAccessor('nodeType',false,'Element');
  __ns.makeInlineStyleDeclaration=function(element){$c.call(inlineDeclarationReceiver,element);var style=$c.get(inlineDeclarations,element);if(!style){style=cssDeclaration($c.call(appliedInlineRead,element),element);$c.set(inlineDeclarations,element,style);}return style;};
  var computedStyleReader=g.getComputedStyle;
  __ns.cloneComputedStyleDeclaration=function(snapshot,element){var source=$c.get(declarationRecords,snapshot),style=cssDeclaration(''),record=$c.get(declarationRecords,style);record.values=source.values;record.names=source.names;record.readOnly=true;record.computed=true;record.fontState=source.fontState;record.refresh=function(){return computedStyleReader(element);};return style;};
  __ns.makeComputedStyleDeclaration=function(values,element,fontState,pseudo){var style=cssDeclaration(''),record=$c.get(declarationRecords,style);$c.forEach($O.keys(values),function(name){var cssName=cssPropertyName(name),value=values[name];if(typeof value==='string'&&value&&knownProperty(cssName))storeDeclaration(record,cssName,value,'');});record.readOnly=true;record.computed=true;record.fontState=fontState;if(element)record.refresh=pseudo?function(){return computedStyleReader(element,'::'+pseudo);}:function(){return computedStyleReader(element);};return style;};
  var cssRuleStates=cssMap('rule');
  var cssRuleRecord=function(rule,type){var record=$c.get(cssRuleStates,rule);if(!record||type&&record.type!==type)throw new TypeError('Illegal invocation');return record;};
  function cssRuleInterface(name,parent){var ctor=__ns.makeNativeFunction(function(){throw new TypeError('Illegal constructor');},name,0,true);ctor.prototype=$O.create(parent||$O.prototype);$O.defineProperty(ctor.prototype,'constructor',{value:ctor,writable:true,configurable:true});$O.defineProperty(ctor.prototype,Symbol.toStringTag,{value:name,configurable:true});$O.defineProperty(ctor,'prototype',{writable:false});if(parent)$O.setPrototypeOf(ctor,parent.constructor);$O.defineProperty(g,name,{value:ctor,writable:true,configurable:true,enumerable:false});}
  cssRuleInterface('CSSRule');cssRuleInterface('CSSStyleRule',g.CSSRule.prototype);cssRuleInterface('CSSImportRule',g.CSSRule.prototype);
  // Every CSSOM setter and mutating method drops reused computed styles (turbo-test#125).
  function cssInvalidating(fn){return function(){try{return cssApply(fn,this,arguments);}finally{invalidateComputedStyles();}};}
  function cssRuleMember(owner,name,get,set){var descriptor={enumerable:true,configurable:true,get:__ns.makeNativeFunction(get,'get '+name,0)};if(set)descriptor.set=__ns.makeNativeFunction(cssInvalidating(set),'set '+name,1);$O.defineProperty(g[owner].prototype,name,descriptor);}
  $c.forEach(['type','parentRule','parentStyleSheet'],function(name){cssRuleMember('CSSRule',name,function(){return cssRuleRecord(this)[name];});});
  function cssRuleText(record){if(record.type===1||record.type===5){var text=declarationText(declarationRecord(record.style));return (record.type===1?record.selectorText:record.prelude)+' {'+(text?' '+text+' ':' ')+'}';}
    // Chrome 156: grouping rules and @keyframes list one child rule per line (turbo-dom#134).
    if((record.grouping||record.type===7)&&record.cssRules){var rules=cssItems(record.cssRules),out=record.prelude+(record.type===7?' { \n':' {\n');for(var index=0;index<rules.length;index++)out+='  '+cssRuleText(cssRuleRecord(rules[index]))+'\n';return out+'}';}
    return record.cssText;}
  __ns.readCssRuleText=function(rule){return cssRuleText(cssRuleRecord(rule));};
  cssRuleMember('CSSRule','cssText',function(){return cssRuleText(cssRuleRecord(this));},function(value){cssRuleRecord(this);if(typeof value==='symbol')throw new TypeError('Cannot convert a Symbol to a string');String(value);});
  cssRuleMember('CSSStyleRule','selectorText',function(){return cssRuleRecord(this,1).selectorText;},function(value){var record=cssRuleRecord(this,1);if(typeof value==='symbol')throw new TypeError('Cannot convert a Symbol to a string');var parsed=parseCssRules(String(value)+' {}',record.parentStyleSheet,record.parentRule);if(parsed.length===1&&cssRuleRecord(parsed[0]).type===1)record.selectorText=cssRuleRecord(parsed[0],1).selectorText;});
  cssRuleMember('CSSStyleRule','style',function(){return cssRuleRecord(this,1).style;},function(value){var record=cssRuleRecord(this,1);if(typeof value==='symbol')throw new TypeError('Cannot convert a Symbol to a string');record.style.cssText=String(value);});
  $c.forEach(['href','media','styleSheet'],function(name){cssRuleMember('CSSImportRule',name,function(){return cssRuleRecord(this,3)[name];});});
  var cssRuleConstants={STYLE_RULE:1,CHARSET_RULE:2,IMPORT_RULE:3,MEDIA_RULE:4,FONT_FACE_RULE:5,PAGE_RULE:6,KEYFRAMES_RULE:7,KEYFRAME_RULE:8,MARGIN_RULE:9,NAMESPACE_RULE:10,COUNTER_STYLE_RULE:11,SUPPORTS_RULE:12,FONT_FEATURE_VALUES_RULE:14};
  $c.forEach($O.keys(cssRuleConstants),function(name){$c.forEach([g.CSSRule,g.CSSRule.prototype],function(owner){$O.defineProperty(owner,name,{value:cssRuleConstants[name],enumerable:true,configurable:true});});});
  function finishCssRule(rule){var record={__proto__:null};$c.forEach(['type','parentRule','parentStyleSheet','cssText','selectorText','href','media','styleSheet','prelude','grouping'],function(name){if($O.hasOwn(rule,name)){record[name]=rule[name];delete rule[name];}});if(rule.style){record.style=rule.style;if(record.type===1)delete rule.style;}if(rule.cssRules)record.cssRules=rule.cssRules;$c.set(cssRuleStates,rule,record);$O.setPrototypeOf(rule,record.type===1?g.CSSStyleRule.prototype:record.type===3?g.CSSImportRule.prototype:g.CSSRule.prototype);return rule;}
  var parseCssRules = function(text, sheet, parent, validated){
    text=String(text);if(!validated&&typeof cssNativeParse==='function')text=cssNativeParse(text);
    var out=cssList(), start=0, depth=0, quote='', escaped=false, comment=false, open=-1;
    for(var i=0;i<text.length;i++){
      var c=text[i],next=i+1<text.length?text[i+1]:undefined;
      if(comment){if(c==='*'&&next==='/'){comment=false;i++;}continue;}
      if(quote){if(escaped)escaped=false;else if(c==='\\')escaped=true;else if(c===quote)quote='';continue;}
      if(c==='/'&&next==='*'){comment=true;i++;continue;} if(c==='"'||c==="'"){quote=c;continue;}
      if(c==='{' ){if(depth++===0)open=i;}
      else if(c==='}'&&depth>0&&--depth===0){
        var head=cssPrelude($c.slice(text,start,open),false),body=$c.slice(text,open+1,i);
        if(head){
          var isAt=head[0]==='@',rule={};
          rule.parentStyleSheet=sheet;rule.parentRule=parent||null;
          if(isAt){rule.prelude=head;rule.type=$c.test(/^@media\b/i,head)?4:$c.test(/^@supports\b/i,head)?12:$c.test(/^@font-face\b/i,head)?5:$c.test(/^@(-webkit-)?keyframes\b/i,head)?7:0;rule.grouping=$c.test(/^@(media|supports|container|scope|starting-style|layer)\b/i,head);rule.cssText=head+($c.trim(body)?' { '+$c.trim(body)+' }':' { }');if(rule.type===5)rule.style=cssDeclaration(body,null,rule,false,true);else if(!$c.test(/^@(property|view-transition)\b/i,head))rule.cssRules=parseCssRules(body,sheet,rule,true);}
          else {rule.type=1;rule.selectorText=cssPrelude(head,true);rule.style=cssDeclaration(body,null,rule);rule.cssText=head+' { '+$c.trim(body)+' }';}
          $c.push(cssItems(out),finishCssRule(rule));
        }
        start=i+1;
      } else if(c===';'&&depth===0){
        var statement=$c.trim($c.slice(text,start,i)),imported=$c.exec(/^@import\s+(?:url\(\s*)?["']([^"']+)["']\s*\)?\s*(.*)$/i,statement);
        if(imported){var rule={};rule.type=3;rule.href=imported[1];rule.media=makeMedia(imported[2]||'');rule.styleSheet=null;rule.parentStyleSheet=sheet;rule.parentRule=parent||null;rule.cssText=statement+';';$c.push(cssItems(out),finishCssRule(rule));}
        else if($c.test(/^@layer\s/i,statement)){var layer={};layer.type=0;layer.parentStyleSheet=sheet;layer.parentRule=parent||null;layer.cssText=statement+';';$c.push(cssItems(out),finishCssRule(layer));}start=i+1;
      }
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
  function appliedSheetRuleRecords(sheet,seen){seen=seen||new Set();if($c.has(seen,sheet))return [];$c.add(seen,sheet);var out=[];for(var $ruleList=cssItems(sheetRecord(sheet).rules),$ruleAt=0,rule;$ruleAt<$ruleList.length&&((rule=$ruleList[$ruleAt]),true);$ruleAt++){var record=cssRuleRecord(rule);if(record.type===3){if(record.styleSheet&&record.media.mediaText)computedVolatile=true;if(record.styleSheet&&(!record.media.mediaText||g.matchMedia(record.media.mediaText).matches))$c.apply(out.push,out,appliedSheetRuleRecords(record.styleSheet,seen));}else $c.push(out,record);}return out;}
  function readableSheet(sheet){var record=sheetRecord(sheet);if(!record.originClean)throw new __ns.DOMException('Cannot access rules of a cross-origin stylesheet','SecurityError');return record;}
  var cssStyleSheetRulesGetter=__ns.makeNativeFunction(function(){return readableSheet(this).rules;},'get cssRules',0);
  $O.defineProperty(sheetConstructor.prototype,'cssRules',{get:cssStyleSheetRulesGetter,enumerable:true,configurable:true});
  cssMember(sheetConstructor.prototype,'rules',function(){return readableSheet(this).rules;});

  function detachCssRule(rule,parent){var record=cssRuleRecord(rule);record.parentStyleSheet=null;record.parentRule=parent||null;if(rule.cssRules)for(var $childList=cssItems(rule.cssRules),$childAt=0;$childAt<$childList.length;$childAt++)detachCssRule($childList[$childAt],rule);}
  function replaceSheetRules(sheet,record,text,keepImports){invalidateComputedStyles();var parsed=cssItems(parseCssRules(cssString(text),sheet,null));if(!keepImports)parsed=$c.filter(parsed,function(rule){return cssRuleRecord(rule).type!==3;});var list=cssItems(record.rules);for(var $ruleAt=0;$ruleAt<list.length;$ruleAt++)detachCssRule(list[$ruleAt]);list.length=0;for(var $parsedAt=0;$parsedAt<parsed.length;$parsedAt++)$c.push(list,parsed[$parsedAt]);}
  function insertSheetRule(sheet,text,index){invalidateComputedStyles();var record=readableSheet(sheet),list=cssItems(record.rules),i=index===undefined?0:Number(index)>>>0;if(record.replacing)throw new __ns.DOMException('The sheet is being replaced','NotAllowedError');if(i>list.length)throw new __ns.DOMException('The rule index is out of range','IndexSizeError');var source=cssNativeParse(cssString(text),false,true),parsed=source===null?cssList():parseCssRules(source,sheet,null,true);if(parsed.length!==1)throw new __ns.DOMException('Expected one CSS rule','SyntaxError');var rule=cssItems(parsed)[0],type=cssRuleRecord(rule).type;if(type===3&&record.constructed)throw new __ns.DOMException('Cannot insert an import into a constructed sheet','SyntaxError');if(type===3&&$c.some($c.slice(list,0,i),function(prior){return cssRuleRecord(prior).type!==3;}))throw new __ns.DOMException('Imports must precede style rules','HierarchyRequestError');if(type!==3&&$c.some($c.slice(list,i),function(next){return cssRuleRecord(next).type===3;}))throw new __ns.DOMException('Style rules must follow imports','HierarchyRequestError');$c.splice(list,i,0,rule);return i;}
  function deleteSheetRule(sheet,index){invalidateComputedStyles();var record=readableSheet(sheet),list=cssItems(record.rules),i=Number(index)>>>0;if(record.replacing)throw new __ns.DOMException('The sheet is being replaced','NotAllowedError');if(i>=list.length)throw new __ns.DOMException('The rule index is out of range','IndexSizeError');detachCssRule(list[i]);$c.splice(list,i,1);}
  cssMethod(sheetConstructor.prototype,'insertRule',1,function(text,index){sheetRecord(this);if(!arguments.length)throw new TypeError('insertRule requires 1 argument');return insertSheetRule(this,text,index);});
  cssMethod(sheetConstructor.prototype,'deleteRule',1,function(index){sheetRecord(this);if(!arguments.length)throw new TypeError('deleteRule requires 1 argument');deleteSheetRule(this,index);});
  cssMethod(sheetConstructor.prototype,'addRule',0,function(selector,style,index){var record=sheetRecord(this);insertSheetRule(this,(selector===undefined?'undefined':cssString(selector))+' {'+(style===undefined?'undefined':cssString(style))+'}',index===undefined?cssItems(record.rules).length:index);return -1;});
  cssMethod(sheetConstructor.prototype,'removeRule',0,function(index){deleteSheetRule(this,index===undefined?0:index);});
  cssMethod(sheetConstructor.prototype,'replaceSync',1,function(text){var record=sheetRecord(this);if(!arguments.length)throw new TypeError('replaceSync requires 1 argument');if(!record.constructed||record.replacing)throw new __ns.DOMException('The sheet cannot be replaced','NotAllowedError');replaceSheetRules(this,record,text,false);});
  cssMethod(sheetConstructor.prototype,'replace',1,function(text){var record;try{record=sheetRecord(this);if(!arguments.length)throw new TypeError('replace requires 1 argument');if(!record.constructed||record.replacing)throw new __ns.DOMException('The sheet cannot be replaced','NotAllowedError');replaceSheetRules(this,record,text,false);return $promiseResolve(this);}catch(error){return $promiseReject(error);}});
  function linkedSheet(el,text,url){var sheet=$O.create(sheetConstructor.prototype),record=newSheetRecord();record.constructed=false;record.ownerNode=el;record.href=url||null;record.title=sheetNativeAttribute(el,'title')||null;record.media=makeMedia(sheetNativeAttribute(el,'media')||'',el);$c.set(sheetRecords,sheet,record);replaceSheetRules(sheet,record,text,true);return sheet;}
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
    if(record.cssRules){output.prelude=cssHostText(record.prelude,budget);output.rules=cssHostRules(record.cssRules,seen,budget);output.cssText=record.prelude+' {';for(var index=0;index<output.rules.length;index++)output.cssText+=' '+output.rules[index].cssText;output.cssText+=' }';}else output.cssText=cssRuleText(record);
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
  function createAdoption(owner){
    var target=[],state={__proto__:null,target:target,proxy:null};
    function length(value){var n=+value;if(n<0||n>4294967295||n!==adoptionFloor(n))throw new RangeError('Invalid array length');if(n<target.length)target.length=n;return true;}
    function index(key,value){adoptionAllowed(owner,value);if(key>target.length)return true;adoptionDefine(target,String(key),{value:value,writable:true,enumerable:true,configurable:true});return true;}
    state.proxy=new adoptionProxy(target,{
      preventExtensions:function(){return false;},
      set:function(target,key,value,receiver){invalidateComputedStyles();var i=cssIndex(key);if(key==='length')return length(value);return i>=0&&i<4294967295?index(i,value):adoptionSet(target,key,value,receiver);},
      deleteProperty:function(target,key){invalidateComputedStyles();var i=cssIndex(key);if(i>=0&&i<target.length){target.length=i;return true;}return adoptionDelete(target,key);},
      defineProperty:function(target,key,descriptor){invalidateComputedStyles();var i=cssIndex(key);if(i>=0&&i<4294967295){if(adoptionOwn(descriptor,'get')||adoptionOwn(descriptor,'set')||adoptionOwn(descriptor,'configurable')&&!descriptor.configurable||adoptionOwn(descriptor,'enumerable')&&!descriptor.enumerable||adoptionOwn(descriptor,'writable')&&!descriptor.writable)return false;return adoptionOwn(descriptor,'value')?index(i,descriptor.value):true;}if(key==='length'){if(adoptionOwn(descriptor,'get')||adoptionOwn(descriptor,'set')||adoptionOwn(descriptor,'writable')&&!descriptor.writable)return false;return adoptionOwn(descriptor,'value')?length(descriptor.value):true;}return adoptionDefine(target,key,descriptor);}
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
          var del = false;
          for (var i = 1; i < parts.length; i++){ var p = parts[i].trim(); var m = /^max-age\s*=\s*(-?\d+)/i.exec(p); if (m && parseInt(m[1], 10) <= 0) del = true; if (/^expires\s*=/i.test(p) && /1970/.test(p)) del = true; }
          if (del) { delete jar[name]; } else { jar[name] = val; }
        }
      }); } catch(e){}
    }
  })();
  // React's isEventSupported('input') checks `'oninput' in document`; make it true (+ keep
  // documentMode ABSENT) so React uses the modern input/change path, not the IE change polyfill —
  // otherwise fireEvent.change never fires onChange.
  d.oninput = null; d.onchange = null; d.onclick = null; d.onkeydown = null; d.onkeyup = null;
  if (!d.head) { try { d.head = d.createElement('head'); if (d.documentElement) d.documentElement.appendChild(d.head); } catch(e){} }
  // NOTE: document.createElementNS uses the native binding (namespace-aware: SVG/MathML elements keep
  // their namespace + case-preserved attributes like viewBox). Previously aliased to createElement.
  // NOTE: document.addEventListener/removeEventListener/dispatchEvent use the native binding (the
  // document is a wrapped node) so document-level listeners — click-outside handlers, global
  // keydown — register and fire via the bubble path. (Previously stubbed to no-ops.)
  // CSSOM shim (<style>.sheet) + form-control value/checked. value/checked are defined BOTH as an
  // OWN accessor on each control element (testing-library's setNativeValue reads
  // getOwnPropertyDescriptor(element)) AND on the interface .prototype (React's value-tracker reads
  // node.constructor.prototype). No setPrototypeOf — avoids touching the native proto chain.
  var currentSheetGetter;
  var sheetTagName=__ns.domAccessor('tagName',false,'Element'),sheetConnected=__ns.domAccessor('isConnected',false,'Node'),sheetTextContent=__ns.domAccessor('textContent',false,'Node'),sheetGetAttribute=d.createElement('style').getAttribute;
  function sheetNativeAttribute(element,name){return cssApply(sheetGetAttribute,element,[name]);}
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
    var validForType = function(el, v){
      var t = $c.toLowerCase($getAttr(el, 'type') || 'text');
      // Range, the date/time family and textarea newlines follow Chromium (input_members.js).
      var sanitized = __ns.sanitizeControlValue ? __ns.sanitizeControlValue(el, t, v) : undefined;
      if (sanitized !== undefined) return sanitized;
      if (t === 'date') return $c.test(/^\d{4}-\d{2}-\d{2}$/, v) ? v : '';
      if (t === 'time') return $c.test(/^\d{2}:\d{2}(:\d{2})?$/, v) ? v : '';
      if (t === 'month') return $c.test(/^\d{4}-\d{2}$/, v) ? v : '';
      if (t === 'datetime-local') return $c.test(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/, v) ? v : '';
      // A number input only retains a valid floating-point number; an invalid/partial value ('.', '-')
      // becomes '' (HTML value-sanitization), which is what lets components see the empty-string clear.
      if (t === 'number') return (v === '' || $c.test(/^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/, v)) ? v : '';
      return v;
    };
    // Live input/textarea values have a dirty flag, separate from their defaults.
    // Hidden/button values remain reflected attributes as required by HTML.
    var controlValues = new WeakMap();
    // The file state's filename value mode (file_inputs.js) registers once, privately.
    var fileControls = null;
    __ns.registerFileControls = function(hooks){ fileControls = hooks; delete __ns.registerFileControls; };
    var fileValueDelete = WeakMap.prototype.delete, fileValueApply = Reflect.apply;
    __ns.resetControlValue = function(element){ fileValueApply(fileValueDelete, controlValues, [element]); };
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
        else $c.set(controlValues,this, validForType(this, v));
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
    var selectionMin=Math.min,selectionMax=Math.max;
    var controlSelections=new WeakMap(),selectionApply=Reflect.apply,selectionGet=WeakMap.prototype.get,selectionSet=WeakMap.prototype.set;
    var selectionString=String,selectionTypeError=TypeError,selectionException=__ns.DOMException;
    function selectionText(value){if(typeof value==='symbol')throw new selectionTypeError('Cannot convert a Symbol value to a string');return selectionString(value);}
    function selectionAvailable(element){var tag=selectionApply(sheetTagName,element,[]);if(tag==='TEXTAREA')return true;var type=sheetNativeAttribute(element,'type')||'text';return $c.test(/^(text|search|url|tel|password)$/,$c.toLowerCase(type));}
    function selectionValue(element){return selectionApply(valDesc.get,element,[]);}
    function selectionState(element){var state=selectionApply(selectionGet,controlSelections,[element]);if(!state){state={start:0,end:0,direction:'none'};selectionApply(selectionSet,controlSelections,[element,state]);}return state;}
    function selectionRequire(element){if(!selectionAvailable(element))throw new selectionException('The input type does not support selection','InvalidStateError');}
    function selectionUpdate(element,start,end,direction){var length=selectionValue(element).length;end=selectionMin(end,length);start=selectionMin(start,end);var state=selectionState(element);state.start=start;state.end=end;state.direction=direction==='forward'||direction==='backward'?direction:'none';}
    function selectionValueChanged(element,oldValue,newValue){if(__ns.controlValueChanged){__ns.controlValueChanged(element,oldValue,newValue);return;}if(oldValue!==newValue&&selectionAvailable(element))selectionUpdate(element,newValue.length,newValue.length,'none');}
    var selStartDesc={configurable:true,get:function(){return selectionAvailable(this)?selectionState(this).start:null;},set:function(value){selectionRequire(this);var state=selectionState(this),start=value===null?0:+value>>>0;selectionUpdate(this,start,selectionMax(start,state.end),state.direction);}};
    var selEndDesc={configurable:true,get:function(){return selectionAvailable(this)?selectionState(this).end:null;},set:function(value){selectionRequire(this);var state=selectionState(this),end=value===null?0:+value>>>0;selectionUpdate(this,selectionMin(state.start,end),end,state.direction);}};
    var selDirDesc={configurable:true,get:function(){return selectionAvailable(this)?selectionState(this).direction:null;},set:function(value){selectionRequire(this);var state=selectionState(this);selectionUpdate(this,state.start,state.end,value===null?'none':selectionText(value));}};
    function setControlSelectionRange(start,end){if(arguments.length<2)throw new selectionTypeError('setSelectionRange requires two arguments');start=+start>>>0;end=+end>>>0;var direction=arguments.length<3||arguments[2]===undefined?'none':selectionText(arguments[2]);selectionRequire(this);selectionUpdate(this,start,end,direction);}
    function setControlRangeText(replacement){
      if(arguments.length===0||arguments.length===2)throw new selectionTypeError('setRangeText requires one or at least three arguments');
      replacement=selectionText(replacement);var state=selectionState(this),start=state.start,end=state.end;
      if(arguments.length>=3){start=+arguments[1]>>>0;end=+arguments[2]>>>0;}
      var mode=arguments.length<4||arguments[3]===undefined?'preserve':selectionText(arguments[3]);
      if(mode!=='select'&&mode!=='start'&&mode!=='end'&&mode!=='preserve')throw new selectionTypeError('Invalid selection mode');
      selectionRequire(this);if(start>end)throw new selectionException('The start exceeds the end','IndexSizeError');
      var value=selectionValue(this),length=value.length;start=selectionMin(start,length);end=selectionMin(end,length);
      var oldStart=state.start,oldEnd=state.end,next=start+replacement.length;
      selectionApply(valDesc.set,this,[value.slice(0,start)+replacement+value.slice(end)]);
      if(mode==='select')selectionUpdate(this,start,next,'none');else if(mode==='start')selectionUpdate(this,start,start,'none');else if(mode==='end')selectionUpdate(this,next,next,'none');else{var delta=replacement.length-(end-start);selectionUpdate(this,oldStart>end?oldStart+delta:oldStart>start?start:oldStart,oldEnd>end?oldEnd+delta:oldEnd>start?next:oldEnd,'none');}
    }
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
    if (!Object.getOwnPropertyDescriptor(baseProto, 'valueAsNumber')) { try { Object.defineProperty(baseProto, 'valueAsNumber', { configurable: true, get: function(){ var v = this.value; if (v == null || v === '') return NaN; var n = Number(v); return isNaN(n) ? NaN : n; }, set: function(n){ this.value = (n == null || isNaN(n)) ? '' : String(n); } }); } catch(e){} }
    // Element.getClientRects() — no layout engine, so mirror getBoundingClientRect's zero rect as a
    // single-entry DOMRectList-ish (with .item). Libs do getClientRects()[0] / .length; [] would NPE.
    if (!baseProto.getClientRects) baseProto.getClientRects = function(){ var r = $N.getBoundingClientRect ? $call($N.getBoundingClientRect, this) : { x:0,y:0,top:0,left:0,right:0,bottom:0,width:0,height:0 }; var list = [r]; list.item = function(i){ return list[i] || null; }; return list; };
    // isEqualNode — React 18 hydration calls it on EVERY node to reconcile the server
    // markup with the client render; without it every comparison throws
    // "e.isEqualNode is not a function", React reports a hydration mismatch (#418) and
    // discards the SSR tree → the whole app falls back to a blank client render. Real
    // spec-shaped deep structural equality (same type/name, same attributes, same
    // character data, recursively-equal children). Recurses via a helper (not
    // child.isEqualNode) so it works even on node types that miss the method.
    if (!baseProto.isEqualNode) {
      var __nodeEq = function(a, b){
        if (a === b) return true;
        if (!a || !b) return false;
        var type = $call($N.nodeType, a);
        if (type !== $call($N.nodeType, b) || $call($N.nodeName, a) !== $call($N.nodeName, b)) return false;
        if (type === 1) {
          var aa = $call($N.nativeAttributes, a), ba = $call($N.nativeAttributes, b);
          if (aa.length !== ba.length) return false;
          for (var i = 0; i < aa.length; i++) {
            if ($call($N.getAttribute, b, $call($N['attr.name'], aa[i])) !== $call($N['attr.value'], aa[i])) return false;
          }
        } else if (($call($N.nodeValue, a) || '') !== ($call($N.nodeValue, b) || '')) {
          return false;
        }
        var ac = $call($N.firstChild, a), bc = $call($N.firstChild, b);
        for (; ac !== null && bc !== null; ac = $call($N.nextSibling, ac), bc = $call($N.nextSibling, bc)) { if (!__nodeEq(ac, bc)) return false; }
        return ac === bc;
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
    ['HTMLInputElement','HTMLTextAreaElement','HTMLSelectElement','HTMLOptionElement','HTMLButtonElement','HTMLLabelElement','HTMLCanvasElement'].forEach(function(n){ if (typeof g[n] !== 'function') g[n] = function(){}; var p = Object.create(baseProto); try {
      var defineControl=function(name,descriptor){Object.defineProperty(p,name,Object.assign({},descriptor,{enumerable:true}));};
      if(n!=='HTMLLabelElement')defineControl('value',valDesc);if(n==='HTMLInputElement'){defineControl('checked',checkedDesc);defineControl('defaultChecked',defaultCheckedDesc);}defineControl('form',formDesc);
      if (defType[n]) defineControl('type',mkTypeDesc(defType[n]));
      // Reflected IDL string attributes: a library setting `input.name = 'x'` (e.g. react-number-
      // format, MUI) must show up as the `name` content attribute (toHaveAttribute, [name] selectors).
      // Each interface gets only the members Chrome 156 exposes on it (control_prototypes_probe.js).
      (controlStrings[n]||[]).forEach(function(a){ Object.defineProperty(p, a, { configurable: true, enumerable:true, get: function(){ return $getAttr(this, a) || ''; }, set: function(v){ $setAttr(this, a, v == null ? '' : String(v)); } }); });
      // Reflected BOOLEAN IDL attributes (presence): el.disabled = true -> the `disabled` content
      // attribute; reading returns hasAttribute. Tests read el.disabled / [disabled] / multiple.
      (controlBooleans[n]||[]).forEach(function(pair){ Object.defineProperty(p, pair[0], { configurable: true, enumerable:true, get: function(){ return $hasAttr(this, pair[1]); }, set: function(v){ if (v) $setAttr(this, pair[1], ''); else $removeAttr(this, pair[1]); } }); });
      if (n === 'HTMLInputElement' || n === 'HTMLTextAreaElement') {
        defineControl('defaultValue',defaultValueDesc);
        defineControl('selectionStart',selStartDesc);defineControl('selectionEnd',selEndDesc);defineControl('selectionDirection',selDirDesc);
        p.setSelectionRange=setControlSelectionRange;
        p.select=function(){if(selectionAvailable(this))selectionUpdate(this,0,selectionValue(this).length,'none');};
        p.setRangeText=setControlRangeText;
      }
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
      var selectionOf = function(sel){
        var os = optionsOf(sel), chosen = [];
        for (var i=0;i<os.length;i++) if (isSelected(os[i])) $c.push(chosen, os[i]);
        var multiple = $hasAttr(sel, 'multiple');
        if (!multiple && chosen.length > 1) chosen = [chosen[chosen.length-1]];
        if (!chosen.length && !multiple && selectNumber($getAttr(sel, 'size') || 0) <= 1 && !$c.has(emptySelects,sel)) {
          for (var j=0;j<os.length;j++) {
            var group = $apply(optionParent, os[j], []);
            if (!$hasAttr(os[j], 'disabled') && !(group && $apply(optionKind, group, []) === 1 && optionName(group) === 'optgroup' && $hasAttr(group, 'disabled'))) { $c.push(chosen, os[j]); break; }
          }
        }
        return chosen;
      };
      Object.defineProperty(op, 'selected', { configurable: true, get: function(){
        var sel = ownerSelect(this); return sel ? $c.indexOf(selectionOf(sel), this) >= 0 : isSelected(this);
      }, set: function(v){
        v = !!v; $c.set(optionStates,this, v);
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
        set: function(idx){ var o = opts(this), index = Number(idx) >> 0;
          $c.add(selection.emptySelects,this);
          for (var i=0;i<o.length;i++) { $c.set(selection.optionStates,o[i], i === index); }
          if (index >= 0 && index < o.length) $c.delete(selection.emptySelects,this);
        } });
      Object.defineProperty(sp, 'value', { configurable: true,
        get: function(){ var chosen = selection.selectionOf(this); return chosen.length ? chosen[0].value : ''; },
        set: function(v){ var o = opts(this), s = String(v), hit = false;
          $c.add(selection.emptySelects,this);
          for (var i=0;i<o.length;i++){ var m = !hit && o[i].value === s; $c.set(selection.optionStates,o[i], m); if (m) hit = true; }
          if (hit) $c.delete(selection.emptySelects,this);
        } });
    })();
    var formValueApply=Reflect.apply,formValueGet=WeakMap.prototype.get,formValueHas=WeakMap.prototype.has;
    var formValueText=__ns.domAccessor('textContent',false,'Node');
    __ns.formControlValue=function(element){
      var tag=formValueApply(sheetTagName,element,[]),type=formValueApply(sheetGetAttribute,element,['type'])||'text';
      var file=fileControls?fileControls.value(element):undefined;if(file!==undefined)return file;
      if(tag==='SELECT'){var chosen=selection.selectionOf(element);return chosen.length?chosen[0].value:'';}
      if(formValueApply(formValueHas,controlValues,[element])&&!$c.test(/^(hidden|button|submit|reset|image|checkbox|radio)$/,$c.toLowerCase(type)))return formValueApply(formValueGet,controlValues,[element]);
      return tag==='TEXTAREA'?formValueApply(formValueText,element,[]):formValueApply(sheetGetAttribute,element,['value'])||'';
    };
    var resetControlDelete=WeakMap.prototype.delete,resetSelectDelete=WeakSet.prototype.delete;
    __ns.formSelectedOptions=function(element){return selection.selectionOf(element);};
    __ns.formResetControl=function(element){
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
    __ns.cloneControlState = function cloneControlState(source, clone, deep){
      var pending=[[source,clone]];
      for(var index=0;index<pending.length;index++){
        var pair=pending[index];source=pair[0];clone=pair[1];
        if ($c.has(controlValues,source)) clone.value = $c.get(controlValues,source);
        if(cloneApply(cloneNodeType,source,[])===1&&formValueApply(sheetTagName,source,[])==='INPUT'){if(checkedNative(3,source))checkedNative(1,clone,checkedNative(0,source));checkedNative(4,clone);if(fileControls)fileControls.clone(source,clone);}
        if ($c.has(selection.optionStates,source)) $c.set(selection.optionStates,clone, $c.get(selection.optionStates,source));
        if ($c.has(selection.emptySelects,source)) $c.add(selection.emptySelects,clone);
        if(deep){
          var originals=cloneApply(cloneChildren,source,[]),copies=cloneApply(cloneChildren,clone,[]);
          for(var i=0;i<originals.length&&i<copies.length;i++)cloneDefine(pending,pending.length,{__proto__:null,value:[originals[i],copies[i]],writable:true,configurable:true,enumerable:true});
        }
      }
    };
    // <label>: htmlFor + control resolution. testing-library's getByLabelText filters
    // querySelectorAll('label') by `label.control === element`, so without `control` NO label ever
    // associates and every form field fails with "no form control was found associated to that label".
    (function(){
      var lp = protoFor.HTMLLabelElement;
      var labelable = function(el){ if (!el || el.nodeType !== 1) return false; var tg = String(el.tagName).toUpperCase(); if (tg === 'INPUT') return (el.getAttribute('type') || 'text').toLowerCase() !== 'hidden'; return tg === 'BUTTON' || tg === 'SELECT' || tg === 'TEXTAREA' || tg === 'METER' || tg === 'OUTPUT' || tg === 'PROGRESS'; };
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
    var applyControlProto = function(el){
      try {
        var t = $call($lower, String($call(controlTagName, el) || ''));
        if (CTRL[t]) {
          Object.setPrototypeOf(el, protoFor[CTRL[t]]);
          // value uses a spec internal dirty-value slot (valDesc, distinct from the content attribute);
          // attach the DOM value-change tracker so framework change-detection works (per-char + dedup).
          if (t === 'input' || t === 'textarea') { attachValueTracker(el, 'value', valDesc); Object.defineProperty(el, 'checked', checkedDesc); }
          // drop the own native click on input/textarea/button so it resolves to the patchable proto
          // click (tests spy on HTMLInputElement.prototype.click for file pickers etc.).
          if ((t === 'input' || t === 'textarea' || t === 'button') && Object.prototype.hasOwnProperty.call(el, 'click')) delete el.click;
        } else if (t === 'a') {
          Object.setPrototypeOf(el, anchorProto);
          if (Object.prototype.hasOwnProperty.call(el, 'click')) delete el.click; // resolve to the patchable proto click
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
            if (paints && value !== null && typeof value === 'object' && __cvPaints.has(value)) { state[name] = value; return; }
            var color = __cvColor(canvas, '' + value);
            if (color != null) state[name] = color;
          } });
      });
    };
    var mkCanvasCtx = function(canvas){
      var noop = function(){};
      var paint = function(){ var p = { addColorStop: noop }; __cvPaints.add(p); return p; };
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
        createPattern: function(){ var p = {}; __cvPaints.add(p); return p; },
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
        measureText: function(s){ this._ops.push(['measureText', String(s||''), this.font]); var wdt = (String(s||'').length)*6; return { width: wdt, actualBoundingBoxAscent: 8, actualBoundingBoxDescent: 2, actualBoundingBoxLeft: 0, actualBoundingBoxRight: wdt, fontBoundingBoxAscent: 8, fontBoundingBoxDescent: 2 }; },
        isPointInPath: function(){ return false; }, isPointInStroke: function(){ return false; },
        lineWidth: 1, lineCap: 'butt', lineJoin: 'miter',
        miterLimit: 10, lineDashOffset: 0, shadowBlur: 0, shadowOffsetX: 0, shadowOffsetY: 0,
        font: '10px sans-serif', textAlign: 'start', textBaseline: 'alphabetic', direction: 'inherit',
        globalAlpha: 1, globalCompositeOperation: 'source-over', imageSmoothingEnabled: true, imageSmoothingQuality: 'low'
      };
      __cvStyles(ctx, canvas);
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
        'shadowOffsetY','font','textAlign','textBaseline','direction','imageSmoothingEnabled','imageSmoothingQuality'];
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
    var mkWebGLCtx = function(canvas, isGL2){
      var C = GLC, noop = function(){};
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
      var EXT = isGL2 ? GL_EXT2 : GL_EXT1;
      var gl = { canvas: canvas, _ops: [],
        drawingBufferWidth: (canvas && canvas.width) || 300, drawingBufferHeight: (canvas && canvas.height) || 150,
        drawingBufferColorSpace: 'srgb', unpackColorSpace: 'srgb' };
      for (var k in C) gl[k] = C[k]; // enum constants readable off the context (gl.VERSION, …)
      gl.getParameter = function(p){ return (p in params) ? params[p] : null; };
      gl.getContextAttributes = function(){ return { alpha:true, antialias:true, depth:true, desynchronized:false,
        failIfMajorPerformanceCaveat:false, powerPreference:'default', premultipliedAlpha:true,
        preserveDrawingBuffer:false, stencil:false, xrCompatible:false }; };
      gl.getSupportedExtensions = function(){ return EXT.slice(); };
      gl.getExtension = function(name){
        if (EXT.indexOf(name) < 0) return null;
        if (name === 'WEBGL_debug_renderer_info') return { UNMASKED_VENDOR_WEBGL: C.UNMASKED_VENDOR_WEBGL, UNMASKED_RENDERER_WEBGL: C.UNMASKED_RENDERER_WEBGL };
        if (name === 'EXT_texture_filter_anisotropic') return { MAX_TEXTURE_MAX_ANISOTROPY_EXT: C.MAX_TEXTURE_MAX_ANISOTROPY_EXT, TEXTURE_MAX_ANISOTROPY_EXT: C.TEXTURE_MAX_ANISOTROPY_EXT };
        if (name === 'OES_vertex_array_object') return { createVertexArrayOES:function(){return{};}, deleteVertexArrayOES:noop, bindVertexArrayOES:noop, isVertexArrayOES:function(){return false;}, VERTEX_ARRAY_BINDING_OES:C.VERTEX_ARRAY_BINDING_OES };
        if (name === 'WEBGL_lose_context') return { loseContext:noop, restoreContext:noop };
        if (name === 'ANGLE_instanced_arrays') return { drawArraysInstancedANGLE:noop, drawElementsInstancedANGLE:noop, vertexAttribDivisorANGLE:noop, VERTEX_ATTRIB_ARRAY_DIVISOR_ANGLE:0x88FE };
        return {}; // non-null stub so `if (gl.getExtension(x))` feature-detects pass
      };
      gl.getShaderPrecisionFormat = function(shaderType, precisionType){
        var isFloat = (precisionType === C.HIGH_FLOAT || precisionType === C.MEDIUM_FLOAT || precisionType === C.LOW_FLOAT);
        // SwiftShader maps all float precisions to IEEE highp (127/127/23); ints 31/30/0.
        return isFloat ? { rangeMin:127, rangeMax:127, precision:23 } : { rangeMin:31, rangeMax:30, precision:0 };
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
      gl.readPixels = function(x,y,w,h,fmt,type,pixels){
        if (pixels && pixels.length){
          var src = __cvBytes(__cvSeed(canvas, gl, 'readPixels:'+x+','+y+','+w+','+h+','+fmt+','+type), Math.min(pixels.length, 4096));
          for (var i = 0; i < pixels.length; i++) pixels[i] = src[i % src.length];
        }
      };
      try { var P = isGL2 ? g.WebGL2RenderingContext : g.WebGLRenderingContext; if (P && P.prototype) Object.setPrototypeOf(gl, P.prototype); } catch(e){}
      return gl;
    };
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
    __ns.styleSheetList=function(refresh){return cssList('StyleSheetList',refresh);};
    try { Object.defineProperty(d, 'styleSheets', { configurable: true, get: function(){return documentSheets;} }); } catch(e){}

    // Alias the shim interface-constructor `.prototype`s onto the REAL element/document prototypes,
    // so a test patching HTMLElement.prototype.requestFullscreen / Document.prototype.exitFullscreen
    // (DataTableCore fullscreen) reaches our instances. Only affects methods that aren't own per
    // instance (own native methods still shadow). The genuine fullscreen API is installed by
    // user_activation_gates.js.
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
      currentSheetGetter=function(){
          var tag = cssApply(sheetTagName,this,[]);
          if (!tag) return null;
          tag=cssTagUpper(tag);if(tag!=='STYLE'&&tag!=='LINK')return null;
          var cached=elementSheets.get(this);
          if (!cssApply(sheetConnected,this,[])) {if(cached)detachSheet(cached.sheet);elementSheets.delete(this);return null;}
          if(tag==='LINK')return cached?cached.sheet:null;
          if(!cssApply(inlineSheetAllowed,this,[])){if(cached)detachSheet(cached.sheet);elementSheets.set(this,{sheet:null,text:null});return null;}
          var text=cssApply(sheetTextContent,this,[])||'';
          if (!cached || !cached.sheet || cached.text!==text) {
            if(cached)detachSheet(cached.sheet);
            cached={sheet:mkSheet(this),text:text};elementSheets.set(this,cached);
          }
          return cached.sheet;
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
      // NB: define these directly (NOT guarded by Object.getOwnPropertyDescriptor) — the all-Rust
      // DOM's NON_MASKING name interceptor intercepts every absent property (returns undefined), so
      // getOwnPropertyDescriptor synthesizes a truthy descriptor and a `if (!descriptor)` guard would
      // wrongly skip the define. A real own accessor still shadows the interceptor once defined.
      try { Object.defineProperty(d, 'fullscreenElement', { configurable: true, writable: true, value: null }); } catch(e){}
      // Document lifecycle/visibility — our document is always fully parsed and "foregrounded".
      // Components gate hydration / effects on these (readyState === 'complete', visibilityState).
      try { Object.defineProperty(d, 'readyState', { configurable: true, get: function(){ return 'complete'; } }); } catch(e){}
      try { Object.defineProperty(d, 'visibilityState', { configurable: true, get: function(){ return 'visible'; } }); } catch(e){}
      try { Object.defineProperty(d, 'hidden', { configurable: true, get: function(){ return false; } }); } catch(e){}
      // No hit-testing without layout → elementFromPoint(s) resolve to nothing.
      d.elementFromPoint = function(){ return null; };
      d.elementsFromPoint = function(){ return []; };
    } catch(e){}
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
  // Compare boundary points (ca,oa) vs (cb,ob) in document order: -1 before, 0 equal, 1 after.
  function __cmp(ca,oa,cb,ob){
    if(ca===cb) return oa<ob?-1:oa>ob?1:0;
    var A=__ancestry(ca), B=__ancestry(cb);
    // walk B looking for a node whose ancestry includes it → common ancestor
    for(var i=0;i<A.length;i++){ var j=B.indexOf(A[i]); if(j>=0){
      var anc=A[i];
      // child of anc on A's side (or ca itself if A[i-1])
      var childA=i>0?A[i-1]:null, childB=j>0?B[j-1]:null;
      if(childA===null) return oa<=__nodeIndex(childB)?-1:1;     // ca IS the common ancestor
      if(childB===null) return __nodeIndex(childA)<ob?-1:1;       // cb IS the common ancestor
      var ia=__nodeIndex(childA), ib=__nodeIndex(childB);
      return ia<ib?-1:ia>ib?1:0;
    }}
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
  Range.prototype.deleteContents=function(){ if(this.collapsed) return; var sc=this.startContainer; if(sc===this.endContainer && sc.nodeType===3){ var t=sc.data||''; if(sc.deleteData) sc.deleteData(this.startOffset, this.endOffset-this.startOffset); else sc.data=t.slice(0,this.startOffset)+t.slice(this.endOffset); this.endOffset=this.startOffset; this.endContainer=sc; return; } var ca=this.commonAncestorContainer; var kids=ca.childNodes?Array.prototype.slice.call(ca.childNodes):[]; for(var i=0;i<kids.length;i++){ var k=kids[i]; if(this.comparePoint(k,0)===0 && this.comparePoint(k, (k.nodeType===3?(k.data||'').length:(k.childNodes?k.childNodes.length:0)))===0 && k.parentNode){ k.parentNode.removeChild(k); } } this.collapse(true); };
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
      // Detached container elements for the child's documentElement/head/body. Plain <div>s,
      // NOT <html>/<head>/<body>: the native binding treats those three as document
      // singletons (createElement('head') attaches to the live documentElement), which would
      // leak the child realm's nodes into the PARENT's serialized tree. Divs stay detached.
      var root = d.createElement('div');
      var head = d.createElement('div');
      var body = d.createElement('div');
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
      var q1 = function(s){
        var r = walk(root, predFor(s));
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
        getElementById: function(id){ return walk(root, function(n){ return attr(n,'id') === String(id) || n.id === String(id); }); },
        querySelector: q1,
        querySelectorAll: function(s){ return walkAll(root, predFor(s), []); },
        getElementsByTagName: function(s){ return walkAll(root, predFor(String(s)), []); },
        addEventListener: function(t,f){ if (typeof f==='function') (dl[t]=dl[t]||[]).push(f); },
        removeEventListener: function(t,f){ var a=dl[t]; if(a){ var i=a.indexOf(f); if(i>=0) a.splice(i,1); } },
        dispatchEvent: function(ev){ if(!ev) return true; var a=dl[ev.type]; if(a) a.slice().forEach(function(f){ try{f.call(cd,ev);}catch(e){} }); return !(ev&&ev.defaultPrevented); },
      };
      return cd;
    };
    // origin (scheme://host[:port]) of a URL string; 'null' when not absolute http(s).
    var originOf = function(u){ try { var m=/^(https?:\/\/[^\/]+)/i.exec(String(u||'')); return m?m[1]:'null'; } catch(e){ return 'null'; } };

    // Build (once) a bridged child realm for the <iframe> `hostEl`. `parentWin` defaults
    // to the top window `g`. Returns the child window; also sets hostEl.contentWindow /
    // contentDocument. Reused by turbo-surf's render tier for the reCAPTCHA bframe.
    __ns.makeFrameRealm = function(hostEl, parentWin, createWindow){
      parentWin = parentWin || g;
      if (hostEl && hostEl.__realm) return hostEl.__realm;
      var childDoc = makeChildDocument();
      var srcAttr = hostEl && (hostEl.src || (hostEl.getAttribute && hostEl.getAttribute('src')));
      var childOrigin = originOf(srcAttr);
      var parentOrigin = (parentWin.location && parentWin.location.origin) || originOf(parentWin.location && parentWin.location.href) || '';
      var wl = {}; // the child window's own event listeners
      var childWin, parentView;
      childWin = typeof createWindow === 'function' ? createWindow() : {};
      Object.assign(childWin, {
        document: childDoc, closed: false, name: (hostEl && hostEl.name) || '',
        origin: childOrigin, length: 0, frames: [], devicePixelRatio: 1,
        innerWidth: 0, innerHeight: 0, outerWidth: 0, outerHeight: 0, screenX: 0, screenY: 0,
        location: { href: String(srcAttr || 'about:blank'), origin: childOrigin,
          protocol: (childOrigin.split(':')[0] || 'about') + ':',
          host: childOrigin.replace(/^https?:\/\//,''),
          hostname: childOrigin.replace(/^https?:\/\//,'').split(':')[0],
          port: '', pathname: '/', search: '', hash: '',
          assign:function(){}, replace:function(){}, reload:function(){}, toString:function(){ return this.href; } },
        navigator: g.navigator, screen: g.screen, history: g.history,
        onmessage: null, onerror: null, onload: null,
        setTimeout: g.setTimeout, clearTimeout: g.clearTimeout,
        setInterval: g.setInterval, clearInterval: g.clearInterval,
        requestAnimationFrame: g.requestAnimationFrame, cancelAnimationFrame: g.cancelAnimationFrame,
        getComputedStyle: g.getComputedStyle, matchMedia: g.matchMedia,
        atob: g.atob, btoa: g.btoa, crypto: g.crypto, performance: g.performance,
        addEventListener: function(t,f){ if(typeof f==='function') (wl[t]=wl[t]||[]).push(f); },
        removeEventListener: function(t,f){ var a=wl[t]; if(a){ var i=a.indexOf(f); if(i>=0) a.splice(i,1); } },
        dispatchEvent: function(ev){var state=beginEventDispatch(ev,childWin,[childWin]),a=wl[state.type];try{if(a)a.slice().forEach(function(f){try{f.call(childWin,ev);}catch(e){}});}finally{endEventDispatch(state);}return !state.defaultPrevented;},
        focus:function(){}, blur:function(){}, close:function(){ childWin.closed = true; },
        // parent → child: fire the CHILD's listeners; source is the parent view.
        postMessage: function(msg, targetOrigin, transfer){
          var ports = Array.isArray(transfer)?transfer:(transfer&&transfer.length?Array.prototype.slice.call(transfer):[]);
          deliver(childWin, mkMsgEvent(msg, parentOrigin, parentView, ports));
        },
      });
      childWin.window = childWin; childWin.self = childWin; childWin.globalThis = childWin;
      childDoc.defaultView = childWin;
      // parentView: what the child holds to reach the parent (child.parent / child.top /
      // the e.source it replies through). child → parent: fire the PARENT's listeners;
      // source is the iframe's own contentWindow so the parent can match + reply.
      parentView = {
        postMessage: function(msg, targetOrigin, transfer){
          var ports = Array.isArray(transfer)?transfer:(transfer&&transfer.length?Array.prototype.slice.call(transfer):[]);
          deliver(parentWin, mkMsgEvent(msg, childOrigin, childWin, ports));
        },
        location: { origin: parentOrigin, href: (parentWin.location && parentWin.location.href) || '' },
      };
      childWin.parent = parentView; childWin.top = parentView; childWin.frameElement = hostEl || null;
      if (hostEl) { hostEl.__realm = childWin; try { hostEl.contentWindow = childWin; hostEl.contentDocument = childDoc; } catch(e){} }
      return childWin;
    };

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
    var eventTargets=__ns.eventTargetInstances||(__ns.eventTargetInstances=new WeakSet());
    var eventCallbacks=__ns.eventTargetCallbacks||(__ns.eventTargetCallbacks={});
    var windowCallbacks=__ns.windowEventCallbacks||(__ns.windowEventCallbacks={});
    ['addEventListener','removeEventListener','dispatchEvent'].forEach(function(name){
      if(!eventCallbacks[name])eventCallbacks[name]=eventTargetProto[name];
      if(!windowCallbacks[name])windowCallbacks[name]=g[name];
    });
    var PreviousEventTarget=g.EventTarget;
    var eventTargetCtor=__ns.makeNativeFunction(class extends PreviousEventTarget{constructor(){super();$c.add(eventTargets,this);}},'EventTarget',0,true,true);
    eventTargetCtor.prototype=eventTargetProto;
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
    if(typeof g.Audio==='function'){
      var audioConstructor=__ns.makeNativeFunction(function(src){var element=d.createElement('audio');if(src!==undefined)element.src=src;return element;},'Audio',0,true,false,false);
      audioConstructor.prototype=prototypes.HTMLAudioElement;Object.defineProperty(audioConstructor,'prototype',{writable:false});
      Object.defineProperty(g,'Audio',{value:audioConstructor,writable:true,configurable:true});
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
    Object.defineProperty(elementProto,'attributes',{get:__ns.makeNativeFunction(function(){getTag.call(this);var owner=this,map=$call($wmGet,mapsByElement,owner);if(map)return map;map=namedFactory(namedMapProto,function(operation,key,value){var entries=nativeAttributes.call(owner),index=/^(0|[1-9]\d*)$/.test(key)?Number(key):null,attribute=index===null?Array.from(entries).find(function(entry){return entry.name===key;}):entries[index];if(operation==='indices')return Array.from(entries,function(_,index){return index;});if(operation==='keys')return Array.from(new Set(Array.from(entries,function(entry){return entry.name;})));if(operation==='get')return attribute;if(operation==='has')return !!attribute;if(operation==='descriptor')return attribute?{value:attribute,writable:false,enumerable:index!==null,configurable:true}:undefined;if(operation==='set'||operation==='define')return attribute?true:undefined;if(operation==='delete')return attribute?false:undefined;},true);$c.set(mapRecords,map,owner);if(collectionSet)collectionSet(map,{kind:'NamedNodeMap',owner:owner});$c.set(mapsByElement,owner,map);return map;},'get attributes',0),enumerable:true,configurable:true});
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
    // HTMLOrSVGElement (turbo-dom#87): focus() / blur() brand-check their interface. An SVG or
    // MathML element takes script focus when connected, rendered and given a valid tabindex (or,
    // in SVG, when it is an <a> with an href / xlink:href).
    var nativeFocus=htmlProto.focus,nativeBlur=htmlProto.blur,nativeClick=htmlProto.click;
    // Internal focusing steps (dialogs, focusgroup navigation) focus any element kind.
    __ns.focusElement=nativeFocus;
    var parsesTabIndex=function(element){var value=element.getAttribute('tabindex');return value!==null&&/^[\t\n\f\r ]*[+-]?[0-9]/.test(value);};
    // Rendering, as Chrome 156 measures it: an SVG element other than <svg> needs a non-HTML
    // parent, a non-SVG element under SVG needs a <foreignObject> parent, and no inclusive
    // ancestor may compute display: none.
    var foreignRendered=function(element){
      for(var node=element;node&&node.nodeType===1;node=node.parentNode){
        var parent=node.parentNode,parentSpace=parent&&parent.nodeType===1?parent.namespaceURI:null;
        if(node.namespaceURI==='http://www.w3.org/2000/svg'&&node.localName!=='svg'&&(parentSpace===null||parentSpace==='http://www.w3.org/1999/xhtml'))return false;
        if(node.namespaceURI!=='http://www.w3.org/2000/svg'&&parentSpace==='http://www.w3.org/2000/svg'&&parent.localName!=='foreignObject')return false;
        if(g.getComputedStyle(node).display==='none')return false;
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
    (function(){
      var member=function(proto,name,getter){for(var o=proto;o;o=Object.getPrototypeOf(o)){var d=$gopd(o,name);if(d)return getter?d.get:d.value;}};
      var getters=[[g.Node.prototype,['nodeType','nodeName','nodeValue','parentNode','parentElement','firstChild','nextSibling','childNodes','ownerDocument']],[g.Document.prototype,['documentElement']],[g.Attr.prototype,['ownerElement','prefix','namespaceURI','value','localName','name']],[elementProto,['namespaceURI','prefix','attributes']]];
      for(var i=0;i<getters.length;i++)for(var k=0;k<getters[i][1].length;k++){var name=getters[i][1][k],key=(getters[i][0]===g.Attr.prototype?'attr.':getters[i][0]===elementProto?'element.':'')+name;$N[key]=member(getters[i][0],name,true);}
      var methods=[[g.Document.prototype,['createTextNode']],[g.Node.prototype,['getRootNode','contains','insertBefore','appendChild']],[elementProto,['getAttribute','getAttributeNode','getAttributeNodeNS','setAttributeNode','setAttributeNodeNS','removeAttributeNode','getBoundingClientRect']]];
      for(i=0;i<methods.length;i++)for(k=0;k<methods[i][1].length;k++)$N[methods[i][1][k]]=member(methods[i][0],methods[i][1][k],false);
    })();
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
        ['get','set'].forEach(function(kind){var delegate=descriptor[kind];if(!delegate)return;copy[kind]=__ns.makeNativeFunction(function(){reflectedApply(check,this,[]);if(!reflectedApply(reflectedSetHas,ownerLocals,[reflectedApply(getLocalTag,this,[])]))throw new TypeError('Illegal invocation');var value=reflectedApply(delegate,this,arguments);if(kind==='get'&&(name==='src'||name==='href')){if(owner==='HTMLBaseElement'){if(value&&typeof g.URL==='function'){try{return new g.URL(value,this.ownerDocument.URL||g.location.href).href;}catch(e){}}return value;}var raw=reflectedApply(reflectedGetAttribute,this,[name]);return raw===null?'':__ns.resolveDocumentURL(this,raw);}return value;},kind+' '+name,kind==='get'?0:1);});
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
    // The muted IDL attribute is media state, not a reflection: it starts false.
    if(prototypes.HTMLMediaElement&&!Object.hasOwn(prototypes.HTMLMediaElement,'muted')){
      var mediaMuted=new WeakMap(),mutedGet=WeakMap.prototype.get,mutedSet=WeakMap.prototype.set,mediaLocals=new Set(['audio','video']),mediaCheck=__ns.domAccessor('nodeType',false,'HTMLElement');
      var mediaBrand=function(receiver){reflectedApply(mediaCheck,receiver,[]);if(!reflectedApply(reflectedSetHas,mediaLocals,[reflectedApply(getLocalTag,receiver,[])]))throw new TypeError('Illegal invocation');};
      Object.defineProperty(prototypes.HTMLMediaElement,'muted',{configurable:true,enumerable:true,
        get:__ns.makeNativeFunction(function(){mediaBrand(this);return reflectedApply(mutedGet,mediaMuted,[this])===true;},'get muted',0),
        set:__ns.makeNativeFunction(function(value){mediaBrand(this);reflectedApply(mutedSet,mediaMuted,[this,!!value]);},'set muted',1)});
    }
    // Text.wholeText (turbo-dom#143): the data of the contiguous Text siblings, in tree order.
    if(g.Text&&!Object.hasOwn(g.Text.prototype,'wholeText')){
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
    [['HTMLElement',htmlProto],['SVGElement',prototypes.SVGElement],['MathMLElement',prototypes.MathMLElement]].forEach(function(owner){var checkHandlerReceiver=__ns.domAccessor('nodeType',false,owner[0]);
    handlerNames.forEach(function(type){
      var name='on'+type;
      Object.defineProperty(owner[1],name,{configurable:true,enumerable:true,
        get:__ns.makeNativeFunction(function(){$call(checkHandlerReceiver,this);if(prepareInlineHandler)$call(prepareInlineHandler,this,name);var handlers=$call($wmGet,eventHandlers,this);return handlers&&handlers[name]&&handlers[name].value||null;},'get '+name,0),
        set:__ns.makeNativeFunction(function(value){
          $call(checkHandlerReceiver,this);if(noteInlineHandler)$call(noteInlineHandler,this,name);value=typeof value==='function'?value:null;
          var handlers=$call($wmGet,eventHandlers,this);if(!handlers){handlers=$create(null);$call($wmSet,eventHandlers,this,handlers);}
          var entry=handlers[name];
          if(!entry){entry={value:null,callback:__ns.makeNativeFunction(function(event){var current=$call($wmGet,eventHandlers,this)[name].value;if(current&&$call(current,this,event)===false)event.preventDefault();},name,1)};handlers[name]=entry;}
          if(value&&!entry.value)$call(__ns.handlerAdd||handlerAdd,this,type,entry.callback);
          if(!value&&entry.value)$call(__ns.handlerRemove||handlerRemove,this,type,entry.callback);
          entry.value=value;
        },'set '+name,1)});
    });
    });
    // The lookup start of locate-a-namespace / locate-a-namespace-prefix, through captured natives.
    function nodeLookupStart(node){var type=$call(getType,node);return type===9?$call($N.documentElement,node):type===2?$call($N['attr.ownerElement'],node):type===1?node:$call($N.parentElement,node);}
    var nodeOperations={
      lookupNamespaceURI(prefix){prefix=prefix==null||prefix===''?null:attributeString(prefix);var element=nodeLookupStart(this);while(element){if(prefix==='xml')return 'http://www.w3.org/XML/1998/namespace';if(prefix==='xmlns')return 'http://www.w3.org/2000/xmlns/';var elementNamespace=$call($N['element.namespaceURI'],element);if(elementNamespace&&$call($N['element.prefix'],element)===prefix)return elementNamespace;var declaration=$call($N.getAttributeNodeNS,element,'http://www.w3.org/2000/xmlns/',prefix===null?'xmlns':prefix);if(declaration&&(prefix===null?$call($N['attr.prefix'],declaration)===null:$call($N['attr.prefix'],declaration)==='xmlns'))return $call($N['attr.value'],declaration)||null;element=$call($N.parentElement,element);}return null;},
      lookupPrefix(namespace){namespace=namespace==null||namespace===''?null:attributeString(namespace);if(namespace===null)return null;var element=nodeLookupStart(this);while(element){var elementPrefix=$call($N['element.prefix'],element);if(elementPrefix&&$call($N['element.namespaceURI'],element)===namespace&&$call(nodeOperations.lookupNamespaceURI,this,elementPrefix)===namespace)return elementPrefix;var attributes=$call($N.nativeAttributes,element);for(var index=0;index<attributes.length;index++){var attribute=attributes[index];if($call($N['attr.namespaceURI'],attribute)==='http://www.w3.org/2000/xmlns/'&&$call($N['attr.prefix'],attribute)==='xmlns'&&$call($N['attr.value'],attribute)===namespace&&$call(nodeOperations.lookupNamespaceURI,this,$call($N['attr.localName'],attribute))===namespace)return $call($N['attr.localName'],attribute);}element=$call($N.parentElement,element);}return null;},
      isDefaultNamespace(namespace){namespace=namespace==null||namespace===''?null:String(namespace);return $call(nodeOperations.lookupNamespaceURI,this,null)===namespace;},
      normalize(){var nodes=this.childNodes;for(var index=0;index<nodes.length;){var node=nodes[index];if(node.nodeType!==3){node.normalize();index++;continue;}while(index+1<nodes.length&&nodes[index+1].nodeType===3){node.data+=nodes[index+1].data;this.removeChild(nodes[index+1]);nodes=this.childNodes;}if(!node.data){this.removeChild(node);nodes=this.childNodes;}else index++;}},
      compareDocumentPosition(other){getType.call(other);if(this===other)return 0;var root=$call($N.getRootNode,this),otherRoot=$call($N.getRootNode,other);if(root!==otherRoot){var order=__ns.nodeComparisonOrder||(__ns.nodeComparisonOrder=new WeakMap());var next=__ns.nodeComparisonNext||1;if(!$c.has(order,this))$c.set(order,this,next++);if(!$c.has(order,other))$c.set(order,other,next++);__ns.nodeComparisonNext=next;return 33|($c.get(order,this)<$c.get(order,other)?4:2);}if($call($N.contains,this,other))return 20;if($call($N.contains,other,this))return 10;var self=this,before=false,found=false;(function visit(node){if(found)return;if(node===self)before=true;if(node===other){found=true;return;}for(var child=$call($N.firstChild,node);child!==null;child=$call($N.nextSibling,child))visit(child);})(root);return before?4:2;}
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
    __ns.projectNativeNode=function(obj){
      if(projectionApply(projectionHas,projected,[obj]))return obj;
      var type=projectionApply(getType,obj,[]),name;
      if(type===1){
        var tag=projectionApply(getLocalTag,obj,[]),namespace=projectionApply(getNamespace,obj,[]);
        if(namespace==='http://www.w3.org/2000/svg'){
          var localStart=0;for(var index=0;index<tag.length;index++)if(tag[index]===':')localStart=index+1;
          name=projectionApply(projectionSlice,tag,[localStart])==='svg'?'SVGSVGElement':'SVGElement';
        }else name=namespace==='http://www.w3.org/1999/xhtml'?htmlInterfaceName(tag):namespace==='http://www.w3.org/1998/Math/MathML'?'MathMLElement':'Element';
      }else name=type===11&&projectionApply(isShadowRoot,obj,[])?'ShadowRoot':({2:'Attr',10:'DocumentType',3:'Text',4:'CDATASection',7:'ProcessingInstruction',8:'Comment',9:'HTMLDocument',11:'DocumentFragment'})[type]||'Node';
      projectionSetPrototype(obj,prototypes[name]);
      projectionApply(projectionAdd,projected,[obj]);
      for(var index=0;index<nativeKeys.length;index++){var key=nativeKeys[index];if(projectionApply(projectionHasOwn,obj,[key]))delete obj[key];}
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
