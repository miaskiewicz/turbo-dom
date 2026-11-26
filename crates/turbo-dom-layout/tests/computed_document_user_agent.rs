use turbo_dom_layout::style::{computed_document_user_agent_css, user_agent_css};
#[test]
fn renderer_source_stays_byte_identical_and_computed_document_omits_only_its_preference() {
    let renderer = include_str!("fixtures/renderer-user-agent-b31d.css");
    assert_eq!(user_agent_css(), renderer);
    let preference = "body { font-family: sans-serif }\n";
    assert_eq!(renderer.matches(preference).count(), 1);
    assert_eq!(
        computed_document_user_agent_css(),
        renderer.replacen(preference, "", 1)
    );
    assert!(computed_document_user_agent_css()
        .contains("code, kbd, samp, pre { font-family: monospace }"));
}
