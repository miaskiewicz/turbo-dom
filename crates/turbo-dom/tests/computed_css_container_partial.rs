#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::{
    computed_css::{
        container::{
            container_matches, container_matches_with_measurements, ContainerAncestor,
            ContainerError, ContainerMeasurements, ContentSize, InlineAxis, PartialContentSize,
            SizeKind,
        },
        ComputationLimits, CssText,
    },
    css_shorthand::LengthContext,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn ancestor<'a>(context: &'a LengthContext<'a>) -> ContainerAncestor<'a> {
    ContainerAncestor {
        id: 42,
        names: &[],
        kind: SizeKind::Size,
        inline_axis: InlineAxis::Horizontal,
        scroll_state: false,
        size: None,
        length_context: context,
    }
}
fn query(
    source: &str,
    ancestors: &[ContainerAncestor<'_>],
    measurements: &ContainerMeasurements,
) -> Result<bool, ContainerError> {
    container_matches_with_measurements(&text(source), ancestors, Some(measurements), limits())
        .map(|value| value.matches)
}
#[test]
fn only_required_axes_are_read_and_missing_is_never_zero() {
    let context = LengthContext { unit_bases: &[] };
    let ancestors = [ancestor(&context)];
    let measurements = ContainerMeasurements::from([(
        42,
        PartialContentSize {
            width_px: Some(400.0),
            height_px: None,
        },
    )]);
    assert_eq!(
        query("(min-width:300px)", &ancestors, &measurements),
        Ok(true)
    );
    for source in [
        "(height:0px)",
        "(aspect-ratio:4/1)",
        "(orientation:landscape)",
        "not (height:0px)",
    ] {
        assert_eq!(
            query(source, &ancestors, &measurements),
            Err(ContainerError::RequiresMeasurement(42)),
            "{source}"
        );
    }
    assert_eq!(
        query("(height:0px) or (width:400px)", &ancestors, &measurements),
        Ok(true)
    );
    assert_eq!(
        query("(height:0px) and (width:0px)", &ancestors, &measurements),
        Ok(false)
    );
}
#[test]
fn original_ancestor_id_selects_authoritative_partial_records() {
    let context = LengthContext { unit_bases: &[] };
    let mut original = ancestor(&context);
    original.size = Some(ContentSize {
        width_px: 200.0,
        height_px: 100.0,
    });
    let measurements = ContainerMeasurements::from([(
        42,
        PartialContentSize {
            width_px: Some(400.0),
            height_px: None,
        },
    )]);
    assert_eq!(query("(width:400px)", &[original], &measurements), Ok(true));
    assert_eq!(
        query("(height:100px)", &[original], &measurements),
        Err(ContainerError::RequiresMeasurement(42))
    );
    assert!(
        container_matches(&text("(width:200px)"), &[original], limits())
            .unwrap()
            .matches
    );
    assert_eq!(
        query("(width:200px)", &[original], &ContainerMeasurements::new()),
        Ok(true)
    );
    let mut farther = original;
    farther.id = 7;
    farther.size = Some(ContentSize {
        width_px: 800.0,
        height_px: 1000.0,
    });
    assert_eq!(
        query("(height:1000px)", &[original, farther], &measurements),
        Err(ContainerError::RequiresMeasurement(42))
    );
}
#[test]
fn vertical_inline_axis_requires_actual_height_and_block_axis_actual_width() {
    let context = LengthContext { unit_bases: &[] };
    let mut value = ancestor(&context);
    value.inline_axis = InlineAxis::Vertical;
    let measurements = ContainerMeasurements::from([(
        42,
        PartialContentSize {
            width_px: Some(400.0),
            height_px: None,
        },
    )]);
    assert_eq!(
        query("(block-size:400px)", &[value], &measurements),
        Ok(true)
    );
    assert_eq!(
        query("(inline-size:400px)", &[value], &measurements),
        Err(ContainerError::RequiresMeasurement(42))
    );
    value.kind = SizeKind::InlineSize;
    let measurements = ContainerMeasurements::from([(
        42,
        PartialContentSize {
            width_px: None,
            height_px: Some(400.0),
        },
    )]);
    assert_eq!(
        query("(inline-size:400px)", &[value], &measurements),
        Ok(true)
    );
    assert_eq!(query("(width:400px)", &[value], &measurements), Ok(false));
}
#[test]
fn actual_zero_is_valid_and_present_invalid_metrics_are_errors() {
    let context = LengthContext { unit_bases: &[] };
    let ancestors = [ancestor(&context)];
    let mut measurements = ContainerMeasurements::from([(
        42,
        PartialContentSize {
            width_px: Some(0.0),
            height_px: None,
        },
    )]);
    assert_eq!(query("(width)", &ancestors, &measurements), Ok(false));
    assert_eq!(query("not (width)", &ancestors, &measurements), Ok(true));
    for invalid in [f64::NAN, f64::INFINITY, -1.0] {
        measurements.insert(
            42,
            PartialContentSize {
                width_px: Some(400.0),
                height_px: Some(invalid),
            },
        );
        assert_eq!(
            query("(width:400px)", &ancestors, &measurements),
            Err(ContainerError::InvalidMeasurement(42))
        );
    }
    measurements.insert(
        42,
        PartialContentSize {
            width_px: Some(400.0),
            height_px: None,
        },
    );
    measurements.insert(
        99,
        PartialContentSize {
            width_px: Some(f64::NAN),
            height_px: None,
        },
    );
    assert_eq!(query("(width:400px)", &ancestors, &measurements), Ok(true));
}
