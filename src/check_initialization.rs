use super::*;

#[derive(Clone, Default)]
pub(super) struct InitializationState {
    definite: HashSet<u8>,
    possible: HashSet<u8>,
    deferred: HashMap<u8, usize>,
}

impl InitializationState {
    pub(super) fn from_slots(slots: impl IntoIterator<Item = u8>) -> Self {
        let mut state = Self::default();
        for slot in slots {
            state.insert(slot);
        }
        state
    }

    pub(super) fn contains(&self, slot: &u8) -> bool {
        self.definite.contains(slot)
    }

    pub(super) fn insert(&mut self, slot: u8) -> bool {
        self.possible.insert(slot);
        self.definite.insert(slot)
    }

    pub(super) fn remove(&mut self, slot: &u8) -> bool {
        self.possible.remove(slot);
        self.definite.remove(slot)
    }

    pub(super) fn defer(&mut self, slot: u8, loop_depth: usize) {
        self.remove(&slot);
        self.deferred.insert(slot, loop_depth);
    }

    pub(super) fn is_deferred(&self, slot: u8) -> bool {
        self.deferred.contains_key(&slot)
    }

    pub(super) fn check_write(&self, slot: u8, loop_depth: usize, offset: usize) -> Result<()> {
        let Some(declared_depth) = self.deferred.get(&slot) else {
            return Err(Diagnostic::new(offset, "Cannot reassign val or parameter")
                .with_kind(crate::DiagnosticKind::InvalidKotlin, "VAL_REASSIGNMENT"));
        };
        if self.possible.contains(&slot) {
            return Err(Diagnostic::new(
                offset,
                "Cannot reassign an already or possibly initialized val",
            )
            .with_kind(crate::DiagnosticKind::InvalidKotlin, "VAL_REASSIGNMENT"));
        }
        if *declared_depth != loop_depth {
            return Err(Diagnostic::new(
                offset,
                "Deferred val initialization across a repeating loop boundary is unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "DEFERRED_VAL_LOOP_INITIALIZATION_UNSUPPORTED",
            ));
        }
        Ok(())
    }

    pub(super) fn joined(&self, other: &Self) -> Self {
        let mut deferred = self.deferred.clone();
        deferred.extend(other.deferred.clone());
        Self {
            definite: self
                .definite
                .intersection(&other.definite)
                .copied()
                .collect(),
            possible: self.possible.union(&other.possible).copied().collect(),
            deferred,
        }
    }

    pub(super) fn extend(&mut self, other: Self) {
        self.definite.extend(other.definite);
        self.possible.extend(other.possible);
        self.deferred.extend(other.deferred);
    }

    pub(super) fn outer_deferred_writes(&self, before: &Self) -> bool {
        before
            .deferred
            .keys()
            .any(|slot| self.possible.contains(slot) && !before.possible.contains(slot))
    }
}
