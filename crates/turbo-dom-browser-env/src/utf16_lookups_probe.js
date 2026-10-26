(() => {
  const units=value=>value===null?null:Array.from({length:value.length},(_,i)=>value.charCodeAt(i));
  const values=[String.fromCharCode(0xd800),String.fromCharCode(0xdc00),'�'],host=document.createElement('div');document.body.append(host);
  for(let i=0;i<values.length;i++){const node=document.createElement(values[i]);node.id=values[i];node.className=values[i];node.setAttribute('name',values[i]);node.setAttribute('data-index',String(i));host.append(node);}
  const ids=value=>Array.from(value,node=>node.getAttribute('data-index'));
  const result={id:values.map(value=>document.getElementById(value)?.getAttribute('data-index')),tag:values.map(value=>ids(host.getElementsByTagName(value))),class:values.map(value=>ids(host.getElementsByClassName(value))),name:values.map(value=>ids(document.getElementsByName(value))),selectors:[],outer:units(host.outerHTML)};
  for(const selector of [...values,...values.map(value=>'#'+value),...values.map(value=>'.'+value),...values.map(value=>'[id="'+value+'"]'),String.raw`\d800`,String.raw`#\d800`,String.raw`[id="\d800"]`,String.raw`\dc00`,String.raw`#\dc00`,String.raw`[id="\dc00"]`]) {
    try{result.selectors.push({selector:units(selector),matches:ids(host.querySelectorAll(selector))})}catch(error){result.selectors.push({selector:units(selector),error:error.name})}
  }
  return result;
})()