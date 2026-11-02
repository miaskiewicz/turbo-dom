(() => {
  // Each candidate is checked through every author-facing selector grammar entry:
  // stylesheet parsing, insertRule, selectorText assignment, CSS.supports and
  // querySelector. A list-form row proves non-forgiving lists drop as a unit.
  const classes = [
    "active", "any-link", "autofill", "blank", "buffering", "checked", "closed",
    "current", "default", "defined", "dir(rtl)", "disabled", "empty", "enabled",
    "first", "first-child", "first-of-type", "focus", "focus-visible",
    "focus-within", "fullscreen", "future", "has-slotted", "heading",
    "heading(1)", "host", "host(.a)", "host-context(.a)", "hover",
    "in-range", "indeterminate", "interest-source", "interest-target",
    "invalid", "is(.a)", "lang(en)", "last-child", "last-of-type", "left",
    "link", "local-link", "modal", "muted", "not(.a)", "nth-child(2n+1)",
    "nth-child(2 of .a)", "nth-col(1)", "nth-last-child(1)",
    "nth-last-col(1)", "nth-last-of-type(1)", "nth-of-type(1)", "only-child",
    "only-of-type", "open", "optional", "out-of-range", "past", "paused",
    "picture-in-picture", "placeholder-shown", "playing", "popover-open",
    "read-only", "read-write", "required", "right", "root", "scope", "seeking",
    "stalled", "state(ready)", "target", "target-current", "target-within",
    "user-invalid", "user-valid", "valid", "visited", "volume-locked",
    "where(.a)", "has(.a)", "xr-overlay", "active-view-transition",
    "active-view-transition-type(a)", "drop", "focus-ring", "unresolved",
    "matches(.a)", "any(.a)", "local(.a)", "global(.a)", "local", "global",
    "horizontal", "vertical", "decrement", "increment", "start", "end",
    "double-button", "single-button", "no-button", "corner-present",
    "window-inactive", "selection", "placeholder", "before", "after",
    "first-line", "first-letter", "marker", "backdrop", "definitely-unknown",
    "definitely-unknown(.a)", "HOVER", "Is(.a)", "NTH-CHILD(odd)",
    "-webkit-any-link", "-webkit-any(.a)", "-webkit-autofill", "-webkit-drag",
    "-webkit-full-screen", "-webkit-full-screen-ancestor",
    "-webkit-full-screen-document", "-webkit-full-page-media",
    "-webkit-full-screen-controls-hidden", "-webkit-definitely-unknown",
    "-moz-any-link", "-moz-focusring", "-moz-placeholder", "-moz-read-only",
    "-ms-input-placeholder", "-internal-autofill-selected",
    "-internal-list-box", "-internal-definitely-unknown",
  ];
  const elements = [
    "after", "before", "backdrop", "checkmark", "column", "cue", "cue(.a)",
    "cue-region", "details-content", "file-selector-button", "first-letter",
    "first-line", "grammar-error", "highlight(a)", "interest-hint", "marker",
    "part(a)", "picker(select)", "picker-icon", "placeholder",
    "scroll-button(*)", "scroll-marker", "scroll-marker-group", "search-text",
    "selection", "slotted(*)", "slotted(.a)", "spelling-error", "target-text",
    "view-transition", "view-transition-group(*)",
    "view-transition-image-pair(*)", "view-transition-new(*)",
    "view-transition-old(*)", "view-transition-group-children(*)",
    "definitely-unknown", "definitely-unknown(a)", "BEFORE", "Part(a)",
    "-webkit-scrollbar", "-webkit-scrollbar-button", "-webkit-scrollbar-corner",
    "-webkit-scrollbar-thumb", "-webkit-scrollbar-track",
    "-webkit-scrollbar-track-piece", "-webkit-resizer",
    "-webkit-input-placeholder", "-webkit-file-upload-button",
    "-webkit-search-cancel-button", "-webkit-inner-spin-button",
    "-webkit-details-marker", "-webkit-progress-bar", "-webkit-slider-thumb",
    "-webkit-meter-bar", "-webkit-backdrop", "-webkit-definitely-unknown",
    "-webkit-definitely-unknown(a)", "-moz-selection", "-moz-placeholder",
    "-ms-clear", "-internal-media-controls-overlay-cast-button",
    "-internal-definitely-unknown", "-webkit-slider-runnable-track",
    "-webkit-progress-value", "-webkit-progress-inner-element",
    "-webkit-meter-optimum-value", "-webkit-meter-suboptimum-value",
    "-webkit-meter-even-less-good-value", "-webkit-search-decoration",
    "-webkit-search-results-button", "-webkit-search-results-decoration",
    "-webkit-calendar-picker-indicator", "-webkit-clear-button",
    "-webkit-datetime-edit", "-webkit-datetime-edit-fields-wrapper",
    "-webkit-datetime-edit-text", "-webkit-datetime-edit-year-field",
    "-webkit-datetime-edit-month-field", "-webkit-datetime-edit-day-field",
    "-webkit-datetime-edit-hour-field", "-webkit-datetime-edit-minute-field",
    "-webkit-datetime-edit-second-field", "-webkit-datetime-edit-ampm-field",
    "-webkit-color-swatch", "-webkit-color-swatch-wrapper",
    "-webkit-media-controls", "-webkit-media-controls-panel",
    "-webkit-media-controls-play-button", "-webkit-media-controls-timeline",
    "-webkit-media-controls-enclosure", "-webkit-media-text-track-container",
    "-webkit-textfield-decoration-container", "-webkit-validation-bubble",
    "-webkit-outer-spin-button", "-webkit-caps-lock-indicator",
    "-webkit-contacts-auto-fill-button", "-webkit-credentials-auto-fill-button",
    "-webkit-input-speech-button", "-webkit-keygen-select",
    "-webkit-full-page-media", "-webkit-file-upload-button(a)",
  ];
  const compound = [
    ":is(.a, :definitely-unknown)", ":is(:definitely-unknown)",
    ":where(.a, :definitely-unknown)", ":where(::before)", ":is(.a, ???)",
    ":is(.a, ::-webkit-definitely-unknown)", ":has(.a, :definitely-unknown)",
    ":has(:definitely-unknown)", ":has(:is(:definitely-unknown))",
    ":not(.a, :definitely-unknown)", ":not(:is(:definitely-unknown))",
    ":nth-child(2 of :definitely-unknown)", ":nth-child(2 of :is(:definitely-unknown))",
    ":host(:definitely-unknown)", "::slotted(:definitely-unknown)",
    ":is(:hover, :-webkit-autofill)", ":is(:-moz-focusring)",
    "::-webkit-scrollbar:horizontal", "::-webkit-scrollbar-thumb:window-inactive",
    "::-webkit-scrollbar:definitely-unknown", "::before:hover",
    "::part(a):hover", "::before .a", "::-webkit-definitely-unknown .a",
    ".a::-webkit-definitely-unknown", "::-webkit-definitely-unknown:hover",
    ":is(::-webkit-definitely-unknown)", "::selection:window-inactive",
    "::-webkit-input-placeholder:hover", ".a:hover::before", ":hover:is(.a)",
    ":not(:hover, :focus)", ":has(> .a)", ":has(:has(.a))", ":is(:has(.a))",
    ":host(.a):hover", ":state(--x)", "::highlight(--x)",
    "::before::marker", "::after::marker", "::part(a)::before",
    "::slotted(.a)::before", "::before::before", "::marker::before",
    "::part(a):first-child", "::part(a):is(:hover)", "::part(a):not(:hover)",
    "::part(a)::-webkit-scrollbar", "::part(a)::part(b)",
    "::-webkit-scrollbar:hover", "::-webkit-scrollbar-thumb:horizontal:active",
    "::-webkit-scrollbar:first-child", "::-webkit-scrollbar:horizontal .a",
    "::selection:hover", "::placeholder:hover", "::marker:hover",
    "::backdrop:hover", "::-webkit-input-placeholder:first-child",
    "::-webkit-input-placeholder:focus", "::-webkit-input-placeholder::before",
    ":horizontal::-webkit-scrollbar", ".a :horizontal",
    ":-webkit-any(.a, :definitely-unknown)", ":-webkit-any(.a .b)",
    ":host-context(:definitely-unknown)", "::cue(:definitely-unknown)",
    ":nth-child(odd of .a, :definitely-unknown)",
    ":is(.a, :definitely-unknown)::before", ":where(:is(:definitely-unknown))",
    ":active-view-transition-type(a, b)", ":dir(sideways)", ':lang("en")',
    ":lang(en, fr)", "::highlight(a b)", "::part(a b)", "::picker(foo)",
    "::view-transition-group(.a)", "::view-transition-group(a.b)",
    "::scroll-button(up)", "::scroll-button(foo)", ":nth-child(-n + 3)",
    ":nth-child(+5)", ":nth-of-type(EVEN)", ":nth-last-child(0n+0)",
    ":nth-child(-2n-1)", ":is()", ":not()", ":has()", ":where()",
    ":is(.a,,.b)", ":not(::before)", ":has(::before)", ":host(::before)",
    ":is(:host)", ":has(:host)", ":host:has(.a)", ":has(:is(:has(.a)))",
    ":has(:not(:has(.a)))", ":has(:where(.a))", ":scope > .a",
    "a|b", "*|a", "|a", ".a:not(:is(.b, :definitely-unknown), .c)",
    ":is(.a, :definitely-unknown, .b)", ":IS(.a, :DEFINITELY-unknown)",
    ":is( .a , :definitely-unknown )", ":is(.a /* x */, :definitely-unknown)",
    ":is(.a:hover, :definitely-unknown > .b)", "& .a", ".a &", ":is(&)",
    "::scroll-button(down)", "::scroll-button(left)", "::scroll-button(right)",
    "::scroll-button(block-start)", "::scroll-button(inline-end)",
    "::scroll-button(prev)", "::scroll-button(next)", "::part()", ":state()",
    ":dir()", "::highlight()", ":host(.a, .b)", "::slotted(.a .b)",
    "::slotted(.a, .b)", ":host-context(.a .b)", ":-webkit-any()",
    "::cue(.a, .b)", "::cue(::before)", ":nth-child(2 of ::before)",
    ":nth-of-type(2 of .a)", "::part(a)::marker", "::slotted(.a)::marker",
    "::slotted(.a)::part(b)", "::part(a)::slotted(.b)", "::part(a)::before:hover",
    "::part(a)::-webkit-scrollbar:horizontal", "::view-transition-group(*):only-child",
    "[a=1]", '[a="1"]', "[ns|a]", "[*|a]", "[|a]", "[a=b i]", "[a=b s]",
    "[a=b x]", "#1a", ".1a", "a||b", "> .a", ".a >", ".a,", ":has(+ .a)",
    ":has(~ .a, > .b)", "::before:is(:hover)", ":is(.a)::part(b)",
    ".a::slotted(.b)", "*|*", "*|a.b", "A.B", "::part(a):focus-visible",
    "::-webkit-scrollbar:enabled", "::-webkit-scrollbar:focus",
    "::-webkit-input-placeholder:active", "::-webkit-input-placeholder:focus-within",
    "::-webkit-definitely-unknown:focus-visible", "::selection:window-inactive:hover",
    ":host::before", ":host(.a)::part(b)", ":is(.a) .b::before", ".a>.b+.c~.d",
  ];
  const selectors = [
    ...classes.map((name) => ":" + name),
    ...elements.map((name) => "::" + name),
    ...compound,
  ];
  const style = document.createElement("style");
  document.head.append(style);
  const outcome = (callback) => {
    try {
      return callback();
    } catch (error) {
      return error.name;
    }
  };
  const result = selectors.map((selector) => {
    style.textContent = `.p {} ${selector} {} .q {}`;
    const sheet = style.sheet;
    const parsed = Array.from(sheet.cssRules, (rule) => rule.selectorText);
    const inserted = outcome(() => {
      sheet.insertRule(`${selector} {}`, 0);
      return sheet.cssRules[0].selectorText;
    });
    const list = outcome(() => {
      sheet.insertRule(`.target, ${selector} {}`, 0);
      return sheet.cssRules[0].selectorText;
    });
    const assigned = outcome(() => {
      const rule = sheet.cssRules[sheet.cssRules.length - 1];
      rule.selectorText = selector;
      return rule.selectorText;
    });
    return {
      selector,
      parsed,
      inserted,
      list,
      assigned,
      supports: CSS.supports(`selector(${selector})`),
      query: outcome(() => (document.querySelector(selector), "ok")),
      matches: outcome(() => (document.body.matches(selector), "ok")),
    };
  });
  style.remove();
  return result;
})();
