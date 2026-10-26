(() => {
  // Value sanitization of the text-like states, for set values and defaults.
  const out = [];
  for (const type of ["text", "search", "tel", "password", "url", "email", "hidden", "invalid-state"]) {
    const input = document.createElement("input");
    input.type = type;
    input.value = " a\r\nb\nc\rd ";
    out.push([type, input.value]);
    input.setAttribute("value", "\n x \r");
    input.value = input.defaultValue;
    out.push(input.value);
  }
  const multiple = document.createElement("input");
  multiple.type = "email";
  multiple.multiple = true;
  multiple.value = " a@b.c ,\n d@e.f , ";
  out.push(multiple.value);
  const parsed = document.createElement("input");
  parsed.setAttribute("value", "x\ny");
  out.push(parsed.value);
  return out;
})()
