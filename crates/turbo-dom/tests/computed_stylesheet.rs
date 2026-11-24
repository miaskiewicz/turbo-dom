//! The shared structural API preserves raw payloads without evaluating their cascade.
use turbo_dom::rtdom::computed_css::{
    declarations,
    stylesheet::{parse_sheet, GroupKind, GroupMode, SheetError, SheetLimits, SheetRule},
};

fn limits() -> SheetLimits {
    SheetLimits {
        source_units: 4096,
        rules: 32,
        group_depth: 4,
    }
}

#[test]
fn owned_rules_preserve_comments_and_raw_units_for_independent_consumers() {
    let mut source: Vec<u16> = ".t{--Token:/* boundary */\"".encode_utf16().collect();
    source.push(0xd800);
    source.extend("\";fill:var(--Token)}@media screen{.t{fill:red}}".encode_utf16());
    let rules = parse_sheet(&source, limits(), GroupMode::Recursive).unwrap();
    source.clear();
    let SheetRule::Style { selector, body } = &rules[0] else {
        panic!("style")
    };
    assert_eq!(selector.as_units(), [46, 116]);
    let parsed = declarations(body.as_units());
    assert_eq!(parsed[0].name, "--Token");
    assert!(parsed[0].value.as_units().contains(&0xd800));
    assert!(String::from_utf16_lossy(parsed[0].value.as_units()).starts_with("/* boundary */"));
    assert_eq!(
        String::from_utf16_lossy(parsed[1].value.as_units()),
        "var(--Token)"
    );
    assert!(
        matches!(&rules[1], SheetRule::Group {kind:GroupKind::Media,rules,..} if rules.len()==1)
    );
}

#[test]
fn opaque_groups_are_reported_and_resource_errors_remain_typed() {
    let source: Vec<u16> = "@media screen{.t{fill:red}}".encode_utf16().collect();
    let rules = parse_sheet(
        &source,
        SheetLimits {
            group_depth: 0,
            ..limits()
        },
        GroupMode::Opaque,
    )
    .unwrap();
    assert!(
        matches!(&rules[0], SheetRule::Opaque {name,body,..} if name=="media" && !body.as_units().is_empty())
    );
    assert_eq!(
        parse_sheet(
            &source,
            SheetLimits {
                group_depth: 0,
                ..limits()
            },
            GroupMode::Recursive
        ),
        Err(SheetError::GroupDepth)
    );
    assert_eq!(
        parse_sheet(
            &source,
            SheetLimits {
                source_units: 0,
                ..limits()
            },
            GroupMode::Opaque
        ),
        Err(SheetError::SourceUnits)
    );
    assert_eq!(
        parse_sheet(
            &source,
            SheetLimits {
                rules: 0,
                ..limits()
            },
            GroupMode::Opaque
        ),
        Err(SheetError::RuleCount)
    );
}
