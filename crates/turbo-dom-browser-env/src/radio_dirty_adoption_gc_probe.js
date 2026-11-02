(async () => {
  let frame=document.createElement('iframe');document.body.append(frame);
  await new Promise(done=>{frame.onload=()=>done();frame.srcdoc='<form><input type="radio" name="g" checked value="a"><input type="radio" name="g" value="b"></form>';});
  frame.onload=null;
  let child=frame.contentWindow,doc=frame.contentDocument,form=doc.querySelector('form'),radio=form.lastChild;
  radio.checked=true;
  const target=document.createElement('form');target.id='adopted-radio-form';target.innerHTML='<input type="radio" name="g" value="parent">';document.body.append(target);
  target.append(document.adoptNode(radio));
  globalThis.__internalsLifetime={retained:radio,refs:[frame,child,doc,radio].map(value=>new WeakRef(value))};
  globalThis.__readInternalsLifetime=()=>{
    const alive=__internalsLifetime.refs.map(ref=>!!ref.deref());
    const selected=document.querySelector('#adopted-radio-form').elements.namedItem('g').value;
    if(__internalsLifetime.retained){__internalsLifetime.retained.remove();}
    return {alive,selected};
  };
  frame.remove();frame=null;child=null;doc=null;form=null;radio=null;return 'prepared';
})();
