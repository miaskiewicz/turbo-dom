(() => {
  // Form-state pseudo-classes over one control matrix, through querySelectorAll,
  // matches() and closest(), before and after state changes that only IDL,
  // validation APIs or submission attempts can make.
  const pseudos = [':checked', ':indeterminate', ':default', ':placeholder-shown', ':read-only', ':read-write',
    ':in-range', ':out-of-range', ':valid', ':invalid', ':user-valid', ':user-invalid', ':autofill',
    ':-webkit-autofill', ':enabled', ':disabled', ':required', ':optional'];
  document.head.replaceChildren();
  document.body.innerHTML = `
    <form id="f1">
      <input id="cb-plain" type="checkbox">
      <input id="cb-checked" type="checkbox" checked>
      <input id="cb-idl" type="checkbox">
      <input id="cb-unchecked" type="checkbox" checked>
      <input id="cb-indet" type="checkbox">
      <input id="cb-indet-checked" type="checkbox" checked>
      <input id="cb-switch" type="checkbox" switch>
      <input id="cb-disabled" type="checkbox" disabled checked>
      <input id="ra1" type="radio" name="a"><input id="ra2" type="radio" name="a">
      <input id="rb1" type="radio" name="b" checked><input id="rb2" type="radio" name="b">
      <input id="rc1" type="radio" name="c"><input id="rc2" type="radio" name="c">
      <input id="rn" type="radio">
      <input id="rd" type="radio" name="d" disabled>
      <button id="b-submit">go</button><button id="b-second">again</button>
      <button id="b-button" type="button">b</button><button id="b-reset" type="reset">r</button>
      <input id="s-input" type="submit"><input id="s-image" type="image" alt="i">
    </form>
    <form id="f2"><input id="f2-text"><button id="f2-button" type="button">x</button></form>
    <form id="f3"><input id="f3-image" type="image" alt="i"><input id="f3-submit" type="submit"></form>
    <button id="b-outside" form="f2">o</button>
    <input id="ra-other" type="radio" name="a">
    <progress id="p-none"></progress><progress id="p-value" value="0.5"></progress>
    <progress id="p-removed" value="1"></progress><progress id="p-later"></progress>
    <meter id="m1" value="0.5"></meter>
    <select id="sel"><option id="o1">1</option><option id="o2" selected>2</option><option id="o3">3</option></select>
    <select id="sel-plain"><option id="o4">4</option><option id="o5">5</option></select>
    <select id="sel-multi" multiple><option id="o6" selected>6</option><option id="o7">7</option><option id="o8" selected>8</option></select>
    <select id="sel-req" required><option id="o9" value="">none</option><option id="o10">10</option></select>
    <input id="ph-text" placeholder="name"><input id="ph-value" placeholder="name" value="x">
    <input id="ph-empty" placeholder=""><input id="ph-newline" placeholder="&#10;">
    <input id="ph-idl" placeholder="name"><input id="ph-number" type="number" placeholder="1">
    <input id="ph-checkbox" type="checkbox" placeholder="c"><input id="ph-date" type="date" placeholder="d">
    <input id="ph-email" type="email" placeholder="e"><input id="ph-hidden" type="hidden" placeholder="h">
    <input id="ph-range" type="range" placeholder="r"><input id="ph-color" type="color" placeholder="c">
    <textarea id="ph-area" placeholder="t"></textarea><textarea id="ph-area-text" placeholder="t">x</textarea>
    <input id="ph-readonly" placeholder="p" readonly><input id="ph-disabled" placeholder="p" disabled>
    <input id="rw-text"><input id="rw-readonly" readonly><input id="rw-disabled" disabled>
    <input id="rw-hidden" type="hidden"><input id="rw-range" type="range"><input id="rw-color" type="color">
    <input id="rw-file" type="file"><input id="rw-date" type="date"><input id="rw-button" type="button" value="b">
    <input id="rw-number-ro" type="number" readonly><textarea id="rw-area"></textarea>
    <textarea id="rw-area-ro" readonly></textarea><textarea id="rw-area-dis" disabled></textarea>
    <div id="ce" contenteditable><span id="ce-child">c</span><b id="ce-false" contenteditable="false">f</b></div>
    <div id="ce-plaintext" contenteditable="plaintext-only"></div><div id="ce-off" contenteditable="false"></div>
    <fieldset id="fs-dis" disabled><input id="fs-input"><legend><input id="fs-legend-input"></legend></fieldset>
    <input id="ir-in" type="number" min="1" max="10" value="5"><input id="ir-over" type="number" min="1" max="10" value="11">
    <input id="ir-under" type="number" min="1" max="10" value="0"><input id="ir-empty" type="number" min="1" max="10">
    <input id="ir-nolimit" type="number" value="5"><input id="ir-range" type="range" min="0" max="10" value="20">
    <input id="ir-date" type="date" min="2020-01-01" max="2020-12-31" value="2021-01-01">
    <input id="ir-time" type="time" min="10:00" max="09:00" value="11:00">
    <input id="ir-text" type="text" min="1" max="2" value="5"><input id="ir-step" type="number" min="0" max="10" step="3" value="4">
    <input id="ir-disabled" type="number" max="1" value="5" disabled><input id="ir-readonly" type="number" max="1" value="5" readonly>
    <input id="ir-idl" type="number" min="1" max="10" value="5">
    <input id="ir-date-nolimit" type="date" value="2020-05-05"><input id="ir-month" type="month" max="2020-01" value="2020-02">
    <input id="ir-week" type="week" min="2020-W10" value="2020-W09"><input id="ir-local" type="datetime-local" max="2020-01-01T00:00" value="2020-01-01T00:01">
    <input id="ir-time-reversed" type="time" min="10:00" max="09:00" value="09:30"><input id="ir-date-step" type="date" step="2" value="2020-01-02">
    <form id="fv">
      <fieldset id="fs-v"><input id="v-required" required><input id="v-filled" required value="x"></fieldset>
      <input id="v-email" type="email" value="nope"><input id="v-pattern" pattern="[0-9]+" value="abc">
      <input id="v-custom"><input id="v-ok"><input id="v-hidden" type="hidden" required>
      <input id="v-disabled" required disabled><input id="v-readonly" required readonly>
      <textarea id="v-area" required></textarea><output id="v-output">o</output>
      <button id="v-submit">s</button><button id="v-button" type="button">b</button>
      <object id="v-object"></object><input id="v-list" required list="dl"><datalist id="dl"><input id="v-in-datalist" required></datalist>
      <input id="v-check" type="checkbox" required><input id="v-radio" type="radio" name="vr" required><input id="v-radio2" type="radio" name="vr">
    </form>
    <fieldset id="fs-ok"><input id="fs-ok-input"></fieldset>
    <form id="fr"><input id="fr-bad" required><input id="fr-good" value="g"></form>
    <form id="fc"><input id="fc-bad" required></form>
    <form id="fe"><input id="fe-bad" required><input id="fe-other" required></form>
    <optgroup id="og" disabled><option id="og-option">x</option></optgroup>`;
  const $ = (id) => document.getElementById(id);
  $('cb-idl').checked = true;
  $('cb-unchecked').checked = false;
  $('cb-indet').indeterminate = true;
  $('cb-indet-checked').indeterminate = true;
  $('cb-switch').indeterminate = true;
  $('rc1').checked = true;
  $('p-removed').removeAttribute('value');
  $('ph-idl').value = 'typed';
  $('v-custom').setCustomValidity('bad');
  $('ir-idl').value = '42';
  const label = (element) => element.id || element.localName;
  const detached = () => {
    const box = document.createElement('input');
    box.type = 'checkbox';
    box.indeterminate = true;
    const radio = document.createElement('input');
    radio.type = 'radio';
    radio.name = 'z';
    const progress = document.createElement('progress');
    const field = document.createElement('input');
    field.placeholder = 'p';
    field.required = true;
    const wrapper = document.createElement('form');
    const inner = document.createElement('input');
    inner.required = true;
    wrapper.append(inner);
    return { box, radio, progress, field, wrapper, inner };
  };
  const nodesOutside = () => { const nodes = detached(); return nodes.wrapper; };
  const snapshot = () => {
    const out = {};
    for (const pseudo of pseudos) {
      try {
        out[pseudo] = Array.from(document.querySelectorAll(pseudo), label);
      } catch (error) {
        out[pseudo] = [error.name];
      }
    }
    const nodes = detached();
    out.detached = {};
    for (const [name, node] of Object.entries(nodes)) {
      out.detached[name] = pseudos.filter((pseudo) => { try { return node.matches(pseudo); } catch (_) { return false; } });
    }
    out.compound = {
      notIndeterminate: Array.from(document.querySelectorAll('input[type=checkbox]:not(:indeterminate)'), label),
      isChecked: Array.from(document.querySelectorAll(':is(:checked, :indeterminate):not(option)'), label),
      closest: label($('ce-child').closest(':read-write:not(span)') || { localName: 'none' }),
      invalidForms: Array.from(document.querySelectorAll('form:invalid, fieldset:invalid'), label),
      scoped: Array.from($('f1').querySelectorAll(':scope > :indeterminate'), label),
      hasInvalid: Array.from(document.querySelectorAll('form:has(:invalid)'), label),
    };
    const fragment = document.createDocumentFragment();
    fragment.append(document.createElement('i'), document.createElement('b'));
    fragment.firstChild.append(document.createElement('u'));
    out.scope = {
      element: Array.from($('f1').querySelectorAll(':scope > button:default, :scope > :checked'), label),
      document: Array.from(document.querySelectorAll(':scope > body, :scope'), label),
      matches: [$('ce').matches(':scope'), $('ce-child').matches(':scope > span'), label($('ce-child').closest(':scope') || { localName: 'none' })],
      fragment: Array.from(fragment.querySelectorAll(':scope > *, :scope'), label),
      detached: Array.from(nodesOutside().querySelectorAll(':scope > :required'), label),
    };
    out.api = ['s-image', 'f3-image', 'b-button', 'b-reset', 'v-output', 'v-object', 'v-hidden', 'v-in-datalist', 'v-disabled', 'v-readonly', 'fs-input', 'ir-disabled', 'ir-readonly', 'ir-date', 'ir-time', 'ir-month', 'ir-week', 'ir-local', 'ir-time-reversed', 'ir-date-step', 'ir-date-nolimit']
      .map((id) => { const element = $(id), validity = element.validity; return [id, element.willValidate, validity.valid, validity.rangeUnderflow, validity.rangeOverflow, validity.stepMismatch]; });
    return out;
  };
  const result = { initial: snapshot() };
  $('cb-indet').indeterminate = false;
  $('ra1').checked = true;
  $('p-later').value = 0.25;
  $('ph-text').value = 'now';
  $('ir-over').value = '3';
  $('v-required').value = 'filled';
  $('cb-checked').checked = false;
  $('o3').selected = true;
  $('b-submit').type = 'button';
  result.changed = snapshot();
  const prevented = (event) => event.preventDefault();
  document.addEventListener('submit', prevented);
  result.requestSubmit = (() => { try { $('fv').requestSubmit(); return 'ok'; } catch (error) { return error.name; } })();
  result.afterRequestSubmit = snapshot();
  result.reportValidity = [$('fr').reportValidity(), $('fe-bad').reportValidity(), $('fc-bad').checkValidity()];
  result.afterReport = snapshot();
  $('fv').reset();
  result.afterReset = snapshot();
  document.removeEventListener('submit', prevented);
  return result;
})()
