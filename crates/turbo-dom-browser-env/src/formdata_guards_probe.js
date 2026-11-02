(() => {
  const form = document.createElement("form"),
    input = document.createElement("input");
  input.name = "name";
  input.value = "value";
  form.append(input);
  document.body.append(form);
  const log = [];
  let depth = 0;
  form.addEventListener("submit", (event) => {
    log.push(["submit", event.isTrusted]);
    event.preventDefault();
    if (depth === 0) {
      depth++;
      form.requestSubmit();
      depth--;
    }
  });
  form.addEventListener("formdata", (event) => {
    log.push(["formdata", event.isTrusted, event.bubbles, event.cancelable]);
    try {
      new FormData(form);
      log.push(["nested", "ok"]);
    } catch (error) {
      log.push(["nested", error.name]);
    }
    form.requestSubmit();
    log.push(["request", "done"]);
    form.submit();
    log.push(["submitMethod", "done"]);
  });
  new FormData(form);
  form.requestSubmit();
  return log;
})();
