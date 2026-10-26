(() => {
  const rows = [];
  for (const mode of ["detached", "detached-invalid", "connected-canceled", "removed-before", "removed-submit", "reconnected-submit", "detached-submit-method", "connected-invalid", "reconnected-invalid"]) {
    const form = document.createElement("form"), input = document.createElement("input");
    input.name = "q";
    if (mode.includes("invalid")) input.required = true;
    form.append(input);
    const events = [];
    if (!mode.startsWith("detached")) document.body.append(form);
    if (mode === "removed-before") form.remove();
    input.addEventListener("invalid", () => {
      events.push("invalid");
      if (mode === "reconnected-invalid") { form.remove(); document.body.append(form); }
    });
    form.addEventListener("submit", event => {
      events.push("submit");
      if (mode === "removed-submit" || mode === "reconnected-submit") form.remove();
      if (mode === "reconnected-submit") document.body.append(form);
      if (mode === "connected-canceled" || mode === "reconnected-submit") event.preventDefault();
    });
    form.addEventListener("formdata", () => events.push("formdata"));
    let result;
    try { result = mode === "detached-submit-method" ? form.submit() : form.requestSubmit(); }
    catch (error) { result = error.name; }
    rows.push({mode, events, result: result === undefined ? "undefined" : result});
    form.remove();
  }
  return rows;
})();
