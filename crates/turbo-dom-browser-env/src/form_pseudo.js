// Form-state pseudo-classes (:checked, :indeterminate, :default, :placeholder-shown,
// :read-write, :in-range, :out-of-range, :valid, :invalid, :user-valid, :user-invalid,
// :enabled, :disabled, :required, :optional) classified from the binding's private control
// state, as Blink's SelectorChecker reads it. form_pseudo.rs turns the result into structural
// selectors for the query that asked; nothing is written into the guest's tree.
(function () {
  'use strict';
  const g = globalThis, ns = g[Symbol.for('ts')];
  const forms = g.__captureFormInternals;
  const apply = Reflect.apply, describe = Object.getOwnPropertyDescriptor, define = Object.defineProperty;
  const MapClass = Map, mapGet = Map.prototype.get, mapSet = Map.prototype.set, mapHas = Map.prototype.has;
  const indexOf = Array.prototype.indexOf;
  // Guest code runs between queries: never reach mutable prototypes while classifying.
  const includes = (list, value) => apply(indexOf, list, [value]) >= 0;
  const push = (list, value) => define(list, list.length, { __proto__: null, value, writable: true, enumerable: true, configurable: true });
  const lower = Function.prototype.call.bind(String.prototype.toLowerCase);
  const localNameGetter = ns.domAccessor('localName', false, 'Element');
  const namespaceGetter = ns.domAccessor('namespaceURI', false, 'Element');
  const contains = Node.prototype.contains;
  const selectedGetter = describe(g.HTMLOptionElement.prototype, 'selected').get;
  const HTML = 'http://www.w3.org/1999/xhtml';
  const { attribute: attr, parent, owner, disabled, candidate, flags, formControls, readValue, checkedValue, associatedGet, userValidity, rangeState, indeterminate } = forms;
  const controlType = forms.type;
  const present = (element, name) => attr(element, name) !== null;
  const knownTypes = ['hidden', 'text', 'search', 'tel', 'url', 'email', 'password', 'date', 'month', 'week', 'time', 'datetime-local', 'number', 'range', 'color', 'checkbox', 'radio', 'file', 'submit', 'image', 'reset', 'button'];
  const inputType = (element) => { const value = lower(attr(element, 'type') || ''); return includes(knownTypes, value) ? value : 'text'; };
  const readOnlyTypes = ['text', 'search', 'url', 'tel', 'email', 'password', 'date', 'month', 'week', 'time', 'datetime-local', 'number'];
  const placeholderTypes = ['text', 'search', 'url', 'tel', 'email', 'password', 'number'];
  const unrequirable = ['hidden', 'range', 'color', 'submit', 'image', 'reset', 'button'];
  const listed = ['input', 'button', 'select', 'textarea', 'object', 'output'];
  const enablableTags = ['button', 'input', 'select', 'textarea', 'fieldset', 'optgroup', 'option'];
  const optionalTags = ['input', 'select', 'textarea', 'button'];
  const submitButton = (element, kind) => kind === 'button' ? controlType(element) === 'submit' : kind === 'input' && (inputType(element) === 'submit' || inputType(element) === 'image');

  // Rich editability: the nearest contenteditable state wins; invalid values inherit.
  const editable = (element) => {
    for (let node = element; node; node = parent(node)) {
      if (apply(namespaceGetter, node, []) !== HTML) continue;
      const value = attr(node, 'contenteditable');
      if (value === null) continue;
      const state = lower(value);
      if (state === '' || state === 'true' || state === 'plaintext-only') return true;
      if (state === 'false') return false;
    }
    return false;
  };

  function classify(elements, names) {
    const records = [];
    // Radio groups: owner form and non-empty name; an unnamed radio is its own group.
    const groups = new MapClass();
    const defaults = new MapClass();
    for (let index = 0; index < elements.length; index++) {
      const element = elements[index], html = apply(namespaceGetter, element, []) === HTML;
      const kind = html ? apply(localNameGetter, element, []) : null;
      const record = { __proto__: null, element, kind, html, custom: html && !!associatedGet(element), type: null, candidate: null, valid: null };
      push(records, record);
      if (kind === 'input') {
        record.type = inputType(element);
        if (record.type === 'radio') {
          const name = attr(element, 'name');
          if (name) {
            const form = owner(element);
            let byName = apply(mapGet, groups, [form]);
            if (!byName) apply(mapSet, groups, [form, byName = new MapClass()]);
            apply(mapSet, byName, [name, apply(mapGet, byName, [name]) || checkedValue(element)]);
          }
        }
      }
    }
    // Constraint validation is computed only for the pseudo-classes that read it.
    const candidateOf = (record) => {
      if (record.candidate === null) record.candidate = (record.custom || includes(listed, record.kind)) && candidate(record.element);
      return record.candidate;
    };
    const validOf = (record) => {
      if (record.valid === null) record.valid = candidateOf(record) && flags(record.element).valid;
      return record.valid;
    };
    let invalidControls = null;
    const invalid = () => {
      if (!invalidControls) {
        invalidControls = [];
        for (let index = 0; index < records.length; index++) if (candidateOf(records[index]) && !validOf(records[index])) push(invalidControls, records[index].element);
      }
      return invalidControls;
    };
    const defaultButton = (form) => {
      if (!apply(mapHas, defaults, [form])) {
        const controls = formControls(form);
        let found = null;
        for (let index = 0; index < controls.length && !found; index++) {
          const control = controls[index];
          if (apply(namespaceGetter, control, []) === HTML && submitButton(control, apply(localNameGetter, control, []))) found = control;
        }
        apply(mapSet, defaults, [form, found]);
      }
      return apply(mapGet, defaults, [form]);
    };
    const radioUnchecked = (record) => {
      const element = record.element, name = attr(element, 'name');
      if (!name) return !checkedValue(element);
      const byName = apply(mapGet, groups, [owner(element)]);
      return !(byName && apply(mapGet, byName, [name]));
    };
    const disabledControl = (record) => {
      const element = record.element;
      if (record.kind === 'option') {
        if (present(element, 'disabled')) return true;
        const group = parent(element);
        return !!group && apply(namespaceGetter, group, []) === HTML && apply(localNameGetter, group, []) === 'optgroup' && present(group, 'disabled');
      }
      if (record.kind === 'optgroup') return present(element, 'disabled');
      return disabled(element);
    };
    const enablable = (record) => record.custom || includes(enablableTags, record.kind);
    const required = (record) => {
      const element = record.element;
      if (record.kind === 'input') return present(element, 'required') && !includes(unrequirable, record.type);
      return (record.kind === 'select' || record.kind === 'textarea') && present(element, 'required');
    };
    const formInvalid = (form) => {
      const controls = invalid();
      for (let index = 0; index < controls.length; index++) if (owner(controls[index]) === form) return true;
      return false;
    };
    const fieldsetInvalid = (fieldset) => {
      const controls = invalid();
      for (let index = 0; index < controls.length; index++) if (controls[index] !== fieldset && apply(contains, fieldset, [controls[index]])) return true;
      return false;
    };
    const validity = (record) => {
      if (record.kind === 'form') return formInvalid(record.element) ? 'invalid' : 'valid';
      if (record.kind === 'fieldset') return fieldsetInvalid(record.element) ? 'invalid' : 'valid';
      if (!candidateOf(record)) return null;
      return validOf(record) ? 'valid' : 'invalid';
    };
    const range = (record) => {
      if (record.kind !== 'input' || !candidateOf(record)) return null;
      const state = rangeState(record.element);
      if (!state) return null;
      if (state.empty || !state.finite) return 'in';
      if (!state.limited) return null;
      const result = flags(record.element);
      return result.rangeUnderflow || result.rangeOverflow ? 'out' : 'in';
    };
    const tests = {
      __proto__: null,
      checked: (record) => record.kind === 'input' ? (record.type === 'checkbox' && checkedValue(record.element) && !indeterminate(record.element)) || (record.type === 'radio' && checkedValue(record.element))
        : record.kind === 'option' && apply(selectedGetter, record.element, []),
      indeterminate: (record) => record.kind === 'input' ? (record.type === 'checkbox' && indeterminate(record.element)) || (record.type === 'radio' && radioUnchecked(record))
        : record.kind === 'progress' && !present(record.element, 'value'),
      default: (record) => {
        const element = record.element;
        if (record.kind === 'input' && (record.type === 'checkbox' || record.type === 'radio')) return present(element, 'checked');
        if (record.kind === 'option') return present(element, 'selected');
        if (!submitButton(element, record.kind)) return false;
        const form = owner(element);
        return !!form && defaultButton(form) === element;
      },
      'placeholder-shown': (record) => (record.kind === 'textarea' || record.kind === 'input' && includes(placeholderTypes, record.type))
        && present(record.element, 'placeholder') && readValue(record.element) === '',
      'read-write': (record) => {
        const element = record.element;
        if (record.kind === 'input') return includes(readOnlyTypes, record.type) && !present(element, 'readonly') && !disabled(element);
        if (record.kind === 'textarea') return !present(element, 'readonly') && !disabled(element);
        return editable(element);
      },
      'in-range': (record) => range(record) === 'in',
      'out-of-range': (record) => range(record) === 'out',
      valid: (record) => validity(record) === 'valid',
      invalid: (record) => validity(record) === 'invalid',
      'user-valid': (record) => (record.kind === 'input' || record.kind === 'select' || record.kind === 'textarea') && userValidity(record.element) && validity(record) === 'valid',
      'user-invalid': (record) => (record.kind === 'input' || record.kind === 'select' || record.kind === 'textarea') && userValidity(record.element) && validity(record) === 'invalid',
      enabled: (record) => enablable(record) && !disabledControl(record),
      disabled: (record) => enablable(record) && disabledControl(record),
      required: (record) => required(record),
      optional: (record) => includes(optionalTags, record.kind) && !required(record),
    };
    const result = [];
    for (let at = 0; at < names.length; at++) {
      const test = tests[names[at]], matched = [];
      for (let index = 0; index < records.length; index++) if (test(records[index])) push(matched, records[index].element);
      push(result, matched);
    }
    return result;
  }
  g.__captureFormPseudo = classify;
})();
