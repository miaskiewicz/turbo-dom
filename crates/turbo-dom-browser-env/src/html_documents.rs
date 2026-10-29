//! Native parsed-document default sanitizer: `Document.parseHTML` without options
//! filters with the built-in safe default configuration, a factual snapshot of
//! Chrome's public `new Sanitizer().get()` (browser_sanitizer_default.json,
//! re-verified against Chrome 156 by sanitizer_chrome156.json).
pub(super) fn default_config() -> &'static super::sanitizer::Config {
    super::sanitizer::Config::builtin_default()
}
#[cfg(test)]
mod tests {
    use super::*;
    use turbo_dom_parser::rtdom::tree::Tree;
    #[test]
    fn default_policy_filters_parsed_namespaces_attributes_and_script_urls() {
        let mut tree=Tree::parse("<html><head><title>safe</title><script>evil()</script></head><body><p id='drop' onclick='evil()'><a href='&#x6a;av&#x09;ascript:evil()'>link</a></p><iframe src='about:blank'></iframe><img src='data:image/png,x'><style>p{color:red}</style><svg onload='evil()'><a href='javascript:evil()'>vector</a></svg><math><mi onclick='evil()'>x</mi></math><!--drop--><unknown><b>drop</b></unknown><template><script>drop</script></template></body></html>");
        let root = tree.root();
        let _ = super::super::sanitizer::sanitize(&mut tree, root, default_config(), true);
        let html = turbo_dom_parser::rtdom::serialize::serialize_inner(&tree, tree.root());
        assert!(!html.contains("script"));
        assert!(!html.contains("iframe"));
        assert!(!html.contains("<img"));
        assert!(!html.contains("<style"));
        assert!(!html.contains("drop"));
        assert!(!html.contains("onclick"));
        assert!(!html.contains("onload"));
        assert!(!html.contains("javascript"));
        assert!(html.contains("<title>safe</title>"), "{html}");
        assert!(html.contains("<p><a>link</a></p>"));
        assert!(html.contains("<svg><a>vector</a></svg>"));
        assert!(html.contains("<math><mi>x</mi></math>"));
    }
}
