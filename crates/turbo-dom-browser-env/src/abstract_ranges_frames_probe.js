(() => {
  const node=document.createTextNode('abcdef');document.body.append(node);
  const own=new StaticRange({startContainer:node,startOffset:1,endContainer:node,endOffset:4});
  const frame=document.createElement('iframe');document.body.append(frame);const child=frame.contentWindow;
  const foreign=new child.StaticRange({startContainer:node,startOffset:2,endContainer:node,endOffset:2});
  const get=Object.getOwnPropertyDescriptor(AbstractRange.prototype,'collapsed').get;
  const foreignGet=Object.getOwnPropertyDescriptor(child.NodeRange.prototype,'startContainer').get;
  const out={cross:[get.call(foreign),foreignGet.call(own)===node],distinct:[child.StaticRange!==StaticRange,child.AbstractRange!==AbstractRange,child.NodeRange!==NodeRange]};
  frame.remove();node.remove();out.retained=[get.call(foreign),foreignGet.call(own)===node,own.endOffset];
  return out;
})();
