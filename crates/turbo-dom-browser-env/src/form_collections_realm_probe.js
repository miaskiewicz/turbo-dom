(() => {
  const out={},ids=list=>Array.from(list,node=>node.id);
  const frame=document.createElement('iframe');document.body.append(frame);const child=frame.contentWindow,childForm=child.document.createElement('form');child.document.body.append(childForm);childForm.innerHTML='<input id="child-one" name="same"><input id="child-two" name="same">';
  const foreign=childForm.elements,foreignNamed=foreign.namedItem('same');
  out.realm={distinct:child.HTMLFormControlsCollection!==HTMLFormControlsCollection,childBrand:foreign instanceof child.HTMLFormControlsCollection,childRadio:foreignNamed instanceof child.RadioNodeList,borrowedLength:Object.getOwnPropertyDescriptor(HTMLCollection.prototype,'length').get.call(foreign),borrowedNamed:HTMLFormControlsCollection.prototype.namedItem.call(foreign,'same')===foreignNamed};
  frame.remove();out.retained={length:Object.getOwnPropertyDescriptor(HTMLCollection.prototype,'length').get.call(foreign),ids:ids(foreignNamed)};
  return out;
})();
