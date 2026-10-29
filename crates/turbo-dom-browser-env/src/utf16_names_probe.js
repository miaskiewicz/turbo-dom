(() => {
  const units=value=>value===null?null:Array.from({length:value.length},(_,i)=>value.charCodeAt(i));
  const values=[String.fromCharCode(0xd800),String.fromCharCode(0xdc00),'�'];
  const result={names:values.map(value=>{const errors={};for(const [name,operation] of [['element',()=>document.createElement(value)],['attribute',()=>document.createAttribute(value)],['elementNS',()=>document.createElementNS('urn:u',value)],['attributeNS',()=>document.createAttributeNS('urn:u',value)]])try{errors[name]=units(operation().nodeName)}catch(error){errors[name]=error.name}return {value:units(value),errors}}),namespaces:[],equality:{}};
  const host=document.createElement('div');for(let i=0;i<values.length;i++){host.setAttributeNS(values[i],'name',String(i));result.namespaces.push({element:units(document.createElementNS(values[i],'x').namespaceURI),attr:units(document.createAttributeNS(values[i],'x').namespaceURI)});}
  result.namespaceValues=values.map(value=>host.getAttributeNS(value,'name'));
  result.attributes=Array.from(host.attributes,attribute=>({namespace:units(attribute.namespaceURI),value:attribute.value}));
  result.equality.text=document.createTextNode(values[0]).isEqualNode(document.createTextNode(values[2]));
  const first=document.createAttribute('name'),second=document.createAttribute('name');first.value=values[0];second.value=values[2];result.equality.attr=first.isEqualNode(second);
  result.equality.clone=first.isEqualNode(first.cloneNode());return result;
})()