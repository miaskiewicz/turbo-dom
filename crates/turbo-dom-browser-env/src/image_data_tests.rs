use super::reference_target::probe_harness::{differences, run};
use sha2::{Digest, Sha256};
#[test]
fn image_data_matches_chromium156() {
    const PROBE: &str = include_str!("image_data_probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("image_data_chrome156.json")).unwrap();
    let digest = Sha256::digest(PROBE.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(control["probeSha256"], hex);
    let actual = run(PROBE);
    let mut out = Vec::new();
    differences("result", &actual, &control["result"], &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

#[test]
fn image_data_keeps_slots_and_captured_intrinsics_private() {
    let result = run(r#"(() => {
      const canvas = document.createElement('canvas'), ctx = canvas.getContext('2d');
      const bytes = new Uint8ClampedArray([12, 34, 56, 255]);
      const image = new ImageData(bytes, 1);
      Object.defineProperty(image, 'data', {value:new Uint8ClampedArray([9,9,9,255])});
      const fail = () => { throw new Error('page patched intrinsic reached'); };
      Object.defineProperty = fail; WeakMap.prototype.get = fail; WeakMap.prototype.set = fail;
      Array.prototype.push = fail; Reflect.apply = fail; globalThis.Uint8ClampedArray = fail;
      ctx.putImageData(image, 0, 0); bytes[0] = 99;
      const read = ctx.getImageData(0, 0, 1, 1);
      const allocated = ctx.createImageData(1, 1);
      return [read.data[0],read.data[1],read.data[2],allocated.width,allocated.data.length];
    })()"#);
    assert_eq!(result, serde_json::json!([12, 34, 56, 1, 4]));
}

#[test]
fn image_data_unsupported_canvas_color_conversion_is_explicit() {
    let result = run(r#"(() => {
      const ctx = document.createElement('canvas').getContext('2d');
      const error = fn => { try { fn(); return 'no error'; } catch(e) { return e.name; } };
      return [error(()=>ctx.putImageData(new ImageData(1,1,{colorSpace:'display-p3'}),0,0)),
        error(()=>ctx.getImageData(0,0,1,1,{pixelFormat:'rgba-float16'}))];
    })()"#);
    assert_eq!(
        result,
        serde_json::json!(["NotSupportedError", "NotSupportedError"])
    );
}

#[test]
fn image_data_accepts_foreign_views_and_rejects_detached_buffers() {
    use super::*;
    super::tests::init_v8();
    let isolate = &mut super::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let (foreign, foreign_image) = {
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body></body>");
        let source = v8::String::new(scope, "[new Uint8ClampedArray([1,2,3,255]),new ImageData(new Uint8ClampedArray([4,5,6,255]),1)]").unwrap();
        let value = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let pair = v8::Local::<v8::Array>::try_from(value).unwrap();
        let bytes = pair.get_index(scope, 0).unwrap();
        let image = pair.get_index(scope, 1).unwrap();
        (v8::Global::new(scope, bytes), v8::Global::new(scope, image))
    };
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<body></body>");
    let global = context.global(scope);
    let key = v8::String::new(scope, "foreignPixels").unwrap();
    let value = v8::Local::new(scope, &foreign);
    global.set(scope, key.into(), value);
    let key = v8::String::new(scope, "foreignImage").unwrap();
    let value = v8::Local::new(scope, &foreign_image);
    global.set(scope, key.into(), value);
    let eval = |scope: &mut v8::PinScope, source: &str| {
        let source = v8::String::new(scope, source).unwrap();
        v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    };
    assert_eq!(eval(scope, "JSON.stringify([foreignPixels instanceof Uint8ClampedArray, new ImageData(foreignPixels,1).data === foreignPixels])"), "[false,true]");
    assert_eq!(
        eval(
            scope,
            r#"JSON.stringify((()=>{try{const get=Object.getOwnPropertyDescriptor(ImageData.prototype,'width').get;const ctx=document.createElement('canvas').getContext('2d');ctx.putImageData(foreignImage,0,0);return [get.call(foreignImage),ctx.createImageData(foreignImage).width,ctx.getImageData(0,0,1,1).data[0]]}catch(e){return e.name}})())"#
        ),
        "[1,1,4]"
    );
    run_js(scope, "globalThis.pixelBytes = new Uint8ClampedArray(4); globalThis.pixelBuffer = pixelBytes.buffer; globalThis.detachedImage = new ImageData(pixelBytes,1);");
    let key = v8::String::new(scope, "pixelBuffer").unwrap();
    let value = global.get(scope, key.into()).unwrap();
    let buffer = v8::Local::<v8::ArrayBuffer>::try_from(value).unwrap();
    assert_eq!(buffer.detach(None), Some(true));
    assert_eq!(
        eval(
            scope,
            r#"JSON.stringify([()=>new ImageData(pixelBytes,1),()=>document.createElement('canvas').getContext('2d').putImageData(detachedImage,0,0)].map(fn=>{try{fn();return 'no error'}catch(e){return e.name}}))"#
        ),
        r#"["InvalidStateError","InvalidStateError"]"#
    );
    reset();
}
