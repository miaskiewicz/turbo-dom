(() => {
  const units=value=>Array.from({length:value.length},(_,i)=>value.charCodeAt(i));
  const results=[];
  for(const point of [0xd800,0xdc00,0xfffd]) {
    const node=document.createElement('button'),character=String.fromCharCode(point);
    node.setAttribute('onclick',"return '"+character+"';");
    document.body.append(node);
    const first=node.onclick;
    results.push({point,attribute:units(node.getAttribute('onclick')),value:units(first.call(node,new Event('click'))),source:units(first.toString()),name:first.name,length:first.length,cached:node.onclick===first});
    node.setAttribute('onclick',"return '"+String.fromCharCode(0xfffd)+"';");
    results[results.length-1].changed=node.onclick!==first;
    results[results.length-1].newValue=units(node.onclick.call(node,new Event('click')));
  }
  const original=document.createElement('button');
  original.setAttribute('onclick',"return '"+String.fromCharCode(0xdc00)+"';");
  document.body.append(original);
  const clone=original.cloneNode(true);
  document.body.append(clone);
  results.push({clone:true,value:units(clone.onclick.call(clone,new Event('click'))),source:units(clone.onclick.toString()),separate:clone.onclick!==original.onclick});
  return results;
})()
