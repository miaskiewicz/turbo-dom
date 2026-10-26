"""Unit tests for scripts/selector-dual-check.py's Gate B classifier.

Run: python3 -m unittest discover -s scripts -p 'test_selector_dual_check.py'
Each bug-list classifier must accept a divergence only when BOTH the selector feature and
the element-side condition hold.
"""
import importlib.util
import os
import unittest

_spec = importlib.util.spec_from_file_location(
    "selector_dual_check", os.path.join(os.path.dirname(__file__), "selector-dual-check.py")
)
check = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(check)


def element(tokens=(), **facts):
    base = {"has_parent": True, "empty_ignorable": False, "link": False, "rw_differs": False}
    base.update(facts)
    base["tokens"] = list(tokens)
    return {"tag": "div", "facts": base}


def record(selector, *elements, api="matches", legacy=False, new=True):
    return {"type": "divergence", "api": api, "selector": selector, "legacy": legacy,
            "new": new, "elements": list(elements)}


class Classifier(unittest.TestCase):
    def test_escapes_need_the_decoded_identifier_on_the_element(self):
        self.assertEqual(check.classify(record(".md\\:flex", element(["md:flex"]))), "escapes")
        self.assertEqual(check.classify(record("#\\31 23", element(["123"]))), "escapes")
        self.assertIsNone(check.classify(record(".md\\:flex", element(["md", "flex"]))))
        self.assertIsNone(check.classify(record(".md", element(["md"]))))

    def test_validity_changes_need_the_feature_character(self):
        rec = record(".md\\:flex", api="matches_checked",
                     legacy={"err": "unexpected '\\' in selector"}, new={"ok": True})
        self.assertEqual(check.classify(rec), "escapes")
        rec = record(".x", api="matches_checked",
                     legacy={"err": "unexpected '\\' in selector"}, new={"ok": True})
        self.assertIsNone(check.classify(rec))
        rec = record("*|p", api="query_selector_all_checked",
                     legacy={"err": "unexpected '|' in selector"}, new={"ok": []})
        self.assertEqual(check.classify(rec), "namespace *|p")
        rec = record(".café", api="matches_checked",
                     legacy={"err": "unexpected 'é' in selector"}, new={"ok": False})
        self.assertEqual(check.classify(rec), "non-ascii")

    def test_non_ascii_identifier(self):
        self.assertEqual(check.classify(record(".日本", element(["日本"]))), "non-ascii")
        self.assertIsNone(check.classify(record(".日本", element(["x"]))))

    def test_namespace_prefix_needs_the_named_type(self):
        self.assertEqual(check.classify(record("div > *|p", element(["p"]))), "namespace *|p")
        self.assertIsNone(check.classify(record("div > *|p", element(["span"]))))
        # a `|` inside an attribute selector is not a namespace prefix
        self.assertIsNone(check.classify(record("[lang|=en]", element(["en"]))))

    def test_pseudo_classes_need_their_element_condition(self):
        self.assertEqual(check.classify(record("div:empty", element(empty_ignorable=True))), ":empty")
        self.assertIsNone(check.classify(record("div:empty", element())))
        self.assertEqual(check.classify(record(":read-write", element(rw_differs=True))),
                         ":read-only/:read-write")
        self.assertIsNone(check.classify(record(":read-write", element())))
        self.assertEqual(check.classify(record("a:link", element(link=True))), ":link/:any-link")
        self.assertIsNone(check.classify(record("a:link", element())))
        self.assertEqual(check.classify(record(":nth-child(2 of .x)", element())),
                         ":nth-child(An+B of S)")
        self.assertIsNone(check.classify(record(":nth-child(2 of .x)", element(has_parent=False))))
        self.assertIsNone(check.classify(record(":nth-child(2n+1)", element())))

    def test_every_recorded_element_must_satisfy_the_condition(self):
        rec = record(":empty", element(empty_ignorable=True), element(), api="query_selector_all",
                     legacy=[], new=[1, 2])
        self.assertIsNone(check.classify(rec))

    def test_unrelated_divergence_is_unclassified(self):
        self.assertIsNone(check.classify(record("p:first-child", element(["p"]))))
        self.assertIsNone(check.classify(record("p", api="query_selector_all", legacy=[], new=[3])))


def cascade_element(tag="div", attrs=(), local_attrs=None, ancestors=()):
    attrs = [list(a) for a in attrs]
    return {"tag": tag, "attrs": attrs,
            "local_attrs": [list(a) for a in local_attrs] if local_attrs is not None else attrs,
            "facts": {"shell": False, "marker": False, "scope_root": False,
                      "position": [1, 1, 1, 1], "kept_children": [], "ancestors": list(ancestors)}}


def cascade(selector, el=None, api="cascade_match", legacy=True, new=False):
    rec = {"type": "divergence", "engine": "cascade", "api": api, "legacy": legacy, "new": new}
    if api in ("cascade_match", "cascade_specificity"):
        rec["selector"] = selector
    else:
        rec["prelude"] = selector
    if el is not None:
        rec["element"] = el
    return rec


class CascadeClassifier(unittest.TestCase):
    """S3b: old html2pdf engine vs the cascade engine (turbo-dom-layout records)."""

    def test_live_tree_records_are_never_classified_at_s3b(self):
        self.assertIsNone(check.classify_cascade(record(".md\\:flex", element(["md:flex"]))))

    def test_escapes_new_matches_need_the_decoded_identifier(self):
        self.assertEqual(check.classify_cascade(
            cascade(".md\\:flex", cascade_element(attrs=[("class", "md:flex")]), legacy=False, new=True)),
            "escapes")
        self.assertIsNone(check.classify_cascade(
            cascade(".md\\:flex", cascade_element(attrs=[("class", "md")]), legacy=False, new=True)))
        self.assertEqual(check.classify_cascade(cascade("\\70", cascade_element("span"))), "escapes")
        # escapes outside the subject compound, or negated, constrain nothing on the element
        self.assertEqual(check.classify_cascade(
            cascade(":is(.md\\:flex) p", cascade_element("p"), legacy=False, new=True)), "escapes")
        self.assertEqual(check.classify_cascade(
            cascade(":not(.md\\:flex)", cascade_element("span"), legacy=False, new=True)), "escapes")

    def test_namespace_prefixes(self):
        self.assertEqual(check.classify_cascade(cascade("*|p", cascade_element("p"), legacy=False, new=True)),
                         "namespace *|p")
        self.assertIsNone(check.classify_cascade(cascade("*|p", cascade_element("div"), legacy=False, new=True)))
        self.assertEqual(check.classify_cascade(cascade("svg|rect", api="cascade_validity")), "namespace *|p")
        self.assertIsNone(check.classify_cascade(cascade("svg rect", api="cascade_validity")))

    def test_attribute_namespace_needs_a_namespaced_attribute(self):
        el = cascade_element("a", attrs=[("xlink:href", "#t")], local_attrs=[("href", "#t")])
        self.assertEqual(check.classify_cascade(cascade('a[href^="#"]', el)), "attribute-namespace")
        plain = cascade_element("a", attrs=[("href", "#t")])
        self.assertIsNone(check.classify_cascade(cascade('a[href^="#"]', plain)))
        self.assertIsNone(check.classify_cascade(cascade("[title]", el)))
        self.assertIsNone(check.classify_cascade(cascade("[href]", el, legacy=False, new=True)))

    def test_pseudo_classes_the_old_engine_dropped(self):
        for sel, name in [(":read-only", ":read-only/:read-write"), ("input:optional", ":required/:optional"),
                          ("p:nth-last-child(2)", ":nth-last-*"), (":nth-child(2 of .x)", ":nth-child(An+B of S)"),
                          ("div:has(> p)", ":has()"), (":lang(en)", "unknown-pseudo"),
                          ("P:FIRST-CHILD", "unknown-pseudo"), ("a:totallyunknown", "unknown-pseudo")]:
            self.assertEqual(check.classify_cascade(cascade(sel, cascade_element())), name, sel)
        self.assertEqual(check.classify_cascade(cascade(":matches(p), .w", api="cascade_validity")),
                         "unknown-pseudo")
        self.assertIsNone(check.classify_cascade(cascade("p:first-child", cascade_element())))
        self.assertIsNone(check.classify_cascade(cascade("[data-x=':lang(en)']", cascade_element())))

    def test_pseudo_elements_never_match(self):
        self.assertEqual(check.classify_cascade(cascade(".bar::-webkit-scrollbar", cascade_element())),
                         "pseudo-element")
        self.assertEqual(check.classify_cascade(
            cascade("*::before", api="cascade_specificity", legacy=[0, 1, 0], new=[0, 0, 1])), "pseudo-element")
        self.assertIsNone(check.classify_cascade(cascade(".bar::after", cascade_element(), legacy=False, new=True)))

    def test_specificity_needs_a_functional_pseudo(self):
        self.assertEqual(check.classify_cascade(
            cascade("a:where(.new)", api="cascade_specificity", legacy=[0, 1, 1], new=[0, 0, 1])), "specificity")
        self.assertIsNone(check.classify_cascade(
            cascade("a.new", api="cascade_specificity", legacy=[0, 1, 1], new=[0, 0, 1])))

    def test_unrelated_cascade_divergence_is_unclassified(self):
        self.assertIsNone(check.classify_cascade(cascade("div > p", cascade_element("p"))))
        self.assertIsNone(check.classify_cascade(cascade("a, b", api="cascade_list", legacy=2, new=1)))


if __name__ == "__main__":
    unittest.main()
