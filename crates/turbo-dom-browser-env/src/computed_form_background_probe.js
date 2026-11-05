(() => {
  // turbo-dom#318: html.css gives buttons (and the button-like inputs) ButtonFace,
  // rgb(239, 239, 239), a UA declaration every author rule and inline style overrides.
  const host = document.createElement('div');
  host.style.cssText = 'background-color:rgb(1, 2, 3)';
  document.body.append(host);
  const style = document.createElement('style');
  document.head.append(style);
  const rows = [];
  for (const [name, markup, authorStyle, inlineStyle] of [
    ['button', '<button>b</button>', '', ''],
    ['input-button', '<input type=button value=b>', '', ''],
    ['input-submit', '<input type=submit>', '', ''],
    ['input-reset', '<input type=reset>', '', ''],
    ['input-upper-type', '<input type=SUBMIT>', '', ''],
    ['div', '<div>d</div>', '', ''],
    ['author-rule', '<button>b</button>', 'button{background-color:rgb(4, 5, 6)}', ''],
    ['author-universal', '<button>b</button>', '*{background:none}', ''],
    ['author-shorthand', '<input type=reset>', '', 'background:rgb(7, 8, 9)'],
    ['inline', '<button>b</button>', '', 'background-color:rgb(10, 11, 12)'],
    ['inherit', '<button>b</button>', '', 'background-color:inherit'],
    ['revert', '<button>b</button>', 'button{background-color:rgb(4, 5, 6)}', 'background-color:revert'],
  ]) {
    style.textContent = authorStyle;
    host.innerHTML = markup;
    const el = host.firstElementChild;
    el.style.cssText = inlineStyle;
    rows.push({ name, backgroundColor: getComputedStyle(el).backgroundColor });
  }
  style.remove();
  host.remove();
  return { rows };
})()
