// Chrome-156 control for turbo-dom's core DOM runtime edge cases (consolidation P5-cov).
//
// Records what Chrome for Testing 156 answers for the web-observable behaviour the P5-cov
// coverage tests assert, in three sections:
//   - `documents`: html5lib-format tree dumps (the format of vendor/html5lib-tests) of
//     `new DOMParser().parseFromString(html, "text/html")` (scripting off, no declarative
//     shadow roots) or, with a `context`, of `element.innerHTML = html` on an element of a
//     scripting-disabled `document.implementation.createHTMLDocument("")` document. The
//     cases target the UTF-16 tokenizer fork's states (NUL handling, attribute / comment /
//     DOCTYPE / character-reference edge cases, end-of-input in every state).
//   - `selectors`: per selector, whether `querySelector` accepts it and `el.matches()` for
//     every element in document order (as a 0/1 string), like
//     cascade_selectors_chrome156.json.
//   - `probes`: scripted DOM API checks (CharacterData, Attr nodes, insertAdjacentElement,
//     toggleAttribute, names and namespaces, `:*-of-type` across namespaces, serialization) with named results.
// Asserted by crates/turbo-dom/tests/rtdom_core_chrome.rs, which reproduces every case and
// probe through the rtdom Tree API (documents through both the upstream html5ever path and
// the UTF-16 tokenizer fork).
//
// Usage: node scripts/rtdom-core-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156
//            (~/Library/Caches/ms-playwright/chromium-1248/...);
//   --out    defaults to crates/turbo-dom/tests/fixtures/rtdom_core_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom/tests/fixtures/rtdom_core_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

// ---- documents: tokenizer / tree-builder inputs -------------------------------------------
const DOCUMENTS = [
  // NUL (U+0000) in every state that replaces or reports it
  { id: "rcdata_nul", html: "<title>a\0b</title>" },
  { id: "rawtext_nul", html: "<style>a\0b</style>" },
  { id: "script_data_nul", html: "<script>a\0b</script>" },
  { id: "plaintext_newlines_nul", html: "<plaintext>a<b>&amp;\r\nc\rd\0e" },
  { id: "tag_name_nul", html: "<a\0b>x</a\0b>" },
  { id: "before_attribute_name_nul", html: "<a \0=x>" },
  { id: "attribute_name_nul", html: "<a b\0c=x>" },
  { id: "after_attribute_name_nul", html: "<a b \0>" },
  { id: "attribute_value_double_quoted_nul", html: '<a b="x\0y">' },
  { id: "attribute_value_single_quoted_nul", html: "<a b='x\0y'>" },
  { id: "attribute_value_unquoted_nul", html: "<a b=x\0y>" },
  { id: "comment_start_nul", html: "<!--\0-->" },
  { id: "comment_start_dash_nul", html: "<!---\0-->" },
  { id: "comment_nul", html: "<!--a\0b-->" },
  { id: "comment_end_dash_nul", html: "<!--a-\0-->" },
  { id: "comment_end_bang_nul", html: "<!--a--!\0-->" },
  { id: "doctype_before_name_nul", html: "<!DOCTYPE \0>" },
  { id: "doctype_name_nul", html: "<!DOCTYPE h\0x>" },
  { id: "doctype_public_double_nul", html: '<!DOCTYPE html PUBLIC "a\0b">' },
  { id: "doctype_public_single_nul", html: "<!DOCTYPE html PUBLIC 'a\0b'>" },
  { id: "bogus_doctype_nul", html: "<!DOCTYPE html x\0y>" },
  // attributes
  { id: "after_attribute_name_whitespace", html: "<a b  c>" },
  { id: "after_attribute_name_solidus", html: "<a b />" },
  { id: "after_attribute_name_equals", html: "<a b =x>" },
  { id: "after_attribute_name_quote", html: "<a b \"c>" },
  { id: "before_attribute_value_whitespace", html: '<a b= \n\t"c">' },
  { id: "before_attribute_value_gt", html: "<a b=>x" },
  { id: "attribute_value_newlines", html: '<a b="x\ny\r\nz" c=\'p\rq\'>' },
  { id: "attribute_value_unquoted_specials", html: "<a b=x\"y'z<w=v`u>" },
  { id: "after_attribute_value_quoted_missing_space", html: '<a b="x"c=d>' },
  { id: "attribute_value_char_refs", html: "<a b='&amp;x' c=&lt;y d=&notit;>" },
  { id: "eof_before_attribute_value", html: "<a b=" },
  { id: "eof_in_attribute_name", html: "<a b" },
  // comments
  { id: "comment_less_than_bang", html: "<!--a<!b-->" },
  { id: "comment_less_than_less_than", html: "<!--a<<b-->" },
  { id: "comment_less_than_bang_dash_other", html: "<!--a<!-b-->" },
  { id: "comment_nested_open", html: "<!--a<!--b-->" },
  { id: "comment_nested_open_close", html: "<!--a<!---->" },
  { id: "comment_nested_open_gt", html: "<!--a<!-->b" },
  { id: "comment_end_bang_dash", html: "<!--a--!-b-->" },
  { id: "comment_end_bang_gt", html: "<!--a--!>b" },
  { id: "eof_comment_less_than_bang", html: "<!--a<!" },
  { id: "eof_comment_less_than_bang_dash", html: "<!--a<!-" },
  { id: "eof_comment_less_than_bang_dash_dash", html: "<!--a<!--" },
  { id: "markup_declaration_bogus", html: "<!x>y<![CDATA[z]]>" },
  { id: "markup_declaration_eof_dash", html: "<!-" },
  // DOCTYPE
  { id: "doctype_uppercase_name", html: "<!DOCTYPE HTML>" },
  { id: "doctype_crlf_before_keyword", html: '<!DOCTYPE html\r\nPUBLIC "p" "s">' },
  { id: "doctype_eof_in_keyword", html: "<!DOCTYPE html PUBL" },
  { id: "doctype_public_double_gt", html: '<!DOCTYPE html PUBLIC "a>b' },
  { id: "doctype_public_single_gt", html: "<!DOCTYPE html PUBLIC 'a>b" },
  { id: "doctype_after_public_whitespace", html: '<!DOCTYPE html PUBLIC "a"  >' },
  { id: "doctype_after_system_bogus", html: '<!DOCTYPE html SYSTEM "a" x>' },
  { id: "doctype_after_system_whitespace", html: '<!DOCTYPE html SYSTEM "a"  >' },
  { id: "doctype_between_single_system", html: "<!DOCTYPE html PUBLIC \"a\" 'b'>" },
  { id: "doctype_between_whitespace_gt", html: '<!DOCTYPE html PUBLIC "a" \n>' },
  { id: "eof_before_doctype_name", html: "<!DOCTYPE " },
  { id: "eof_between_doctype_identifiers", html: '<!DOCTYPE html PUBLIC "a" ' },
  { id: "eof_bogus_doctype", html: "<!DOCTYPE html x" },
  { id: "eof_doctype_system_single", html: "<!DOCTYPE html SYSTEM 'a" },
  // quirks-mode decisions (compatMode) the DOCTYPE states feed
  { id: "doctype_trailing_space", html: "<!DOCTYPE html >" },
  { id: "doctype_lone_cr_before_keyword", html: '<!DOCTYPE html\rPUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" "s">' },
  { id: "doctype_public_strict_no_system", html: '<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN">' },
  { id: "doctype_public_strict_space_gt", html: '<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" >' },
  { id: "doctype_transitional_no_system", html: '<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01 Transitional//EN">' },
  { id: "doctype_transitional_empty_system", html: '<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01 Transitional//EN" "">' },
  { id: "doctype_transitional_single_empty_system", html: "<!DOCTYPE html PUBLIC '-//W3C//DTD HTML 4.01 Transitional//EN' ''>" },
  { id: "doctype_transitional_empty_system_table", html: '<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01 Transitional//EN" ""><p><table>' },
  { id: "doctype_frameset_empty_system", html: '<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01 Frameset//EN" ""><p><table>' },
  { id: "doctype_empty_public_and_system", html: '<!DOCTYPE html PUBLIC "" ""><p><table>' },
  { id: "doctype_system_then_space", html: '<!DOCTYPE html SYSTEM "about:legacy-compat" >' },
  { id: "doctype_public_empty", html: '<!DOCTYPE html PUBLIC "">' },
  { id: "doctype_public_space_system", html: '<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN"\n"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">' },
  // tags and end tags
  { id: "end_tag_open_gt", html: "a</>b" },
  { id: "eof_end_tag_open", html: "a</" },
  { id: "eof_tag_open", html: "a<" },
  // character references
  { id: "char_ref_named_prefix_semicolon", html: "x&a;y" },
  { id: "char_ref_bogus_name_space", html: "x&zz y" },
  { id: "char_ref_bogus_name_semicolon", html: "x&zzq;y" },
  { id: "eof_char_ref_bogus_name", html: "x&zzzz" },
  { id: "eof_char_ref_named_prefix", html: "x&noti" },
  { id: "eof_char_ref_numeric", html: "x&#x41" },
  { id: "eof_char_ref_octothorpe", html: "x&#" },
  { id: "char_ref_numeric_no_semicolon", html: "x&#65y&#x42 z" },
  { id: "char_ref_numeric_invalid", html: "&#0;&#xD800;&#x110000;&#x80;&#x81;&#1;&#xFFFE;&#99999999999;" },
  { id: "char_ref_hex_no_digits", html: "&#xg;&#g;&#X41;" },
  { id: "char_ref_numeric_limits", html: "&#x10FFFF;|&#x110000;|&#4294967393;|&#x100000061;|&#1114111;" },
  // tree builder hooks the UTF-16 path delegates to upstream
  { id: "declarative_shadow_root_not_attached", html: '<div><template shadowrootmode="open"><b>x</b></template></div>' },
  { id: "annotation_xml_integration_point", html: '<math><annotation-xml encoding="text/html"><div>x</div></annotation-xml></math>' },
  { id: "form_association", html: "<form id=f><table><input name=a></table></form>" },
  { id: "foreign_attributes_prefixed", html: '<svg xlink:href="u" xml:lang="en"><a xlink:title="t"/></svg>' },
  { id: "meta_charset", html: '<meta charset="utf-8"><p>x</p>' },
  // miaskiewicz/turbo-dom#104: the selected option's content is cloned into every
  // selectedcontent of a non-multiple select
  { id: "selectedcontent_clone", html: "<select><button><selectedcontent></selectedcontent></button><option>A</option><option selected>B</option></select>" },
  { id: "selectedcontent_first_option", html: "<select><button><selectedcontent></selectedcontent></button><option>A</option><option>B</option></select>" },
  { id: "selectedcontent_last_selected", html: "<select><button><selectedcontent></selectedcontent></button><option selected>A</option><option selected>B</option></select>" },
  { id: "selectedcontent_replaces_markup", html: "<select><button><selectedcontent>old<i>x</i></selectedcontent></button><option><b class=k>A</b> x<!--c--></option></select>" },
  { id: "selectedcontent_after_option", html: "<select><option selected>A</option><button><selectedcontent></selectedcontent></button></select>" },
  { id: "selectedcontent_multiple", html: "<select multiple><button><selectedcontent></selectedcontent></button><option selected>A</option></select>" },
  { id: "selectedcontent_in_selected_option", html: "<select><option>a<selectedcontent></selectedcontent></option></select>" },
  { id: "selectedcontent_in_other_option", html: "<select><option>a</option><option>b<selectedcontent></selectedcontent></option></select>" },
  { id: "selectedcontent_twice", html: "<select><button><selectedcontent></selectedcontent><selectedcontent></selectedcontent></button><option selected>A</option></select>" },
  { id: "selectedcontent_optgroup", html: "<select><button><selectedcontent></selectedcontent></button><optgroup><option selected>G</option></optgroup></select>" },
  { id: "selectedcontent_implied_end", html: "<select><button><selectedcontent></selectedcontent></button><option>A</option><option selected>B<option>C</select>" },
  { id: "selectedcontent_outside_select", html: "<div><selectedcontent></selectedcontent><option selected>A</option></div>" },
  { id: "selectedcontent_nested", html: "<select><button><span><selectedcontent></selectedcontent></span></button><option selected id=o>A</option></select>" },
  { id: "selectedcontent_no_option", html: "<select><button><selectedcontent>keep</selectedcontent></button></select>" },
  { id: "selectedcontent_disabled_first", html: "<select><button><selectedcontent></selectedcontent></button><option disabled>A</option><option>B</option></select>" },
  { id: "selectedcontent_all_disabled", html: "<select><button><selectedcontent>k</selectedcontent></button><option disabled>A</option></select>" },
  { id: "selectedcontent_disabled_optgroup", html: "<select><button><selectedcontent></selectedcontent></button><optgroup disabled><option>A</option></optgroup><option>B</option></select>" },
  { id: "selectedcontent_disabled_selected", html: "<select><button><selectedcontent></selectedcontent></button><option disabled selected>A</option><option>B</option></select>" },
  { id: "selectedcontent_datalist", html: "<select><button><selectedcontent></selectedcontent></button><datalist><option>D</option></datalist><option>B</option></select>" },
  { id: "selectedcontent_template_option", html: "<select><button><selectedcontent></selectedcontent></button><template><option selected>T</option></template><option>B</option></select>" },
  { id: "selectedcontent_wrapped_option", html: "<select><button><selectedcontent></selectedcontent></button><div><option selected>X</option></div></select>" },
  { id: "selectedcontent_uppercase", html: "<SELECT><BUTTON><SELECTEDCONTENT></SELECTEDCONTENT></BUTTON><OPTION SELECTED>A</OPTION></SELECT>" },
  { id: "script_end_tags", html: "<script>a</script><script>b</script>c" },
  // fragments (innerHTML in a scripting-disabled document)
  // miaskiewicz/turbo-dom#92: Chrome drops a literal U+0000 in an RCDATA-context
  // (textarea / title) fragment; the per-context matrix below pins every other context
  { id: "fragment_textarea_rcdata", context: "textarea", html: "a\0b</textarea>c" },
  { id: "fragment_style_rawtext", context: "style", html: "a\0b</style>c" },
  { id: "fragment_textarea_nul", context: "textarea", html: "a\0b" },
  { id: "fragment_textarea_char_ref_nul", context: "textarea", html: "a&#0;b" },
  { id: "fragment_textarea_end_tag", context: "textarea", html: "a\0b</textarea>c\0d" },
  { id: "fragment_textarea_char_ref_split", context: "textarea", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_textarea_newlines", context: "textarea", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_title_nul", context: "title", html: "a\0b" },
  { id: "fragment_title_char_ref_nul", context: "title", html: "a&#0;b" },
  { id: "fragment_title_end_tag", context: "title", html: "a\0b</title>c\0d" },
  { id: "fragment_title_char_ref_split", context: "title", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_title_newlines", context: "title", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_script_nul", context: "script", html: "a\0b" },
  { id: "fragment_script_char_ref_nul", context: "script", html: "a&#0;b" },
  { id: "fragment_script_end_tag", context: "script", html: "a\0b</script>c\0d" },
  { id: "fragment_script_char_ref_split", context: "script", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_script_newlines", context: "script", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_style_nul", context: "style", html: "a\0b" },
  { id: "fragment_style_char_ref_nul", context: "style", html: "a&#0;b" },
  { id: "fragment_style_end_tag", context: "style", html: "a\0b</style>c\0d" },
  { id: "fragment_style_char_ref_split", context: "style", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_style_newlines", context: "style", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_xmp_nul", context: "xmp", html: "a\0b" },
  { id: "fragment_xmp_char_ref_nul", context: "xmp", html: "a&#0;b" },
  { id: "fragment_xmp_end_tag", context: "xmp", html: "a\0b</xmp>c\0d" },
  { id: "fragment_xmp_char_ref_split", context: "xmp", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_xmp_newlines", context: "xmp", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_iframe_nul", context: "iframe", html: "a\0b" },
  { id: "fragment_iframe_char_ref_nul", context: "iframe", html: "a&#0;b" },
  { id: "fragment_iframe_end_tag", context: "iframe", html: "a\0b</iframe>c\0d" },
  { id: "fragment_iframe_char_ref_split", context: "iframe", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_iframe_newlines", context: "iframe", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_noembed_nul", context: "noembed", html: "a\0b" },
  { id: "fragment_noembed_char_ref_nul", context: "noembed", html: "a&#0;b" },
  { id: "fragment_noembed_end_tag", context: "noembed", html: "a\0b</noembed>c\0d" },
  { id: "fragment_noembed_char_ref_split", context: "noembed", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_noembed_newlines", context: "noembed", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_noframes_nul", context: "noframes", html: "a\0b" },
  { id: "fragment_noframes_char_ref_nul", context: "noframes", html: "a&#0;b" },
  { id: "fragment_noframes_end_tag", context: "noframes", html: "a\0b</noframes>c\0d" },
  { id: "fragment_noframes_char_ref_split", context: "noframes", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_noframes_newlines", context: "noframes", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_plaintext_nul", context: "plaintext", html: "a\0b" },
  { id: "fragment_plaintext_char_ref_nul", context: "plaintext", html: "a&#0;b" },
  { id: "fragment_plaintext_end_tag", context: "plaintext", html: "a\0b</plaintext>c\0d" },
  { id: "fragment_plaintext_char_ref_split", context: "plaintext", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_plaintext_newlines", context: "plaintext", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_noscript_nul", context: "noscript", html: "a\0b" },
  { id: "fragment_noscript_char_ref_nul", context: "noscript", html: "a&#0;b" },
  { id: "fragment_noscript_end_tag", context: "noscript", html: "a\0b</noscript>c\0d" },
  { id: "fragment_noscript_char_ref_split", context: "noscript", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_noscript_newlines", context: "noscript", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_div_nul", context: "div", html: "a\0b" },
  { id: "fragment_div_char_ref_nul", context: "div", html: "a&#0;b" },
  { id: "fragment_div_end_tag", context: "div", html: "a\0b</div>c\0d" },
  { id: "fragment_div_char_ref_split", context: "div", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_div_newlines", context: "div", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_select_nul", context: "select", html: "a\0b" },
  { id: "fragment_select_char_ref_nul", context: "select", html: "a&#0;b" },
  { id: "fragment_select_end_tag", context: "select", html: "a\0b</select>c\0d" },
  { id: "fragment_select_char_ref_split", context: "select", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_select_newlines", context: "select", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_td_nul", context: "td", html: "a\0b" },
  { id: "fragment_td_char_ref_nul", context: "td", html: "a&#0;b" },
  { id: "fragment_td_end_tag", context: "td", html: "a\0b</td>c\0d" },
  { id: "fragment_td_char_ref_split", context: "td", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_td_newlines", context: "td", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_svg_title_nul", context: "svg title", html: "a\0b" },
  { id: "fragment_svg_title_char_ref_nul", context: "svg title", html: "a&#0;b" },
  { id: "fragment_svg_title_end_tag", context: "svg title", html: "a\0b</title>c\0d" },
  { id: "fragment_svg_title_char_ref_split", context: "svg title", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_svg_title_newlines", context: "svg title", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_math_mi_nul", context: "math mi", html: "a\0b" },
  { id: "fragment_math_mi_char_ref_nul", context: "math mi", html: "a&#0;b" },
  { id: "fragment_math_mi_end_tag", context: "math mi", html: "a\0b</mi>c\0d" },
  { id: "fragment_math_mi_char_ref_split", context: "math mi", html: "x&am\0p;&not\0in;&#x4\u00001;y" },
  { id: "fragment_math_mi_newlines", context: "math mi", html: "a\r\0\nb\r\n\0c" },
  { id: "fragment_svg_cdata", context: "svg svg", html: "<![CDATA[x]]y]]z]]]>w" },
  { id: "fragment_svg_cdata_eof", context: "svg svg", html: "<![CDATA[x]" },
  { id: "fragment_svg_cdata_eof_bracket", context: "svg svg", html: "<![CDATA[x]]" },
  { id: "fragment_svg_cdata_nul", context: "svg svg", html: "<![CDATA[a\0b]]>" },
  { id: "fragment_selectedcontent", context: "div", html: "<select><button><selectedcontent></selectedcontent></button><option>A</option><option selected>B</option></select>" },
  { id: "fragment_html_frameset_comment", context: "html", html: "<frameset></frameset></html><!--x-->" },
];

// ---- selectors through the string and UTF-16 selector parsers ------------------------------
const SELECTORS = [
  {
    id: "pseudo_element_arguments",
    html: "<p class=a>x</p>",
    selectors: [
      "::view-transition-group(*)",
      "::view-transition-group(foo)",
      "::view-transition-group()",
      "::view-transition-group(a b)",
      "::view-transition-old(.x)",
      "::view-transition-new(*.a)",
      "::view-transition-image-pair(foo.a.b)",
      "::view-transition-group( foo )",
      "::view-transition-group(foo .a)",
      "::view-transition-group(*.a .b)",
      "::view-transition-group(foo.)",
      "::view-transition-group(--x)",
      "::view-transition-group(foo,bar)",
      "::view-transition-group(\"a\")",
      "::view-transition-group(\\31a)",
      "::view-transition-group(1a)",
      "::view-transition-old(foo . a)",
      "::view-transition-new(.1a)",
      "::view-transition-group(foo/**/.a)",
      "::slotted( div )",
      "::slotted(div span)",
      "::slotted(div>span)",
      "::slotted(.a:hover)",
      "::slotted(:not(a b))",
      "::slotted(:is(a, b))",
      "::slotted(:is(a b))",
      "::slotted(:has(a))",
      "::slotted(:nth-child(1 of a b))",
    ],
  },
  {
    id: "pseudo_element_chains",
    // a sub-pseudo-element after a pseudo-element (miaskiewicz/turbo-dom#195)
    html: "<p class=a>x</p>",
    selectors: [
      "::slotted(div)::before",
      "::slotted(div)::after",
      "::slotted(div)::marker",
      "::slotted(a)::placeholder",
      "::slotted(a)::file-selector-button",
      "::slotted(a)::file-selector-button:hover",
      "::slotted(a)::details-content::before",
      "::slotted(a):before",
      "::slotted(a)::before::marker",
      "::before::marker",
      ":after::marker",
      "::column::scroll-marker",
      "::part(x)::before",
      "::part(x):hover::before",
      "::part(x)::first-line",
      "::part(x)::selection:window-inactive",
      "::part(x)::-webkit-scrollbar",
      "::picker(select)::picker-icon",
      "::details-content::details-content::marker",
      "div::slotted(.a)::backdrop",
      "::slotted(x)::view-transition-new(a):only-child",
      "::view-transition-group(foo):only-child",
      "::slotted(div)::after:hover",
      "::slotted(div)::part(x)",
      "::before::after",
      "::before::marker::before",
      "::before::marker:hover",
      "::slotted(x):hover::before",
      "::slotted(x)::first-line",
      "::slotted(x)::selection",
      "::part(x)::part(y)",
      "::part(x)::slotted(y)",
      "::part(x)::before:hover",
      "::file-selector-button::before",
      "::scroll-marker-group::before",
      "::view-transition-group(foo):only-child:hover",
      "::before ::marker",
      "::slotted(x)::marker.a",
      "::marker::marker",
      "::column::before",
      "::slotted(x)::bogus",
      "::part(x)::picker(1)",
    ],
  },
  {
    id: "pseudo_classes_after_pseudo_elements",
    // the pseudo-classes Chrome 156 allows after each pseudo-element
    html: "<p class=a>x</p>",
    selectors: [
      "::before:hover",
      "::before:focus",
      "::before:focus-within",
      "::before:target",
      "::before:checked",
      "::before:disabled",
      "::before:enabled",
      "::before:required",
      "::before:open",
      "::before:defined",
      "::before:first-child",
      "::before:only-child",
      "::before:empty",
      "::before:window-inactive",
      "::before:horizontal",
      "::before:corner-present",
      "::before:state(x)",
      "::before:is(a)",
      "::before:where(:hover)",
      "::before:not(a)",
      "::before:has(a)",
      "::before:lang(en)",
      "::before:current",
      "::before:playing",
      "::before:target-current",
      "::marker:hover",
      "::marker:focus",
      "::marker:focus-within",
      "::marker:target",
      "::marker:checked",
      "::marker:disabled",
      "::marker:enabled",
      "::marker:required",
      "::marker:open",
      "::marker:defined",
      "::marker:first-child",
      "::marker:only-child",
      "::marker:empty",
      "::marker:window-inactive",
      "::marker:horizontal",
      "::marker:corner-present",
      "::marker:state(x)",
      "::marker:is(a)",
      "::marker:where(:hover)",
      "::marker:not(a)",
      "::marker:has(a)",
      "::marker:lang(en)",
      "::marker:current",
      "::marker:playing",
      "::marker:target-current",
      "::file-selector-button:hover",
      "::file-selector-button:focus",
      "::file-selector-button:focus-within",
      "::file-selector-button:target",
      "::file-selector-button:checked",
      "::file-selector-button:disabled",
      "::file-selector-button:enabled",
      "::file-selector-button:required",
      "::file-selector-button:open",
      "::file-selector-button:defined",
      "::file-selector-button:first-child",
      "::file-selector-button:only-child",
      "::file-selector-button:empty",
      "::file-selector-button:window-inactive",
      "::file-selector-button:horizontal",
      "::file-selector-button:corner-present",
      "::file-selector-button:state(x)",
      "::file-selector-button:is(a)",
      "::file-selector-button:where(:hover)",
      "::file-selector-button:not(a)",
      "::file-selector-button:has(a)",
      "::file-selector-button:lang(en)",
      "::file-selector-button:current",
      "::file-selector-button:playing",
      "::file-selector-button:target-current",
      "::selection:hover",
      "::selection:focus",
      "::selection:focus-within",
      "::selection:target",
      "::selection:checked",
      "::selection:disabled",
      "::selection:enabled",
      "::selection:required",
      "::selection:open",
      "::selection:defined",
      "::selection:first-child",
      "::selection:only-child",
      "::selection:empty",
      "::selection:window-inactive",
      "::selection:horizontal",
      "::selection:corner-present",
      "::selection:state(x)",
      "::selection:is(a)",
      "::selection:where(:hover)",
      "::selection:not(a)",
      "::selection:has(a)",
      "::selection:lang(en)",
      "::selection:current",
      "::selection:playing",
      "::selection:target-current",
      "::column:hover",
      "::column:focus",
      "::column:focus-within",
      "::column:target",
      "::column:checked",
      "::column:disabled",
      "::column:enabled",
      "::column:required",
      "::column:open",
      "::column:defined",
      "::column:first-child",
      "::column:only-child",
      "::column:empty",
      "::column:window-inactive",
      "::column:horizontal",
      "::column:corner-present",
      "::column:state(x)",
      "::column:is(a)",
      "::column:where(:hover)",
      "::column:not(a)",
      "::column:has(a)",
      "::column:lang(en)",
      "::column:current",
      "::column:playing",
      "::column:target-current",
      "::scroll-marker:hover",
      "::scroll-marker:focus",
      "::scroll-marker:focus-within",
      "::scroll-marker:target",
      "::scroll-marker:checked",
      "::scroll-marker:disabled",
      "::scroll-marker:enabled",
      "::scroll-marker:required",
      "::scroll-marker:open",
      "::scroll-marker:defined",
      "::scroll-marker:first-child",
      "::scroll-marker:only-child",
      "::scroll-marker:empty",
      "::scroll-marker:window-inactive",
      "::scroll-marker:horizontal",
      "::scroll-marker:corner-present",
      "::scroll-marker:state(x)",
      "::scroll-marker:is(a)",
      "::scroll-marker:where(:hover)",
      "::scroll-marker:not(a)",
      "::scroll-marker:has(a)",
      "::scroll-marker:lang(en)",
      "::scroll-marker:current",
      "::scroll-marker:playing",
      "::scroll-marker:target-current",
      "::scroll-marker-group:hover",
      "::scroll-marker-group:focus",
      "::scroll-marker-group:focus-within",
      "::scroll-marker-group:target",
      "::scroll-marker-group:checked",
      "::scroll-marker-group:disabled",
      "::scroll-marker-group:enabled",
      "::scroll-marker-group:required",
      "::scroll-marker-group:open",
      "::scroll-marker-group:defined",
      "::scroll-marker-group:first-child",
      "::scroll-marker-group:only-child",
      "::scroll-marker-group:empty",
      "::scroll-marker-group:window-inactive",
      "::scroll-marker-group:horizontal",
      "::scroll-marker-group:corner-present",
      "::scroll-marker-group:state(x)",
      "::scroll-marker-group:is(a)",
      "::scroll-marker-group:where(:hover)",
      "::scroll-marker-group:not(a)",
      "::scroll-marker-group:has(a)",
      "::scroll-marker-group:lang(en)",
      "::scroll-marker-group:current",
      "::scroll-marker-group:playing",
      "::scroll-marker-group:target-current",
      "::scroll-button(up):hover",
      "::scroll-button(up):focus",
      "::scroll-button(up):focus-within",
      "::scroll-button(up):target",
      "::scroll-button(up):checked",
      "::scroll-button(up):disabled",
      "::scroll-button(up):enabled",
      "::scroll-button(up):required",
      "::scroll-button(up):open",
      "::scroll-button(up):defined",
      "::scroll-button(up):first-child",
      "::scroll-button(up):only-child",
      "::scroll-button(up):empty",
      "::scroll-button(up):window-inactive",
      "::scroll-button(up):horizontal",
      "::scroll-button(up):corner-present",
      "::scroll-button(up):state(x)",
      "::scroll-button(up):is(a)",
      "::scroll-button(up):where(:hover)",
      "::scroll-button(up):not(a)",
      "::scroll-button(up):has(a)",
      "::scroll-button(up):lang(en)",
      "::scroll-button(up):current",
      "::scroll-button(up):playing",
      "::scroll-button(up):target-current",
      "::details-content:hover",
      "::details-content:focus",
      "::details-content:focus-within",
      "::details-content:target",
      "::details-content:checked",
      "::details-content:disabled",
      "::details-content:enabled",
      "::details-content:required",
      "::details-content:open",
      "::details-content:defined",
      "::details-content:first-child",
      "::details-content:only-child",
      "::details-content:empty",
      "::details-content:window-inactive",
      "::details-content:horizontal",
      "::details-content:corner-present",
      "::details-content:state(x)",
      "::details-content:is(a)",
      "::details-content:where(:hover)",
      "::details-content:not(a)",
      "::details-content:has(a)",
      "::details-content:lang(en)",
      "::details-content:current",
      "::details-content:playing",
      "::details-content:target-current",
      "::picker(select):hover",
      "::picker(select):focus",
      "::picker(select):focus-within",
      "::picker(select):target",
      "::picker(select):checked",
      "::picker(select):disabled",
      "::picker(select):enabled",
      "::picker(select):required",
      "::picker(select):open",
      "::picker(select):defined",
      "::picker(select):first-child",
      "::picker(select):only-child",
      "::picker(select):empty",
      "::picker(select):window-inactive",
      "::picker(select):horizontal",
      "::picker(select):corner-present",
      "::picker(select):state(x)",
      "::picker(select):is(a)",
      "::picker(select):where(:hover)",
      "::picker(select):not(a)",
      "::picker(select):has(a)",
      "::picker(select):lang(en)",
      "::picker(select):current",
      "::picker(select):playing",
      "::picker(select):target-current",
      "::part(x):hover",
      "::part(x):focus",
      "::part(x):focus-within",
      "::part(x):target",
      "::part(x):checked",
      "::part(x):disabled",
      "::part(x):enabled",
      "::part(x):required",
      "::part(x):open",
      "::part(x):defined",
      "::part(x):first-child",
      "::part(x):only-child",
      "::part(x):empty",
      "::part(x):window-inactive",
      "::part(x):horizontal",
      "::part(x):corner-present",
      "::part(x):state(x)",
      "::part(x):is(a)",
      "::part(x):where(:hover)",
      "::part(x):not(a)",
      "::part(x):has(a)",
      "::part(x):lang(en)",
      "::part(x):current",
      "::part(x):playing",
      "::part(x):target-current",
      "::slotted(x):hover",
      "::slotted(x):focus",
      "::slotted(x):focus-within",
      "::slotted(x):target",
      "::slotted(x):checked",
      "::slotted(x):disabled",
      "::slotted(x):enabled",
      "::slotted(x):required",
      "::slotted(x):open",
      "::slotted(x):defined",
      "::slotted(x):first-child",
      "::slotted(x):only-child",
      "::slotted(x):empty",
      "::slotted(x):window-inactive",
      "::slotted(x):horizontal",
      "::slotted(x):corner-present",
      "::slotted(x):state(x)",
      "::slotted(x):is(a)",
      "::slotted(x):where(:hover)",
      "::slotted(x):not(a)",
      "::slotted(x):has(a)",
      "::slotted(x):lang(en)",
      "::slotted(x):current",
      "::slotted(x):playing",
      "::slotted(x):target-current",
      "::search-text:hover",
      "::search-text:focus",
      "::search-text:focus-within",
      "::search-text:target",
      "::search-text:checked",
      "::search-text:disabled",
      "::search-text:enabled",
      "::search-text:required",
      "::search-text:open",
      "::search-text:defined",
      "::search-text:first-child",
      "::search-text:only-child",
      "::search-text:empty",
      "::search-text:window-inactive",
      "::search-text:horizontal",
      "::search-text:corner-present",
      "::search-text:state(x)",
      "::search-text:is(a)",
      "::search-text:where(:hover)",
      "::search-text:not(a)",
      "::search-text:has(a)",
      "::search-text:lang(en)",
      "::search-text:current",
      "::search-text:playing",
      "::search-text:target-current",
      "::cue:hover",
      "::cue:focus",
      "::cue:focus-within",
      "::cue:target",
      "::cue:checked",
      "::cue:disabled",
      "::cue:enabled",
      "::cue:required",
      "::cue:open",
      "::cue:defined",
      "::cue:first-child",
      "::cue:only-child",
      "::cue:empty",
      "::cue:window-inactive",
      "::cue:horizontal",
      "::cue:corner-present",
      "::cue:state(x)",
      "::cue:is(a)",
      "::cue:where(:hover)",
      "::cue:not(a)",
      "::cue:has(a)",
      "::cue:lang(en)",
      "::cue:current",
      "::cue:playing",
      "::cue:target-current",
      "::cue(b):hover",
      "::cue(b):focus",
      "::cue(b):focus-within",
      "::cue(b):target",
      "::cue(b):checked",
      "::cue(b):disabled",
      "::cue(b):enabled",
      "::cue(b):required",
      "::cue(b):open",
      "::cue(b):defined",
      "::cue(b):first-child",
      "::cue(b):only-child",
      "::cue(b):empty",
      "::cue(b):window-inactive",
      "::cue(b):horizontal",
      "::cue(b):corner-present",
      "::cue(b):state(x)",
      "::cue(b):is(a)",
      "::cue(b):where(:hover)",
      "::cue(b):not(a)",
      "::cue(b):has(a)",
      "::cue(b):lang(en)",
      "::cue(b):current",
      "::cue(b):playing",
      "::cue(b):target-current",
      "::view-transition-old(a):hover",
      "::view-transition-old(a):focus",
      "::view-transition-old(a):focus-within",
      "::view-transition-old(a):target",
      "::view-transition-old(a):checked",
      "::view-transition-old(a):disabled",
      "::view-transition-old(a):enabled",
      "::view-transition-old(a):required",
      "::view-transition-old(a):open",
      "::view-transition-old(a):defined",
      "::view-transition-old(a):first-child",
      "::view-transition-old(a):only-child",
      "::view-transition-old(a):empty",
      "::view-transition-old(a):window-inactive",
      "::view-transition-old(a):horizontal",
      "::view-transition-old(a):corner-present",
      "::view-transition-old(a):state(x)",
      "::view-transition-old(a):is(a)",
      "::view-transition-old(a):where(:hover)",
      "::view-transition-old(a):not(a)",
      "::view-transition-old(a):has(a)",
      "::view-transition-old(a):lang(en)",
      "::view-transition-old(a):current",
      "::view-transition-old(a):playing",
      "::view-transition-old(a):target-current",
      "::-webkit-scrollbar:hover",
      "::-webkit-scrollbar:focus",
      "::-webkit-scrollbar:focus-within",
      "::-webkit-scrollbar:target",
      "::-webkit-scrollbar:checked",
      "::-webkit-scrollbar:disabled",
      "::-webkit-scrollbar:enabled",
      "::-webkit-scrollbar:required",
      "::-webkit-scrollbar:open",
      "::-webkit-scrollbar:defined",
      "::-webkit-scrollbar:first-child",
      "::-webkit-scrollbar:only-child",
      "::-webkit-scrollbar:empty",
      "::-webkit-scrollbar:window-inactive",
      "::-webkit-scrollbar:horizontal",
      "::-webkit-scrollbar:corner-present",
      "::-webkit-scrollbar:state(x)",
      "::-webkit-scrollbar:is(a)",
      "::-webkit-scrollbar:where(:hover)",
      "::-webkit-scrollbar:not(a)",
      "::-webkit-scrollbar:has(a)",
      "::-webkit-scrollbar:lang(en)",
      "::-webkit-scrollbar:current",
      "::-webkit-scrollbar:playing",
      "::-webkit-scrollbar:target-current",
      "::-webkit-scrollbar-thumb:hover",
      "::-webkit-scrollbar-thumb:focus",
      "::-webkit-scrollbar-thumb:focus-within",
      "::-webkit-scrollbar-thumb:target",
      "::-webkit-scrollbar-thumb:checked",
      "::-webkit-scrollbar-thumb:disabled",
      "::-webkit-scrollbar-thumb:enabled",
      "::-webkit-scrollbar-thumb:required",
      "::-webkit-scrollbar-thumb:open",
      "::-webkit-scrollbar-thumb:defined",
      "::-webkit-scrollbar-thumb:first-child",
      "::-webkit-scrollbar-thumb:only-child",
      "::-webkit-scrollbar-thumb:empty",
      "::-webkit-scrollbar-thumb:window-inactive",
      "::-webkit-scrollbar-thumb:horizontal",
      "::-webkit-scrollbar-thumb:corner-present",
      "::-webkit-scrollbar-thumb:state(x)",
      "::-webkit-scrollbar-thumb:is(a)",
      "::-webkit-scrollbar-thumb:where(:hover)",
      "::-webkit-scrollbar-thumb:not(a)",
      "::-webkit-scrollbar-thumb:has(a)",
      "::-webkit-scrollbar-thumb:lang(en)",
      "::-webkit-scrollbar-thumb:current",
      "::-webkit-scrollbar-thumb:playing",
      "::-webkit-scrollbar-thumb:target-current",
      "::-webkit-resizer:hover",
      "::-webkit-resizer:focus",
      "::-webkit-resizer:focus-within",
      "::-webkit-resizer:target",
      "::-webkit-resizer:checked",
      "::-webkit-resizer:disabled",
      "::-webkit-resizer:enabled",
      "::-webkit-resizer:required",
      "::-webkit-resizer:open",
      "::-webkit-resizer:defined",
      "::-webkit-resizer:first-child",
      "::-webkit-resizer:only-child",
      "::-webkit-resizer:empty",
      "::-webkit-resizer:window-inactive",
      "::-webkit-resizer:horizontal",
      "::-webkit-resizer:corner-present",
      "::-webkit-resizer:state(x)",
      "::-webkit-resizer:is(a)",
      "::-webkit-resizer:where(:hover)",
      "::-webkit-resizer:not(a)",
      "::-webkit-resizer:has(a)",
      "::-webkit-resizer:lang(en)",
      "::-webkit-resizer:current",
      "::-webkit-resizer:playing",
      "::-webkit-resizer:target-current",
      "::-webkit-inner-spin-button:hover",
      "::-webkit-inner-spin-button:focus",
      "::-webkit-inner-spin-button:focus-within",
      "::-webkit-inner-spin-button:target",
      "::-webkit-inner-spin-button:checked",
      "::-webkit-inner-spin-button:disabled",
      "::-webkit-inner-spin-button:enabled",
      "::-webkit-inner-spin-button:required",
      "::-webkit-inner-spin-button:open",
      "::-webkit-inner-spin-button:defined",
      "::-webkit-inner-spin-button:first-child",
      "::-webkit-inner-spin-button:only-child",
      "::-webkit-inner-spin-button:empty",
      "::-webkit-inner-spin-button:window-inactive",
      "::-webkit-inner-spin-button:horizontal",
      "::-webkit-inner-spin-button:corner-present",
      "::-webkit-inner-spin-button:state(x)",
      "::-webkit-inner-spin-button:is(a)",
      "::-webkit-inner-spin-button:where(:hover)",
      "::-webkit-inner-spin-button:not(a)",
      "::-webkit-inner-spin-button:has(a)",
      "::-webkit-inner-spin-button:lang(en)",
      "::-webkit-inner-spin-button:current",
      "::-webkit-inner-spin-button:playing",
      "::-webkit-inner-spin-button:target-current",
    ],
  },
  {
    id: "functional_pseudo_element_arguments",
    // ::picker() takes `select`, ::scroll-button() `*` or a direction, ::highlight() one
    // identifier, ::part() identifiers, ::cue() compound selectors
    html: "<p class=a>x</p>",
    selectors: [
      "::picker(1)",
      "::picker(foo)",
      "::picker(select)",
      "::picker( select )",
      "::picker(SELECT)",
      "::picker()",
      "::picker(a b)",
      "::picker(.a)",
      "::scroll-button(up)",
      "::scroll-button(*)",
      "::scroll-button(foo)",
      "::scroll-button(1)",
      "::scroll-button()",
      "::scroll-button(block-start)",
      "::scroll-button(up down)",
      "::highlight(1)",
      "::highlight(a b)",
      "::highlight()",
      "::highlight(--x)",
      "::cue(1)",
      "::cue()",
      "::cue(a b)",
      "::part(1)",
      "::part()",
      "::part(a b)",
      "::part(a,b)",
      "::part(--a)",
      "::slotted()",
      "::scroll-button(down)",
      "::scroll-button(left)",
      "::scroll-button(right)",
      "::scroll-button(block-end)",
      "::scroll-button(inline-start)",
      "::scroll-button(inline-end)",
      "::scroll-button(UP)",
      "::scroll-button( up )",
      "::scroll-button(prev)",
      "::scroll-button(next)",
      "::scroll-button(start)",
      "::cue(a, b)",
      "::cue(.x)",
      "::cue(:hover)",
      "::cue(a>b)",
      "::cue(b.x)",
      "::cue(*)",
      "::cue(:is(a b))",
      "::cue(::before)",
      "::highlight(a,b)",
      "::highlight( a )",
      "::highlight(\\31 a)",
      "::highlight(\"a\")",
      "::part( a  b )",
      "::part(\"a\")",
      "::part(a.b)",
      "::picker(select/**/)",
      "::picker(\\73 elect)",
      "::view-transition-group(a)::before",
    ],
  },
  {
    id: "attribute_operators",
    html: '<div id=a class="x y" lang="en-US" title="abc"></div><p id=b lang=en title=""></p><span data-v="a b"></span>',
    selectors: [
      "[class~=y]",
      "[class~=\"x\"]",
      "[lang|=en]",
      "[lang|=\"en\"]",
      "[title*=b]",
      "[title*=\"\"]",
      "[title^=a]",
      "[title$=c]",
      "[data-v~=\"a b\"]",
      "[class~=Y i]",
      "[lang|=EN i]",
      "[title*=B i]",
    ],
  },
  {
    id: "attribute_comments_and_escapes",
    html: '<div id=a title="a b"></div><p id=b title="x\\y"></p>',
    selectors: [
      "[title/* c */=\"a b\"]",
      "[/* c */title]",
      "[title=\"a b\"/* c */]",
      "[title/* unterminated",
      "[title=\"x\\\\y\"]",
      "[title=x\\\\y]",
      "[title=\"a\\\nb\"]",
      "[title=\"a\\\r\nb\"]",
      "[id=\\61]",
      "[title='a\\\fb']",
    ],
  },
  {
    id: "combinators_and_classes",
    html: "<div class=a></div><p class='b c'></p><span class=b></span><i class=d></i>",
    selectors: [
      "div + p",
      "div ~ span",
      "p+span",
      "p~i",
      ".a+.b",
      ".a~.d",
      "div,p",
      "div, ,p",
      ".b.c",
      "+ p",
      "p >",
    ],
  },
  {
    // miaskiewicz/turbo-dom#91
    id: "comments_eof_and_webkit_any",
    html: "<p title=a class=a>x</p>",
    selectors: [
      "p /* c",
      "p/* c */",
      "[title",
      "[title=a",
      "[title=\"a\"/* c",
      ":state(x)",
      ":-webkit-any(p)",
      ":-webkit-any(:is(p))",
      ":-webkit-any(:not(p))",
      ":-webkit-any(p, :has(x))",
      ":-webkit-any(::before)",
      ":not(:-webkit-any(:hover))",
      ":is(:-webkit-any(:has(p)))",
    ],
  },
  {
    // miaskiewicz/turbo-dom#91 (follow-up): selector-list items, CSS comments, EOF closing,
    // escapes at EOF and the :-webkit-any() grammar
    id: "selector_syntax",
    html: "<div class=d><p title=a class=a id=x>x</p><span></span></div>",
    selectors: [
      ":not(p, )",
      ":not(p,,a)",
      ":not(,p)",
      ":is(p,,a)",
      ":where(p, )",
      ":nth-child(1 of p,)",
      ":has(p,)",
      ":has(,p)",
      "p\\",
      "p,",
      "p,,a",
      "div/**/p",
      "p/**/.a",
      "p /**/ .a",
      ".d/*x*/>p",
      "p,/*c*/span",
      "/* c */p",
      "p/*",
      "[ti/**/tle]",
      "[title/**/=a]",
      "[title=/**/a]",
      "[title=a/**/i]",
      "[title=a/**/b]",
      "#/**/x",
      "./**/a",
      ":is(div/**/p, p)",
      "p:/**/first-child",
      "[title=\"a/**/b\"]",
      "[title=\"a",
      "[title='a",
      "[title=\"a\" i",
      ":nth-child(2n+1",
      ":not(p",
      "p:has(span",
      ":lang(en",
      ":is(p, :where(span",
      ":-webkit-any(div p)",
      ":-webkit-any(div > p)",
      ":-webkit-any()",
      ":-webkit-any(p, )",
      ":-webkit-any(*)",
      ":-webkit-any(:-webkit-any(p))",
      ":-webkit-any(p, :hover)",
      ":-webkit-any(.a, span)",
      ":-webkit-any(p",
      ":-webkit-any(p:not(.b))",
      ":-webkit-any(:is(div p))",
      ":-WEBKIT-ANY(p)",
      "p:-webkit-any(.a)",
      "div :-webkit-any(p)",
      ":-webkit-any(:not(div p))",
      ":-webkit-any(:not(div))",
      ":-webkit-any(:is(p, div p))",
      ":-webkit-any(:where(div p))",
      ":-webkit-any(:nth-child(1 of div p))",
      ":-webkit-any(:not(:has(p)))",
      ":-webkit-any(p::before)",
      ":-webkit-any(p,,a)",
      ":not(:-webkit-any(div p))",
      ":is(:-webkit-any(div p), p)",
      ":-webkit-any(:is(:has(p)))",
      ":-webkit-any(*|p)",
      ":-webkit-any([title])",
      ":-webkit-any(p.a#x)",
      ":-webkit-any(:-webkit-any(div p))",
      ":-webkit-any(:not(:is(div p)))",
      ":-webkit-any(:scope)",
      ":-webkit-any(:nth-child(1 of :is(div p)))",
    ],
  },
  {
    // P5-cov: strict-lexer, An+B, unforgiving-list and UTF-16 parser edges
    id: "parser_edges",
    html: "<div><p id=x title=ab></p><span></span><p></p></div>",
    selectors: [
      "p:state",
      ":state",
      ":nth-child(foo of p)",
      ":nth-child(99999999999999999999)",
      ":nth-child(-99999999999999999999n+1)",
      ":not(p >)",
      ":is(p >, span)",
      ":not(!)",
      ":nth-child(1 of !)",
      ":has(!)",
      ":not(\\70, )",
      ":nth-child(1 of \\70,)",
      ":has(+ \\73pan)",
      ":has(~ \\70)",
      ":has(> \\70)",
      ":state(a\\\nb)",
      ":state(\\61\r\n)",
      ":state(\\61 )",
      ":state(\\)",
      "[title=\"a\\62",
      "[id=x\\",
      "#x\\",
    ],
  },
  {
    // P5-cov: An+B integers clamp to 32 bits, a "-digits" glued to `n` must fit, and an
    // A or B outside 31 bits never matches (Chrome 156)
    id: "anb_limits",
    html: "<div><i></i><i></i><i></i><i></i><i></i></div>",
    selectors: [
      ":nth-child(-2147483648n+1)",
      ":nth-child(-2147483649n+1)",
      ":nth-child(-2147483647n+1)",
      ":nth-child(2147483648)",
      ":nth-child(2147483647)",
      ":nth-child(-99999999999999999999n+3)",
      ":nth-child(99999999999999999999n+1)",
      ":nth-child(n+99999999999999999999)",
      ":nth-child(-n+99999999999999999999)",
      ":nth-child(-n-99999999999999999999)",
      ":nth-child(2147483647n+1)",
      ":nth-child(-2147483648)",
      ":nth-child(99999999999999999999n-99999999999999999998)",
      ":nth-child(-1000000n+1)",
      ":nth-child(1000000n+1)",
      ":nth-child(-n+1000000000)",
      ":nth-child(1073741823n+1)",
      ":nth-child(1073741824n+1)",
      ":nth-child(-1073741824n+1)",
      ":nth-child(-1073741825n+1)",
      ":nth-child(n+1073741823)",
      ":nth-child(n+1073741824)",
      ":nth-child(-n+1073741823)",
      ":nth-child(-n-2147483648)",
      ":nth-child(-n-2147483649)",
      ":nth-child(n-2147483648)",
      ":nth-child(n-2147483649)",
      ":nth-child(n- 2147483649)",
      ":nth-child(n -2147483649)",
      ":nth-child(2n-0000000000000000000001)",
      ":nth-of-type(1073741824n+1)",
      ":nth-last-child(-n+99999999999999999999)",
      ":nth-child(1073741824n+1 of i)",
    ],
  },
  {
    // P5-cov: leading combinators in selector-list arguments and :has() nesting through
    // every list-taking pseudo
    id: "nested_lists",
    html: "<div><p></p><span></span><p></p></div>",
    selectors: [
      ":not(> p)",
      ":not(>p)",
      ":not(+ p)",
      ":nth-child(1 of > p)",
      ":nth-last-child(1 of ~ p)",
      ":is(> p, p)",
      ":where(+ p)",
      ":is(> \\70, p)",
      ":not(\\3e p)",
      ":-webkit-any(:nth-child(1 of :not(div p)))",
      ":-webkit-any(:nth-child(1 of div p))",
      ":has(:is(:has(p)))",
      ":has(:not(:has(p)))",
      ":has(:nth-child(1 of :has(p)))",
      ":has(:nth-last-child(1 of :has(p)))",
      ":has(:where(:has(p)))",
      ":has(:-webkit-any(:has(p)))",
      ":has(:is(:not(:has(p))))",
      ":has(:is(:where(:has(p))))",
      ":has(:is(:nth-child(1 of :has(p))))",
      ":has(:is(:nth-last-child(1 of :has(p))))",
      ":has(:is(:-webkit-any(p)))",
      ":has(:is(:not(p)))",
      ":has(:is(:nth-child(1 of p)))",
    ],
  },
  {
    id: "pseudo_edge_cases",
    html: "<div><p id=x></p><p></p></div>",
    selectors: [
      ":not(:has(p))",
      ":is(:has(p))",
      ":where(:has(p))",
      ":nth-child(1 of :has(p))",
      ":nth-last-child(1 of :has(p))",
      ":-webkit-any(:has(p))",
      ":has(:has(p))",
      "p:nth-child(2n+1 of #x)",
      "p:unknown-pseudo",
      "p:hover(x)",
      "p::before",
      ":state(x",
      "p:nth-child(foo)",
    ],
  },
];

// ---- probes: scripted DOM API checks -------------------------------------------------------
// Each probe runs in a fresh scripting-disabled HTML document `doc`; results are named.
const PROBES = [
  {
    id: "character_data",
    script: (doc) => {
      const t = doc.createTextNode("hello");
      const p = doc.createElement("p");
      p.append(t);
      t.insertData(1, "XY");
      const afterInsert = t.data;
      t.deleteData(0, 2);
      const afterDelete = t.data;
      t.appendData("!");
      const afterAppend = t.data;
      t.replaceData(1, 3, "-");
      const afterReplace = t.data;
      const sub = t.substringData(1, 2);
      const tail = t.splitText(2);
      return {
        afterInsert,
        afterDelete,
        afterAppend,
        afterReplace,
        sub,
        head: t.data,
        tail: tail.data,
        tailIsNextSibling: t.nextSibling === tail,
        parentChildren: p.childNodes.length,
        comment: (() => {
          const c = doc.createComment("abc");
          c.appendData("d");
          c.deleteData(0, 1);
          return c.data;
        })(),
      };
    },
  },
  {
    id: "attr_nodes",
    script: (doc) => {
      const el = doc.createElement("div");
      const other = doc.createElement("span");
      const a = doc.createAttribute("title");
      a.value = "v";
      el.setAttributeNode(a);
      const again = el.setAttributeNode(a);
      const ns = doc.createAttributeNS("urn:x", "p:local");
      el.setAttributeNode(ns);
      let removeForeign;
      try {
        other.removeAttributeNode(a);
        removeForeign = "ok";
      } catch (e) {
        removeForeign = e.name;
      }
      a.textContent = "via-textContent";
      const afterTextContent = el.getAttribute("title");
      a.nodeValue = "via-nodeValue";
      const afterNodeValue = el.getAttribute("title");
      const detached = doc.createAttribute("data-d");
      detached.value = "dv";
      return {
        againIsSame: again === a,
        removeForeign,
        afterTextContent,
        afterNodeValue,
        localName: ns.localName,
        name: ns.name,
        prefix: ns.prefix,
        namespaceURI: ns.namespaceURI,
        htmlAttrNamespace: a.namespaceURI,
        htmlAttrPrefix: a.prefix,
        detachedValue: detached.value,
        detachedOwner: detached.ownerElement,
      };
    },
  },
  {
    id: "node_value_on_elements",
    script: (doc) => {
      const el = doc.createElement("div");
      el.textContent = "x";
      el.nodeValue = "ignored";
      return { text: el.textContent, nodeValue: el.nodeValue };
    },
  },
  {
    id: "insert_adjacent_element",
    script: (doc) => {
      const host = doc.createElement("div");
      host.append(doc.createElement("i"));
      const wrapper = doc.createElement("section");
      wrapper.append(host);
      const ids = [];
      for (const position of ["beforebegin", "afterbegin", "beforeend", "afterend", "AfterBegin"]) {
        const el = doc.createElement("b");
        el.id = position;
        ids.push(host.insertAdjacentElement(position, el) === el);
      }
      const detached = doc.createElement("div");
      const before = detached.insertAdjacentElement("beforebegin", doc.createElement("u"));
      const after = detached.insertAdjacentElement("afterend", doc.createElement("u"));
      let invalid;
      try {
        host.insertAdjacentElement("nowhere", doc.createElement("u"));
        invalid = "ok";
      } catch (e) {
        invalid = e.name;
      }
      return {
        inserted: ids,
        wrapper: wrapper.innerHTML,
        detachedBefore: before,
        detachedAfter: after,
        invalid,
      };
    },
  },
  {
    id: "toggle_attribute",
    script: (doc) => {
      const el = doc.createElement("div");
      const r = [];
      r.push(el.toggleAttribute("hidden"), el.hasAttribute("hidden"));
      r.push(el.toggleAttribute("hidden"), el.hasAttribute("hidden"));
      r.push(el.toggleAttribute("hidden", true), el.hasAttribute("hidden"));
      r.push(el.toggleAttribute("hidden", true), el.getAttribute("hidden"));
      r.push(el.toggleAttribute("hidden", false), el.hasAttribute("hidden"));
      r.push(el.toggleAttribute("hidden", false), el.hasAttribute("hidden"));
      return { sequence: r };
    },
  },
  {
    id: "names_and_namespaces",
    script: (doc) => {
      const upper = doc.createElement("DiV");
      const nullNs = doc.createElementNS(null, "thing");
      const custom = doc.createElementNS("urn:a", "p:thing");
      const parsed = new DOMParser().parseFromString('<svg><a xlink:href="u"></a></svg>', "text/html");
      const link = parsed.querySelector("a");
      return {
        upperLocalName: upper.localName,
        upperTagName: upper.tagName,
        nullNamespaceURI: nullNs.namespaceURI,
        nullTagName: nullNs.tagName,
        customTagName: custom.tagName,
        customPrefix: custom.prefix,
        parsedAttributeNames: link.getAttributeNames(),
        parsedAttributePrefix: link.attributes[0].prefix,
        parsedAttributeNamespace: link.attributes[0].namespaceURI,
        getAttributeNSMissing: doc.createElement("div").getAttributeNS("urn:x", "a"),
      };
    },
  },
  {
    id: "serialization",
    script: (doc) => {
      const div = doc.createElement("div");
      div.innerHTML = '<a title="a&nbsp;b">c&nbsp;d</a>';
      const holder = doc.createElement("div");
      holder.append(doc.createComment("c"), doc.createProcessingInstruction("t", "d"));
      return { nbsp: div.innerHTML, commentAndPi: holder.innerHTML };
    },
  },
  {
    id: "remove_attribute",
    script: (doc) => {
      const el = doc.createElement("div");
      el.setAttribute("a", "1");
      el.setAttributeNS("urn:x", "p:b", "2");
      el.removeAttribute("missing");
      el.removeAttribute("p:b");
      return { names: el.getAttributeNames() };
    },
  },
  {
    id: "of_type_namespaces",
    script: (doc) => {
      const host = doc.createElement("div");
      const kids = [
        doc.createElement("rect"),
        doc.createElementNS("http://www.w3.org/2000/svg", "rect"),
        doc.createElementNS("urn:a", "x"),
        doc.createElementNS("urn:b", "x"),
        doc.createElementNS("urn:a", "x"),
        doc.createElementNS(null, "x"),
      ];
      host.append(...kids);
      return {
        firstOfType: kids.map((k) => k.matches(":first-of-type")),
        lastOfType: kids.map((k) => k.matches(":last-of-type")),
      };
    },
  },
  {
    // miaskiewicz/turbo-dom#96: `:scope` is the element matches()/closest()/
    // element.querySelectorAll() are called on, and the document element for document queries
    id: "scope",
    script: (doc) => {
      doc.body.innerHTML = "<div id=d><p id=p><span id=s></span></p><p id=q></p></div>";
      const d = doc.getElementById("d"),
        p = doc.getElementById("p"),
        s = doc.getElementById("s");
      const ids = (list) => [...list].map((e) => e.id || e.localName);
      const id = (e) => (e ? e.id || e.localName : null);
      return {
        documentScope: ids(doc.querySelectorAll(":scope")),
        documentChild: ids(doc.querySelectorAll(":scope > body")),
        elementChild: ids(d.querySelectorAll(":scope > p")),
        elementSelf: ids(d.querySelectorAll(":scope")),
        elementNested: ids(d.querySelectorAll(":scope p span, :is(:scope) > p")),
        elementFirst: id(d.querySelector(":scope > p ~ p")),
        matches: [
          p.matches(":scope"),
          p.matches("div > :scope"),
          p.matches(":scope > span"),
          p.matches(":is(:scope)"),
          p.matches(":not(:scope)"),
          p.matches(":has(> :scope)"),
          d.matches(":has(:scope)"),
          d.matches(":has(> p:not(:scope))"),
        ],
        closest: [
          id(s.closest(":scope")),
          id(s.closest("div:scope")),
          id(s.closest(":scope span")),
          id(s.closest("div :scope")),
          id(s.closest(":not(:scope)")),
        ],
      };
    },
  },
  {
    // attribute values holding lone surrogates: string selectors match their scalar
    // (U+FFFD) reading only where the original units agree
    id: "lone_unit_attributes",
    script: (doc) => {
      const el = doc.createElement("div");
      el.setAttribute("class", "b \uD800");
      el.setAttribute("lang", "en-\uDC00");
      el.setAttribute("title", "x\uD800y");
      const m = (s) => el.matches(s);
      return {
        includes: m("[class~=b]"),
        includesReplacement: m('[class~="\uFFFD"]'),
        className: m(".b"),
        classReplacement: m(".\uFFFD"),
        dash: m("[lang|=en]"),
        substr: m("[title*=x]"),
        substrReplacement: m('[title*="\uFFFD"]'),
        equalsReplacement: m('[title="x\uFFFDy"]'),
        prefix: m("[title^=x]"),
        suffix: m("[title$=y]"),
        presence: m("[title]"),
      };
    },
  },
  {
    // the fragment parser's form pointer is the context's nearest HTML form ancestor
    id: "contextual_fragment_form",
    script: (doc) => {
      const range = doc.createRange();
      const fragmentIn = (container) => {
        const context = doc.createElement("div");
        container.append(context);
        doc.body.append(container);
        range.selectNodeContents(context);
        const fragment = range.createContextualFragment("<form id=inner></form><b></b>");
        return [...fragment.childNodes].map((n) => n.localName);
      };
      return {
        insideForm: fragmentIn(doc.createElement("form")),
        insideSvgForm: fragmentIn(doc.createElementNS("http://www.w3.org/2000/svg", "form")),
        insideDiv: fragmentIn(doc.createElement("div")),
      };
    },
  },
  {
    id: "contextual_fragment_is",
    script: (doc) => {
      const range = doc.createRange();
      range.selectNodeContents(doc.body);
      const fragment = range.createContextualFragment('<div is="x-y"></div><p></p><svg is="x-y"></svg>');
      return {
        isDefined: fragment.firstChild.matches(":defined"),
        plainDefined: fragment.childNodes[1].matches(":defined"),
        svgIsDefined: fragment.lastChild.matches(":defined"),
      };
    },
  },
  {
    // a `dir=auto` textarea's direction comes from its value, whose default is the child
    // text content (direct Text children only; miaskiewicz/turbo-dom#106)
    id: "textarea_dir_auto",
    script: (doc) => {
      const textarea = (...children) => {
        const el = doc.createElement("textarea");
        el.setAttribute("dir", "auto");
        doc.body.append(el);
        for (const [tag, text] of children) {
          if (tag) {
            const child = doc.createElement(tag);
            child.textContent = text;
            el.append(child);
          } else {
            el.append(text);
          }
        }
        return [el.matches(":dir(rtl)"), el.matches(":dir(ltr)")];
      };
      return {
        empty: textarea(),
        hebrew: textarea([null, "א"]),
        digits: textarea([null, "123"]),
        digitsThenHebrew: textarea([null, "123"], [null, "א"]),
        latinThenHebrew: textarea([null, "a"], [null, "א"]),
        spanChild: textarea(["span", "א"]),
        bdiChild: textarea(["bdi", "א"]),
        spanThenHebrew: textarea(["span", "a"], [null, "א"]),
      };
    },
  },
];

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage();
await page.setContent("<!DOCTYPE html><html><head></head><body></body></html>", { waitUntil: "domcontentloaded" });

const documents = await page.evaluate((cases) => {
  const dump = (root) => {
    const lines = [];
    const indent = (depth) => `| ${"  ".repeat(depth)}`;
    const walk = (node, depth) => {
      switch (node.nodeType) {
        case Node.ELEMENT_NODE: {
          const ns =
            node.namespaceURI === "http://www.w3.org/2000/svg"
              ? "svg "
              : node.namespaceURI === "http://www.w3.org/1998/Math/MathML"
                ? "math "
                : "";
          lines.push(`${indent(depth)}<${ns}${node.localName}>`);
          const attrs = [...node.attributes]
            .map((a) => [a.prefix ? `${a.prefix} ${a.localName}` : a.localName, a.value])
            .sort((x, y) => (x[0] < y[0] ? -1 : x[0] > y[0] ? 1 : 0));
          for (const [name, value] of attrs) lines.push(`${indent(depth + 1)}${name}="${value}"`);
          if (ns === "" && node.localName === "template") {
            lines.push(`${indent(depth + 1)}content`);
            for (const child of node.content.childNodes) walk(child, depth + 2);
          }
          for (const child of node.childNodes) walk(child, depth + 1);
          break;
        }
        case Node.TEXT_NODE:
          lines.push(`${indent(depth)}"${node.data}"`);
          break;
        case Node.COMMENT_NODE:
          lines.push(`${indent(depth)}<!-- ${node.data} -->`);
          break;
        case Node.DOCUMENT_TYPE_NODE:
          lines.push(
            node.publicId || node.systemId
              ? `${indent(depth)}<!DOCTYPE ${node.name} "${node.publicId}" "${node.systemId}">`
              : `${indent(depth)}<!DOCTYPE ${node.name}>`,
          );
          break;
        default:
          throw new Error(`unexpected node type ${node.nodeType}`);
      }
    };
    for (const child of root.childNodes) walk(child, 0);
    return lines.join("\n");
  };
  return cases.map((c) => {
    if (!c.context) {
      const doc = new DOMParser().parseFromString(c.html, "text/html");
      return { ...c, dump: dump(doc), compatMode: doc.compatMode };
    }
    const doc = document.implementation.createHTMLDocument("");
    const [prefix, local] = c.context.includes(" ") ? c.context.split(" ") : [null, c.context];
    const ns = { svg: "http://www.w3.org/2000/svg", math: "http://www.w3.org/1998/Math/MathML" }[prefix];
    const element = ns ? doc.createElementNS(ns, local) : doc.createElement(local);
    element.innerHTML = c.html;
    return { ...c, dump: dump(element) };
  });
}, DOCUMENTS);

const selectors = [];
for (const testCase of SELECTORS) {
  await page.setContent(`<!DOCTYPE html><html><head></head><body>${testCase.html}</body></html>`, {
    waitUntil: "domcontentloaded",
  });
  const recorded = await page.evaluate((list) => {
    const elements = [];
    const walk = (el) => {
      elements.push(el);
      for (const child of el.children) walk(child);
    };
    walk(document.documentElement);
    return {
      elements: elements.map((el) => el.localName),
      rows: list.map((selector) => {
        let valid = true;
        try {
          document.querySelector(selector);
        } catch {
          valid = false;
        }
        return {
          selector,
          valid,
          matches: valid ? elements.map((el) => (el.matches(selector) ? "1" : "0")).join("") : "",
        };
      }),
    };
  }, testCase.selectors);
  selectors.push({ id: testCase.id, html: testCase.html, elements: recorded.elements, selectors: recorded.rows });
}

const probes = [];
await page.setContent("<!DOCTYPE html><html><head></head><body></body></html>", { waitUntil: "domcontentloaded" });
for (const probe of PROBES) {
  const values = await page.evaluate(
    `(${probe.script.toString()})(document.implementation.createHTMLDocument(""))`,
  );
  probes.push({ id: probe.id, values });
}
await browser.close();

const fixture = {
  generator: "scripts/rtdom-core-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  documents,
  selectors,
  probes,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(
  `Chrome ${version}: ${documents.length} documents, ${selectors.reduce((n, c) => n + c.selectors.length, 0)} selectors, ` +
    `${probes.length} probes -> ${out}`,
);
