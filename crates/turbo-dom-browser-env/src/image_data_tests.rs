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
      globalThis.Float16Array = fail; globalThis.ArrayBuffer = fail;
      ctx.putImageData(image, 0, 0); bytes[0] = 99;
      const read = ctx.getImageData(0, 0, 1, 1);
      const allocated = ctx.createImageData(1, 1);
      const floating = new ImageData(1,1,{pixelFormat:'rgba-float16'});
      floating.data[0] = 0.5;
      return [read.data[0],read.data[1],read.data[2],allocated.width,allocated.data.length,floating.data[0],floating.data.BYTES_PER_ELEMENT];
    })()"#);
    assert_eq!(result, serde_json::json!([12, 34, 56, 1, 4, 0.5, 2]));
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
    run_js(scope, "globalThis.pixelBytes = new Uint8ClampedArray([12,34,56,255]); globalThis.pixelBuffer = pixelBytes.buffer; globalThis.detachedImage = new ImageData(pixelBytes,1);globalThis.uploadBeforeDetach=document.createElement('canvas').getContext('2d');uploadBeforeDetach.putImageData(detachedImage,0,0);uploadBeforeDetach._ops.length=0;");
    let key = v8::String::new(scope, "pixelBuffer").unwrap();
    let value = global.get(scope, key.into()).unwrap();
    let buffer = v8::Local::<v8::ArrayBuffer>::try_from(value).unwrap();
    assert_eq!(buffer.detach(None), Some(true));
    assert_eq!(
        eval(
            scope,
            "JSON.stringify(Array.from(uploadBeforeDetach.getImageData(0,0,1,1).data))"
        ),
        "[12,34,56,255]"
    );
    assert_eq!(
        eval(
            scope,
            r#"JSON.stringify([()=>new ImageData(pixelBytes,1),()=>document.createElement('canvas').getContext('2d').putImageData(detachedImage,0,0)].map(fn=>{try{fn();return 'no error'}catch(e){return e.name}}))"#
        ),
        r#"["InvalidStateError","InvalidStateError"]"#
    );
    reset();
}

#[test]
fn image_data_allocates_float16_without_a_global_constructor_at_install() {
    use super::*;
    super::tests::init_v8();
    let isolate = &mut super::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    run_js(scope, "delete globalThis.Float16Array;");
    install_with_html(scope, "<body></body>");
    let source = v8::String::new(scope, r#"JSON.stringify((()=>{
      try {
        const image = new ImageData(1, 1, {pixelFormat:'rgba-float16'});
        image.data[0] = 0.5;
        const clone = document.createElement('canvas').getContext('2d').createImageData(image);
        globalThis.createdFloat = image;
        const view = image.data;
        const borrowed = new ImageData(view,1,1,{pixelFormat:'rgba-float16'});
        return [ArrayBuffer.isView(image.data),image.data.BYTES_PER_ELEMENT,image.data[0],clone.pixelFormat,clone.data[0],borrowed.data===view,Object.prototype.toString.call(image.data)];
      } catch(e) { return e.name; }
    })())"#).unwrap();
    let result = v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    assert_eq!(
        result,
        r#"[true,2,0.5,"rgba-float16",0,true,"[object Float16Array]"]"#
    );
    let global = context.global(scope);
    let key = v8::String::new(scope, "createdFloat").unwrap();
    let image = global
        .get(scope, key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    let key = v8::String::new(scope, "data").unwrap();
    assert!(image.get(scope, key.into()).unwrap().is_float16_array());
    reset();
}

#[test]
fn image_data_partial_readback_storage_is_bounded_after_uploads_drain() {
    use super::*;
    super::tests::init_v8();
    let isolate = &mut super::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<body></body>");
    run_js(scope, "globalThis.cv=document.createElement('canvas');cv.width=128;cv.height=128;globalThis.cx=cv.getContext('2d');globalThis.im=new ImageData(128,128);im.data.fill(255);cx.putImageData(im,0,0);cx._ops.length=0;");
    scope.low_memory_notification();
    let before = scope.get_heap_statistics().used_heap_size();
    run_js(scope, "for(let n=0;n<160;n++){im.data[0]=n;cx.putImageData(im,0,0,0,0,1,1);cx._ops.length=0;}if(cx.getImageData(0,0,1,1).data[0]!==159||cx.getImageData(127,127,1,1).data[3]!==255)throw Error('lost latest or untouched pixels');");
    scope.low_memory_notification();
    let after = scope.get_heap_statistics().used_heap_size();
    eprintln!("retained readback heap: {before} -> {after}");
    assert!(
        after.saturating_sub(before) < 8 * 1024 * 1024,
        "retained upload history grew heap: {before} -> {after}"
    );
}

#[test]
fn image_data_backing_preserves_partial_transparent_pixels_and_resize() {
    let actual = run(r#"(() => {
      const cv=document.createElement('canvas');cv.width=3;cv.height=2;
      const ctx=cv.getContext('2d'), im=new ImageData(new Uint8ClampedArray([10,20,30,255,40,50,60,255]),2);
      ctx.putImageData(im,0,0);ctx._ops.length=0;im.data.fill(99);
      const partial=new ImageData(new Uint8ClampedArray([0,0,0,0,70,80,90,255]),2);
      ctx.putImageData(partial,0,0,0,0,1,1);ctx._ops.length=0;
      const first=Array.from(ctx.getImageData(0,0,3,1).data);
      const independent=ctx.getImageData(0,0,3,1);independent.data.fill(88);
      const second=Array.from(ctx.getImageData(1,0,1,1).data);
      cv.width=4;ctx._ops.length=0;
      const resized=Array.from(ctx.getImageData(0,0,2,1).data);
      ctx.putImageData(new ImageData(4,2),0,0);ctx.putImageData(new ImageData(4,2),0,0);
      return [first,second,resized,ctx._ops.length];
    })()"#);
    assert_eq!(
        actual,
        serde_json::json!([
            [0, 0, 0, 0, 40, 50, 60, 255, 0, 0, 0, 0],
            [40, 50, 60, 255],
            [0, 0, 0, 0, 0, 0, 0, 0],
            1
        ])
    );
}

#[test]
fn canvas_dimensions_and_reset_match_chromium156() {
    const PROBE: &str = include_str!("../tests/fixtures/canvas_dimensions_probe.js");
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/canvas_dimensions_chrome156.json"
    ))
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(PROBE.as_bytes())),
        control["probeSha256"].as_str().unwrap()
    );
    let actual = run(PROBE);
    let mut out = Vec::new();
    differences("result", &actual, &control["result"], &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

#[test]
fn canvas_readback_does_not_invoke_prototype_serialization_hooks() {
    let actual = run(r#"(() => {
      const cv=document.createElement('canvas'),ctx=cv.getContext('2d');ctx.fillRect(0,0,1,1);
      let hits=0;const array=Object.getOwnPropertyDescriptor(Array.prototype,'toJSON'),object=Object.getOwnPropertyDescriptor(Object.prototype,'toJSON');
      Object.defineProperty(Array.prototype,'toJSON',{configurable:true,get(){hits++;throw Error('array hook')}});
      Object.defineProperty(Object.prototype,'toJSON',{configurable:true,get(){hits++;throw Error('object hook')}});
      Object.defineProperty(ctx,'_ops',{configurable:true,get(){hits++;throw Error('ops getter')}});
      Object.defineProperty(cv,'__ctx2d',{configurable:true,get(){hits++;throw Error('ctx getter')}});
      let status='ok';try{cv.toDataURL();ctx.getImageData(0,0,1,1);}catch(e){status=e.message;}
      if(array)Object.defineProperty(Array.prototype,'toJSON',array);else delete Array.prototype.toJSON;
      if(object)Object.defineProperty(Object.prototype,'toJSON',object);else delete Object.prototype.toJSON;
      return [status,hits];
    })()"#);
    assert_eq!(actual, serde_json::json!(["ok", 0]));
}

#[test]
fn canvas_readback_rejects_accessors_and_proxies_without_running_them() {
    let actual = run(r#"(() => {
      let calls=0;const outcomes=[];
      for(const kind of ['accessor','proxy']) {
        const cv=document.createElement('canvas'),ctx=cv.getContext('2d');
        const row=kind==='proxy'?new Proxy([],{get(){calls++;throw Error('proxy')},ownKeys(){calls++;throw Error('proxy')}}):[];
        if(kind==='accessor')Object.defineProperty(row,0,{get(){calls++;throw Error('getter')}});
        ctx._ops.push(row);
        for(const read of [()=>cv.toDataURL(),()=>ctx.getImageData(0,0,1,1)]) {
          try{read();outcomes.push('no error')}catch(e){outcomes.push(e.name)}
        }
      }
      return [outcomes,calls];
    })()"#);
    assert_eq!(
        actual,
        serde_json::json!([["TypeError", "TypeError", "TypeError", "TypeError"], 0])
    );
}

#[test]
fn canvas_readback_rejects_bigint_without_its_serialization_hook() {
    let actual = run(r#"(() => {
      const cv=document.createElement('canvas'),ctx=cv.getContext('2d');ctx._ops.push(['fillRect',[1n,0,1,1]]);
      let calls=0;const descriptor=Object.getOwnPropertyDescriptor(BigInt.prototype,'toJSON');
      Object.defineProperty(BigInt.prototype,'toJSON',{configurable:true,get(){calls++;throw Error('bigint hook')}});
      let error='none';try{cv.toDataURL()}catch(e){error=e.name}
      if(descriptor)Object.defineProperty(BigInt.prototype,'toJSON',descriptor);else delete BigInt.prototype.toJSON;
      return [error,calls];
    })()"#);
    assert_eq!(actual, serde_json::json!(["TypeError", 0]));
}
