use turbo_dom::rtdom::computed_css::{
    stylesheet::{
        parse_sheet, parse_sheet_with_unknown_rules, GroupKind, GroupMode, SheetError, SheetLimits,
        SheetRule, UnknownRulePolicy,
    },
    CssText,
};
fn limits() -> SheetLimits {
    SheetLimits {
        source_units: 4096,
        rules: 32,
        group_depth: 8,
    }
}
fn units(source: &str) -> Vec<u16> {
    source.encode_utf16().collect()
}
fn text(source: &str) -> CssText {
    CssText::from_units(units(source))
}
#[test]
fn default_and_explicit_ignore_remain_identical_in_both_group_modes() {
    let source=units("@future before;@counter-style count{/*source*/system:numeric;symbols:'a'}@container(width>1px){@future nested; .x{color:red}}.last{color:blue}");
    for mode in [GroupMode::Recursive, GroupMode::Opaque] {
        assert_eq!(
            parse_sheet(&source, limits(), mode),
            parse_sheet_with_unknown_rules(&source, limits(), mode, UnknownRulePolicy::Ignore)
        );
    }
    let parsed = parse_sheet(
        &units("@future x;@counter-style y{x:y}.x{color:red}"),
        limits(),
        GroupMode::Recursive,
    )
    .unwrap();
    assert_eq!(parsed.len(), 1);
    assert!(matches!(&parsed[0],SheetRule::Style{selector,..}if selector==&text(".x")));
}
#[test]
fn retained_unknown_rules_keep_source_order_inside_conditional_groups() {
    let source=units("@future before;@counter-style count{/*source*/system:numeric;symbols:'a'}@container(width>1px){@future nested;@future block{/*raw*/x:y}.x{color:red}}");
    let parsed = parse_sheet_with_unknown_rules(
        &source,
        limits(),
        GroupMode::Recursive,
        UnknownRulePolicy::PreserveOpaque,
    )
    .unwrap();
    assert!(
        matches!(&parsed[0],SheetRule::Statement{name,prelude}if name=="future"&&prelude==&text("before"))
    );
    assert!(
        matches!(&parsed[1],SheetRule::Opaque{name,prelude,body}if name=="counter-style"&&prelude==&text("count")&&body==&text("/*source*/system:numeric;symbols:'a'"))
    );
    let SheetRule::Group { kind, rules, .. } = &parsed[2] else {
        panic!("container group")
    };
    assert_eq!(*kind, GroupKind::Container);
    assert!(
        matches!(&rules[0],SheetRule::Statement{name,prelude}if name=="future"&&prelude==&text("nested"))
    );
    assert!(
        matches!(&rules[1],SheetRule::Opaque{name,body,..}if name=="future"&&body==&text("/*raw*/x:y"))
    );
    assert!(matches!(&rules[2],SheetRule::Style{selector,..}if selector==&text(".x")));
}
#[test]
fn opaque_unknown_payloads_retain_original_utf16_comments_and_do_not_recurse() {
    let mut prelude = units("/*p*/ ");
    prelude.push(0xd800);
    let mut body = units(" /*raw*/ @container(x){");
    body.push(0xdfff);
    body.extend(units("} "));
    let mut source = units("@future ");
    source.extend(&prelude);
    source.extend(units("{"));
    source.extend(&body);
    source.extend(units("}"));
    let mut policy = limits();
    policy.group_depth = 0;
    let parsed = parse_sheet_with_unknown_rules(
        &source,
        policy,
        GroupMode::Recursive,
        UnknownRulePolicy::PreserveOpaque,
    )
    .unwrap();
    let SheetRule::Opaque {
        prelude: actual_prelude,
        body: actual_body,
        ..
    } = &parsed[0]
    else {
        panic!("opaque unknown")
    };
    assert_eq!(actual_prelude.as_units(), prelude);
    assert_eq!(actual_body.as_units(), body);
    assert!(parse_sheet(&source, policy, GroupMode::Recursive)
        .unwrap()
        .is_empty());
}
#[test]
fn retention_preserves_source_rule_and_recursive_group_limits() {
    let source = units("@future x;@future y{}");
    let mut policy = limits();
    policy.source_units = source.len() - 1;
    assert_eq!(
        parse_sheet_with_unknown_rules(
            &source,
            policy,
            GroupMode::Recursive,
            UnknownRulePolicy::PreserveOpaque
        ),
        Err(SheetError::SourceUnits)
    );
    policy = limits();
    policy.rules = 1;
    for unknown in [UnknownRulePolicy::Ignore, UnknownRulePolicy::PreserveOpaque] {
        assert_eq!(
            parse_sheet_with_unknown_rules(&source, policy, GroupMode::Recursive, unknown),
            Err(SheetError::RuleCount)
        );
    }
    policy = limits();
    policy.group_depth = 1;
    assert_eq!(
        parse_sheet_with_unknown_rules(
            &units("@container(x){@container(y){@future z{}}}"),
            policy,
            GroupMode::Recursive,
            UnknownRulePolicy::PreserveOpaque
        ),
        Err(SheetError::GroupDepth)
    );
    assert!(parse_sheet_with_unknown_rules(
        &units("@container(x){@future z{@container(y){}}}"),
        policy,
        GroupMode::Recursive,
        UnknownRulePolicy::PreserveOpaque
    )
    .is_ok());
}
#[test]
fn opaque_group_mode_keeps_nested_unknown_rules_in_original_body() {
    let source = units("@container (width>1px){@future nested;/*tail*/}");
    let parsed = parse_sheet_with_unknown_rules(
        &source,
        limits(),
        GroupMode::Opaque,
        UnknownRulePolicy::PreserveOpaque,
    )
    .unwrap();
    assert!(
        matches!(&parsed[0],SheetRule::Opaque{name,body,..}if name=="container"&&body==&text("@future nested;/*tail*/"))
    );
}
