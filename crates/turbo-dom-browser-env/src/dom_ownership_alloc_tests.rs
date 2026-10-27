//! miaskiewicz/turbo-test#127: `pin_callback_owners` runs before every native DOM method and
//! setter, so it must not heap-allocate per call (it built a HashSet and a Vec each time and
//! showed up high in profiles of a React suite). Counted with a per-thread counting allocator,
//! never wall-clock time; plus the owners it pins (one per distinct owner document).
use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    // `const` with no destructor: safe to touch from inside the allocator.
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        unsafe { System.realloc(pointer, layout, size) }
    }
}

// Counts per thread, so the crate's other tests running in parallel do not pollute it.
#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn allocations() -> usize {
    ALLOCATIONS.with(Cell::get)
}

fn object<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Object> {
    let code = v8::String::new(scope, source).unwrap();
    let value = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    v8::Local::<v8::Object>::try_from(value).unwrap()
}

#[test]
#[ignore = "turbo-test#127: fails until the fix"]
fn pinning_call_owners_does_not_allocate_and_pins_each_owner_document_once() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><div id=a><b></b></div>");
        let receiver = object(scope, "document.getElementById('a')");
        let child = object(scope, "document.querySelector('b')");
        let foreign = object(
            scope,
            "globalThis.foreign = document.implementation.createHTMLDocument('').createElement('i')",
        );
        let text = v8::String::new(scope, "text").unwrap().into();
        let number = v8::Number::new(scope, 7.0).into();
        let plain = v8::Object::new(scope).into();
        // Same-document call: receiver, a node, and non-node arguments.
        let before = allocations();
        for _ in 0..100 {
            v8::scope!(let call, scope);
            let owners = dom_ownership::pin_callback_owners(
                call,
                receiver,
                [child.into(), text, number, plain],
            );
            assert_eq!(owners.documents(), 1);
        }
        let same = allocations() - before;
        // Cross-document call: the foreign node's document is pinned too, once.
        let before = allocations();
        for _ in 0..100 {
            v8::scope!(let call, scope);
            let owners = dom_ownership::pin_callback_owners(
                call,
                receiver,
                [foreign.into(), child.into(), foreign.into()],
            );
            assert_eq!(owners.documents(), 2);
        }
        let cross = allocations() - before;
        assert_eq!((same, cross), (0, 0), "heap allocations per 100 calls");
        reset();
    }
    clear_interface_records(isolate);
}
