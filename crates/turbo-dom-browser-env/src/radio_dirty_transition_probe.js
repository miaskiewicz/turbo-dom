(() => {
const out={};
for(const action of ['none','checked','defaultChecked','attribute','click','list']){
const f=document.createElement('form');f.innerHTML='<input name=g type=radio checked value=a><input name=g type=radio value=b>';document.body.append(f);const a=f.firstChild,b=f.lastChild;
if(action==='checked')b.checked=true;
if(action==='defaultChecked')b.defaultChecked=true;
if(action==='attribute')b.setAttribute('checked','');
if(action==='click')b.click();
if(action==='list')f.elements.namedItem('g').value='b';
const rows=[[a.checked,b.checked]];a.defaultChecked=false;rows.push([a.checked,b.checked]);a.defaultChecked=true;rows.push([a.checked,b.checked]);b.remove();a.defaultChecked=false;a.defaultChecked=true;rows.push([a.checked]);a.type='checkbox';a.defaultChecked=false;rows.push([a.checked]);a.defaultChecked=true;rows.push([a.checked]);out[action]=rows;f.remove();
}
return out;
})();
