//! Private immutable custom-token snapshots for canonical transition endpoints.
use super::*;
use std::collections::BTreeMap;
use turbo_dom_parser::rtdom::computed_css::{self, ComputationLimits, ComputedCssError, CssText};

const MAX_EFFECT_TOKEN_UNITS: usize = 1_048_576;
const POLICY: ComputationLimits = ComputationLimits {
    max_nesting: 16,
    max_output_units: MAX_EFFECT_TOKEN_UNITS,
    max_total_units: MAX_EFFECT_TOKEN_UNITS,
};

fn preflight(
    scope: &mut v8::PinScope,
    values: &[v8::Local<v8::Value>],
    source: Option<v8::Local<v8::Value>>,
) -> bool {
    let mut units = 0usize;
    if let Some(source) = source {
        let Ok(source) = v8::Local::<v8::String>::try_from(source) else {
            return false;
        };
        units = source.length();
    }
    for value in values {
        let Ok(array) = v8::Local::<v8::Array>::try_from(*value) else {
            return false;
        };
        if array.length() as usize > MAX_EFFECT_TOKEN_UNITS {
            failure(scope, ComputedCssError::ResourceLimit);
            return false;
        }
        for index in 0..array.length() {
            let Some(value) = array.get_index(scope, index) else {
                return false;
            };
            let Ok(value) = v8::Local::<v8::String>::try_from(value) else {
                return false;
            };
            let Some(next) = units.checked_add(value.length()) else {
                failure(scope, ComputedCssError::ResourceLimit);
                return false;
            };
            units = next;
            if units > MAX_EFFECT_TOKEN_UNITS {
                failure(scope, ComputedCssError::ResourceLimit);
                return false;
            }
        }
    }
    if units > MAX_EFFECT_TOKEN_UNITS {
        failure(scope, ComputedCssError::ResourceLimit);
        return false;
    }
    true
}

fn text(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<CssText> {
    let value = v8::Local::<v8::String>::try_from(value).ok()?;
    Some(CssText::from_units(string_code_units(scope, value)))
}

fn pairs(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
) -> Option<BTreeMap<String, CssText>> {
    let array = v8::Local::<v8::Array>::try_from(value).ok()?;
    if array.length() % 2 != 0 {
        return None;
    }
    let mut out = BTreeMap::new();
    for index in (0..array.length()).step_by(2) {
        let key = array.get_index(scope, index)?;
        let key = text(scope, key)?;
        let key = String::from_utf16(key.as_units()).ok()?;
        let value = array.get_index(scope, index + 1)?;
        out.insert(key, text(scope, value)?);
    }
    Some(out)
}

fn failure(scope: &mut v8::PinScope, error: ComputedCssError) {
    if scope.is_execution_terminating() {
        return;
    }
    if matches!(
        error,
        ComputedCssError::ResourceLimit | ComputedCssError::NestingLimit
    ) {
        let Some(message) = v8::String::new(
            scope,
            "Animation custom tokens exceed their computation budget",
        ) else {
            return;
        };
        if scope.is_execution_terminating() {
            return;
        }
        let error = v8::Exception::range_error(scope, message);
        scope.throw_exception(error);
    }
}

fn custom(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    rv.set(v8::null(scope).into());
    if !preflight(scope, &[args.get(0), args.get(1)], None) {
        return;
    }
    let Some(parent) = pairs(scope, args.get(0)) else {
        return;
    };
    let Some(own) = pairs(scope, args.get(1)) else {
        return;
    };
    match computed_css::compute_custom_properties(&parent, &own, POLICY) {
        Ok(computed) => {
            let mut values = Vec::new();
            for (name, value) in computed.as_map() {
                values.push(utf8_string(scope, name).into());
                values.push(unit_string(scope, value.as_units()).into());
            }
            let result = v8::Array::new_with_elements(scope, &values);
            let null = v8::null(scope);
            result.set_prototype(scope, null.into());
            rv.set(result.into());
        }
        Err(error) => failure(scope, error),
    }
}

fn substitute(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    rv.set(v8::null(scope).into());
    let Some(source) = text(scope, args.get(0)) else {
        return;
    };
    let scan = ComputationLimits {
        max_output_units: source.as_units().len(),
        max_total_units: source.as_units().len(),
        ..POLICY
    };
    match computed_css::has_variable_references(&source, scan) {
        Ok(false) => {
            rv.set(args.get(0));
            return;
        }
        Ok(true) => {}
        Err(_) => {
            rv.set(v8::undefined(scope).into());
            return;
        }
    }
    if !preflight(scope, &[], Some(args.get(0))) {
        return;
    }
    if !preflight(scope, &[args.get(1)], Some(args.get(0))) {
        return;
    }
    let Some(parent) = pairs(scope, args.get(1)) else {
        return;
    };
    let variables = match computed_css::compute_custom_properties(&parent, &BTreeMap::new(), POLICY)
    {
        Ok(variables) => variables,
        Err(error) => {
            failure(scope, error);
            return;
        }
    };
    match computed_css::substitute(&source, &variables, POLICY) {
        Ok(Some(value)) => rv.set(unit_string(scope, value.as_units()).into()),
        Ok(None) => rv.set_bool(false),
        Err(error) => failure(scope, error),
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    if let Some(namespace) = engine_namespace(scope) {
        bind_method(scope, namespace, "computeEffectCustomTokens", custom);
        bind_method(scope, namespace, "substituteEffectTokens", substitute);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_formatter_preserves_native_termination_and_ordinary_range_errors() {
        crate::browser_env::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        v8::tc_scope!(let scope, scope);
        let code = v8::String::new(scope, "0").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        scope.terminate_execution();
        assert!(script.run(scope).is_none());
        assert!(scope.is_execution_terminating());
        failure(scope, ComputedCssError::ResourceLimit);
        assert!(scope.has_terminated());
        assert!(scope.is_execution_terminating());
        assert!(!scope
            .exception()
            .is_some_and(|error| error.is_native_error()));

        // Only the test owner cancels after proving the helper preserved the
        // interruption, then checks the ordinary resource-error inverse.
        scope.cancel_terminate_execution();
        scope.reset();
        failure(scope, ComputedCssError::ResourceLimit);
        assert!(scope.has_caught());
        assert!(!scope.has_terminated());
        assert!(scope.exception().unwrap().is_native_error());
        let message = scope
            .message()
            .unwrap()
            .get(scope)
            .to_rust_string_lossy(scope);
        assert!(message.contains("RangeError") && message.contains("computation budget"));
    }
}
