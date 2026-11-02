JSON.stringify(Array.from(document.querySelectorAll('button'),button=>{
          const computed=getComputedStyle(button);
          return [button.id,button.clientLeft,button.clientTop,computed.borderLeftWidth,computed.borderTopWidth];
        }))