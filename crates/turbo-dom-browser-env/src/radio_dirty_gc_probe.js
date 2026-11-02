(() => {
  const form=document.createElement('form');form.id='dirty-gc-form';
  form.innerHTML='<input id="dirty-gc-radio" type="radio" checked>';
  document.body.append(form);
  let radio=form.firstChild;radio.checked=false;
  globalThis.__internalsLifetime={retained:null,weak:new WeakRef(radio)};
  globalThis.__readInternalsLifetime=()=>{
    const wrapperAlive=!!__internalsLifetime.weak.deref();
    const current=document.querySelector('#dirty-gc-radio');
    const before=current.checked;
    current.defaultChecked=false;current.defaultChecked=true;
    return {wrapperAlive,before,after:current.checked,defaultChecked:current.defaultChecked};
  };
  radio=null;return 'prepared';
})();
