use turbo_dom::rtdom::{
    tree::{FormProp, Handle},
    Tree,
};
#[test]
fn live_checkedness_does_not_change_default_attributes_and_invalidates_queries() {
    let mut tree=Tree::parse("<input id=c type=checkbox checked><input id=r type=radio checked><input id=t checked><div checked></div><option selected></option>");
    let c = tree.query_selector("#c").unwrap();
    let t = tree.query_selector("#t").unwrap();
    assert_eq!(tree.query_selector_all(":checked").len(), 3);
    tree.start_recording();
    tree.set_form_property(c, FormProp::Checked, false);
    assert!(!tree.matches(c, ":checked"));
    assert_eq!(tree.get_attribute(c, "checked"), Some(""));
    assert_eq!(tree.form_property(c, FormProp::Checked), Some(false));
    let version = tree.version;
    tree.set_form_property(c, FormProp::Checked, false);
    assert_eq!(tree.version, version);
    tree.set_form_property(t, FormProp::Checked, true);
    assert!(!tree.matches(t, ":checked"));
    tree.set_attribute(c, "checked", "updated");
    assert!(!tree.matches(c, ":checked"));
    tree.take_mutation_events();
    tree.clear_form_property(c, FormProp::Checked);
    assert!(tree.matches(c, ":checked"));
    assert_eq!(tree.take_mutation_events(), []);
    assert_eq!(tree.form_property(c, FormProp::Checked), None);
    let version = tree.version;
    tree.set_form_property(Handle::from_raw(u32::MAX), FormProp::Checked, true);
    tree.set_form_property(tree.root(), FormProp::Checked, true);
    assert_eq!(tree.version, version);
}
