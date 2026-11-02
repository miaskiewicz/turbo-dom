// Window interface chain probe (miaskiewicz/turbo-dom#223); run as a function body.
const out={};const chain=[];let o=window;while(o){const tag=Object.prototype.hasOwnProperty.call(o,Symbol.toStringTag)?o[Symbol.toStringTag]:null;chain.push({tag,ctor:Object.prototype.hasOwnProperty.call(o,'constructor')?o.constructor.name:null,own:Reflect.ownKeys(o).length,syms:Object.getOwnPropertySymbols(o).map(String)});o=Object.getPrototypeOf(o);}
out.chain=chain.slice(1);out.winSyms=Object.getOwnPropertySymbols(window).map(String);
out.instWindow=typeof Window==='function'&&window instanceof Window;out.instET=window instanceof EventTarget;
out.wp=Reflect.ownKeys(Window.prototype).map(String);
out.wpd=Object.fromEntries(Reflect.ownKeys(Window.prototype).map(k=>[String(k),JSON.stringify(Object.getOwnPropertyDescriptor(Window.prototype,k))]));
const WP=Object.getPrototypeOf(Window.prototype);out.wps=Reflect.ownKeys(WP).map(String);out.wpsd=JSON.stringify(Object.getOwnPropertyDescriptor(WP,Symbol.toStringTag));
out.wdesc=JSON.stringify(Object.getOwnPropertyDescriptor(window,'Window'));out.wlen=Window.length;out.wname=Window.name;
try{new Window();out.newW='ok'}catch(e){out.newW=e.constructor.name+': '+e.message}
try{Window()}catch(e){out.callW=e.constructor.name+': '+e.message}
out.own=['addEventListener','removeEventListener','dispatchEvent','window','self','document','location'].map(k=>[k,Object.prototype.hasOwnProperty.call(window,k)]);
out.str=String(window);out.ots=Object.prototype.toString.call(window);
out.wpProtoStr=Object.prototype.toString.call(Window.prototype);out.wpsStr=Object.prototype.toString.call(WP);
out.WPcons=Object.getOwnPropertyNames(WP);
out.etOk=(()=>{let n=0;EventTarget.prototype.addEventListener.call(window,'x',()=>n++);window.dispatchEvent(new Event('x'));return n})();
return JSON.stringify(out);
