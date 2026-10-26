(() => {
  const frame=document.createElement('iframe');document.body.append(frame);
  const window=frame.contentWindow,doc=window.document,form=doc.createElement('form');doc.body.append(form);
  form.innerHTML='<input name="group" value="first"><input name="group" value="second">';
  const controls=form.elements,list=controls.namedItem('group');
  const values=[frame,window,doc,form,controls,list];
  globalThis.__internalsLifetime={retained:list,refs:values.map(value=>new WeakRef(value))};
  globalThis.__readInternalsLifetime=()=>__internalsLifetime.refs.map(reference=>reference.deref()!==undefined);
  frame.remove();return 'prepared';
})();
