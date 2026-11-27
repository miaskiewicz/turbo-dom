//! Synthetic provider API contracts; actual Chrome inheritance controls are separate.
#![cfg(feature = "computed-style-grammar")]
use std::collections::{BTreeMap, BTreeSet};
use turbo_dom::rtdom::computed_css::{
    used_values::{
        project_used_property, Axis, AxisMeasurement, BoxBasis, Projection, UsedContext,
        UsedMeasurement, UsedValueError,
    },
    ComputationLimits, ComputedCssError, CssText,
};
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 32,
        max_output_units: 128,
        max_total_units: 256,
    }
}
fn read(
    name: &str,
    entries: &[(&str, &str)],
    measurement: Option<&UsedMeasurement<'_>>,
) -> Result<Projection, UsedValueError> {
    let ordinary = entries
        .iter()
        .map(|(name, value)| ((*name).into(), text(value)))
        .collect();
    let public_keys = BTreeSet::from(["width".into(), "height".into(), "color".into()]);
    let namespace: Vec<_> = "http://www.w3.org/1999/xhtml".encode_utf16().collect();
    project_used_property(
        name,
        &UsedContext {
            target: 7,
            world: &[1, 2],
            namespace: &namespace,
            pseudo: None,
            ordinary: &ordinary,
            public_keys: &public_keys,
            measurement,
        },
        limits(),
    )
}
fn report(horizontal: AxisMeasurement) -> UsedMeasurement<'static> {
    UsedMeasurement {
        target: 7,
        world: &[1, 2],
        horizontal,
        vertical: AxisMeasurement::Unknown,
    }
}
#[test]
fn unrelated_and_nonpublic_reads_do_not_require_context_or_measurement() {
    let context = UsedContext {
        target: 7,
        world: &[],
        namespace: &[],
        pseudo: Some("before"),
        ordinary: &BTreeMap::new(),
        public_keys: &BTreeSet::new(),
        measurement: None,
    };
    assert_eq!(
        project_used_property("width", &context, limits()),
        Ok(Projection::Unchanged)
    );
    assert_eq!(
        read("color", &[("display", "invalid")], None),
        Ok(Projection::Unchanged)
    );
}
#[test]
fn no_box_inline_and_zero_are_distinct_and_raw_inline_never_suppresses_measurement() {
    for display in ["none", "CoNtEnTs", "\\6e one"] {
        assert_eq!(
            read("width", &[("display", display)], None),
            Ok(Projection::Unchanged)
        );
    }
    assert_eq!(
        read("width", &[("display", "inline")], None),
        Ok(Projection::NeedsMeasurement {
            axis: Axis::Horizontal,
            basis: BoxBasis::Content
        })
    );
    for absent in [AxisMeasurement::NoBox, AxisMeasurement::NonAtomicInline] {
        assert_eq!(
            read("width", &[("width", "50%")], Some(&report(absent))),
            Ok(Projection::Unchanged)
        );
    }
    assert_eq!(
        read(
            "width",
            &[],
            Some(&report(AxisMeasurement::Pixels {
                content: Some(0.0),
                border: None
            }))
        ),
        Ok(Projection::Value(text("0px")))
    );
}
#[test]
fn selected_axis_and_basis_are_independent_and_actual_constraints_win() {
    let measured = report(AxisMeasurement::Pixels {
        content: Some(200.0),
        border: None,
    });
    assert_eq!(
        read(
            "width",
            &[("width", "1000px"), ("max-width", "200px")],
            Some(&measured)
        ),
        Ok(Projection::Value(text("200px")))
    );
    assert_eq!(
        read("height", &[], Some(&measured)),
        Ok(Projection::NeedsMeasurement {
            axis: Axis::Vertical,
            basis: BoxBasis::Content
        })
    );
    assert_eq!(
        read("width", &[("box-sizing", "BORDER-BOX")], Some(&measured)),
        Ok(Projection::NeedsMeasurement {
            axis: Axis::Horizontal,
            basis: BoxBasis::Border
        })
    );
    let border = report(AxisMeasurement::Pixels {
        content: Some(f32::NAN),
        border: Some(240.0),
    });
    assert_eq!(
        read("width", &[("box-sizing", "border-box")], Some(&border)),
        Ok(Projection::Value(text("240px")))
    );
    assert_eq!(
        read("width", &[], Some(&border)),
        Err(UsedValueError::InvalidMeasurement)
    );
}
#[test]
fn provenance_invalid_metrics_and_checked_output_remain_errors() {
    let mut measured = report(AxisMeasurement::Pixels {
        content: Some(400.0),
        border: None,
    });
    measured.target = 8;
    assert_eq!(
        read("width", &[], Some(&measured)),
        Err(UsedValueError::ForeignMeasurement)
    );
    measured.target = 7;
    measured.world = &[3];
    assert_eq!(
        read("width", &[], Some(&measured)),
        Err(UsedValueError::ForeignMeasurement)
    );
    for value in [f32::NAN, f32::INFINITY, -1.0] {
        assert_eq!(
            read(
                "width",
                &[],
                Some(&report(AxisMeasurement::Pixels {
                    content: Some(value),
                    border: None
                }))
            ),
            Err(UsedValueError::InvalidMeasurement)
        );
    }
    assert_eq!(
        read("width", &[("display", "not-display")], None),
        Err(UsedValueError::InvalidProperty("display".into()))
    );
    assert_eq!(
        read("width", &[("box-sizing", "unknown")], None),
        Err(UsedValueError::InvalidProperty("box-sizing".into()))
    );
}
#[test]
fn relevant_unsupported_context_and_exact_output_limits() {
    let ordinary = BTreeMap::new();
    let public_keys = BTreeSet::from(["width".into()]);
    let namespace: Vec<_> = "http://www.w3.org/1999/xhtml".encode_utf16().collect();
    let measurement = report(AxisMeasurement::Pixels {
        content: Some(400.0),
        border: None,
    });
    let mut context = UsedContext {
        target: 7,
        world: &[1, 2],
        namespace: &namespace,
        pseudo: Some("before"),
        ordinary: &ordinary,
        public_keys: &public_keys,
        measurement: Some(&measurement),
    };
    assert_eq!(
        project_used_property("width", &context, limits()),
        Err(UsedValueError::UnsupportedPseudo)
    );
    context.pseudo = None;
    context.namespace = &[];
    assert_eq!(
        project_used_property("width", &context, limits()),
        Err(UsedValueError::UnsupportedNamespace)
    );
    context.namespace = &namespace;
    let mut bound = limits();
    bound.max_output_units = 5;
    assert_eq!(
        project_used_property("width", &context, bound),
        Ok(Projection::Value(text("400px")))
    );
    let supplied = BTreeMap::from([("box-sizing".into(), text("content-box"))]);
    context.ordinary = &supplied;
    assert_eq!(
        project_used_property("width", &context, bound),
        Err(UsedValueError::Computation(ComputedCssError::ResourceLimit))
    );
    context.ordinary = &ordinary;
    bound.max_output_units = 4;
    assert_eq!(
        project_used_property("width", &context, bound),
        Err(UsedValueError::Computation(ComputedCssError::ResourceLimit))
    );
    bound = limits();
    bound.max_total_units = 4;
    assert_eq!(
        project_used_property("width", &context, bound),
        Err(UsedValueError::Computation(ComputedCssError::ResourceLimit))
    );
}

#[test]
fn weighted_backend_nesting_refuses_before_style_grammar_admission() {
    let value = format!("{}block{}", "env(foo,".repeat(251), ")".repeat(251));
    let ordinary = BTreeMap::from([("display".into(), text(&value))]);
    let public_keys = BTreeSet::from(["width".into()]);
    let namespace: Vec<_> = "http://www.w3.org/1999/xhtml".encode_utf16().collect();
    let context = UsedContext {
        target: 7,
        world: &[1, 2],
        namespace: &namespace,
        pseudo: None,
        ordinary: &ordinary,
        public_keys: &public_keys,
        measurement: None,
    };
    let policy = ComputationLimits {
        max_nesting: 1024,
        max_output_units: value.len(),
        max_total_units: value.len(),
    };
    assert_eq!(
        project_used_property("width", &context, policy),
        Err(UsedValueError::Computation(ComputedCssError::NestingLimit))
    );
}
