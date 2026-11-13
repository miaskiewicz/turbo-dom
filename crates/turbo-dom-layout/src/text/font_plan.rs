//! Reuse the immutable OpenType plan across distinct strings on one loaded face.
use std::collections::VecDeque;
use std::sync::Arc;

use rustybuzz::{Direction, Language, Script, ShapePlan, UnicodeBuffer};

use super::{collect_glyphs, FontFace, ShapedGlyph};

// Separate from the bounded glyph-result payload: at most eight resident plans per
// face. Face clones share this cache; collection indexes have separate parsed faces.
const PLAN_CACHE_ENTRIES: usize = 8;

#[derive(Clone, PartialEq, Eq)]
struct PlanKey {
    direction: Direction,
    script: Script,
    language: Option<Language>,
}

impl PlanKey {
    fn of(buffer: &UnicodeBuffer) -> Self {
        Self {
            direction: buffer.direction(),
            script: buffer.script(),
            language: buffer.language(),
        }
    }
}

struct PlanEntry {
    key: PlanKey,
    plan: Arc<ShapePlan>,
}

#[derive(Default)]
pub(super) struct PlanCache {
    entries: VecDeque<PlanEntry>,
}

impl PlanCache {
    fn get(&mut self, key: &PlanKey) -> Option<Arc<ShapePlan>> {
        let index = self.entries.iter().position(|entry| entry.key == *key)?;
        let entry = self.entries.remove(index).expect("matched resident plan");
        let plan = Arc::clone(&entry.plan);
        self.entries.push_back(entry);
        Some(plan)
    }

    fn insert(&mut self, key: PlanKey, plan: Arc<ShapePlan>) -> Arc<ShapePlan> {
        if let Some(resident) = self.get(&key) {
            return resident;
        }
        if self.entries.len() == PLAN_CACHE_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(PlanEntry {
            key,
            plan: Arc::clone(&plan),
        });
        plan
    }
}

impl FontFace {
    pub(super) fn shape_buffer(&self, buffer: UnicodeBuffer) -> Vec<ShapedGlyph> {
        // Callers prepare segment properties before entering here, including
        // applying a forced direction after guessing the other properties.
        // Features are always empty; selection would require a PlanKey field.
        let key = PlanKey::of(&buffer);
        let hit = self
            .shape_cache
            .lock()
            .expect("shape cache not poisoned")
            .plans
            .get(&key);
        let plan = hit.unwrap_or_else(|| self.create_plan(key));
        let glyphs = rustybuzz::shape_with_plan(&self.faces.borrow_dependent().rb, &plan, buffer);
        collect_glyphs(&glyphs)
    }

    fn create_plan(&self, key: PlanKey) -> Arc<ShapePlan> {
        crate::hot!("font.shape_plan");
        let plan = Arc::new(ShapePlan::new(
            &self.faces.borrow_dependent().rb,
            key.direction,
            (key.script != rustybuzz::script::UNKNOWN).then_some(key.script),
            key.language.as_ref(),
            &[],
        ));
        // Build without holding the cache mutex. A concurrent miss may already
        // have admitted the same key; keep that first immutable plan in that case.
        self.shape_cache
            .lock()
            .expect("shape cache not poisoned")
            .plans
            .insert(key, plan)
    }
}

#[cfg(test)]
#[path = "font_plan_tests.rs"]
mod tests;
