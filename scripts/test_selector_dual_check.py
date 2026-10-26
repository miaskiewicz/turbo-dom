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


if __name__ == "__main__":
    unittest.main()
