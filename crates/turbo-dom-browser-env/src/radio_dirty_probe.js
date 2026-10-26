(() => {
  const result = {};
  const group = () => {
    const form = document.createElement('form');
    form.innerHTML = '<input id="a" name="g" type="radio" checked><input id="b" name="g" type="radio">';
    document.body.append(form);
    return [form, form.querySelector('#a'), form.querySelector('#b')];
  };
  const state = (a,b) => [a.checked,b.checked,a.defaultChecked,b.defaultChecked];
  for(const dirty of [false,true]) {
    const [form,a,b]=group();
    if(dirty) a.checked=true;
    b.checked=true;
    const rows=[state(a,b)];
    a.setAttribute('checked','again');rows.push(state(a,b));
    a.removeAttribute('checked');rows.push(state(a,b));
    a.defaultChecked=true;rows.push(state(a,b));
    form.reset();rows.push(state(a,b));
    result[dirty?'dirtyPeer':'cleanPeer']=rows;form.remove();
  }
  {
    const [form,a,b]=group();b.checked=true;
    const copy=a.cloneNode();copy.name='clone';form.append(copy);
    result.cleanClone=[copy.checked,copy.defaultChecked];copy.defaultChecked=true;
    result.cleanClone.push(copy.checked);
    const dirty=b.cloneNode();dirty.name='dirty-clone';form.append(dirty);
    result.dirtyClone=[dirty.checked,dirty.defaultChecked];dirty.defaultChecked=true;
    result.dirtyClone.push(dirty.checked);form.remove();
  }
  {
    const [form,a,b]=group();b.addEventListener('click',event=>event.preventDefault());
    b.click();result.canceledClick=state(a,b);a.defaultChecked=true;
    result.canceledClick.push(a.checked,b.checked);a.defaultChecked=false;result.canceledClick.push(a.checked,b.checked);a.defaultChecked=true;result.canceledClick.push(a.checked,b.checked);form.remove();
  }
  {
    const [form,a,b]=group();b.checked=true;
    const other=document.implementation.createHTMLDocument('adopt');
    other.body.append(other.adoptNode(a));result.adoptClean=[a.checked,a.defaultChecked];
    a.defaultChecked=true;result.adoptClean.push(a.checked);
    other.body.append(other.adoptNode(b));result.adoptDirty=[b.checked,b.defaultChecked];
    b.defaultChecked=true;result.adoptDirty.push(b.checked);form.remove();
  }
  {
    const [form,a,b]=group();b.checked=true;b.name='h';a.defaultChecked=true;
    result.rename=state(a,b);b.name='g';result.rename.push(a.checked,b.checked);form.remove();
  }
  {
    const [form,a,b]=group();b.checked=true;const oldList=form.elements.namedItem('g');
    const other=document.implementation.createHTMLDocument('target');
    other.body.innerHTML='<form id="target"><input type="radio" name="g" value="target"></form>';
    const target=other.querySelector('form');const newList=target.elements;
    target.append(other.adoptNode(b));
    result.adoptLive=[oldList.length,oldList.value,newList.namedItem('g').length,newList.namedItem('g').value,b.checked];
    target.firstChild.checked=true;result.adoptLive.push(b.checked,newList.namedItem('g').value);form.remove();
  }
  {
    const form=document.createElement('form');form.innerHTML='<input name="g" type="radio" checked><input name="g" type="radio" checked>';document.body.append(form);
    result.parsedDefaults=[form.firstChild.checked,form.lastChild.checked];
    form.firstChild.checked=true;form.reset();result.parsedDefaults.push(form.firstChild.checked,form.lastChild.checked);form.remove();
  }
  {
    const [form,a,b]=group();const c=document.createElement('input');c.type='radio';c.name='g';form.append(c);
    b.addEventListener('click',event=>event.preventDefault());b.click();c.defaultChecked=true;
    result.untouchedPeer=[a.checked,b.checked,c.checked];form.remove();
  }
  return result;
})();
