(async () => {
  let frame=document.createElement('iframe');document.body.append(frame);
  await new Promise(done=>{frame.onload=()=>done();frame.srcdoc='<label for="source-input" id="source-label"></label><input id="source-input">';});
  frame.onload=null;
  let child=frame.contentWindow,doc=frame.contentDocument,input=doc.getElementById('source-input'),list=input.labels;
  const label=document.createElement('label');label.htmlFor='source-input';label.id='adopted-label';document.body.append(label);
  document.body.append(document.adoptNode(input));
  globalThis.__internalsLifetime={retained:list,refs:[frame,child,doc,input,list].map(value=>new WeakRef(value))};
  globalThis.__readInternalsLifetime=()=>{
    const alive=__internalsLifetime.refs.map(ref=>!!ref.deref());
    let labels=null,same=null;
    if(__internalsLifetime.retained){
      const control=document.getElementById('source-input');
      labels=Array.from(__internalsLifetime.retained,label=>label.id);
      same=control.labels===__internalsLifetime.retained;
      control.remove();
    }
    return {alive,labels,same};
  };
  frame.remove();frame=null;child=null;doc=null;input=null;list=null;return 'prepared';
})();
