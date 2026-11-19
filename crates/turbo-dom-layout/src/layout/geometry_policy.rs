//! Fractional CSS geometry is explicit and independent of fonts and image caps.
use std::cell::Cell;

thread_local! { static FRACTIONAL: Cell<bool> = const { Cell::new(false) }; }

struct Restore(bool);
impl Drop for Restore {
    fn drop(&mut self) {
        FRACTIONAL.set(self.0);
    }
}

/// Preserve fractional flex/grid coordinates while running a static CSS render.
/// The default paged-layout policy remains unchanged outside this call. Nested
/// calls and unwinding restore the previous policy; font metrics and image caps
/// are independent. Native browser layout already preserves these coordinates.
pub fn with_fractional_css_geometry<T>(run: impl FnOnce() -> T) -> T {
    let _restore = Restore(FRACTIONAL.replace(true));
    run()
}

pub(super) fn fractional() -> bool {
    FRACTIONAL.get()
}
