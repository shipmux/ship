//! The mode machine: terminal events to bindings in the active mode.

use crokey::KeyCombination;
use crossterm::event::{Event, KeyEventKind};
use ship_core::prelude::*;

use super::lane::Item;
use crate::keymap::{Binding, Keymap, ModeKind, ModeName};

pub(super) struct Keys {
    pub keymap: Keymap,
    pub active: ModeName,
}

impl Keys {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            active: ModeName::normal(),
        }
    }

    /// `KeyCombination::from(key)`, looked up in the active mode. Unbound:
    /// a program key in normal, dropped in any other mode. A one-shot mode
    /// returns to normal after any key unless the binding enters a mode itself.
    /// What the input means, as `Item::Action`, `Item::Key` for the selected
    /// pane's program or `Item::Paste`; `None` when unbound outside normal,
    /// or not a key or paste. The lane decides when it runs.
    pub fn handle(&mut self, event: Event) -> Option<Item> {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                let Some(mode) = self.keymap.modes.get(&self.active) else {
                    self.active = ModeName::normal();
                    return None;
                };
                let binding = mode.keys.get(&KeyCombination::from(key)).cloned();
                let normal = self.active.0 == ModeName::NORMAL;
                if mode.kind == ModeKind::Oneshot
                    && !binding.as_ref().is_some_and(Binding::enters_mode)
                {
                    self.active = ModeName::normal();
                }
                match binding.and_then(|binding| binding.action) {
                    Some(action) => Some(Item::Action(action)),
                    None => normal.then_some(Item::Key(key)),
                }
            }
            Event::Paste(text) => Some(Item::Paste(text)),
            _ => None,
        }
    }

    pub fn enter(&mut self, mode: ModeName) -> Result<()> {
        if !self.keymap.modes.contains_key(&mode) {
            return Err(err!(Configuration, "no mode named \"{}\"", mode.0));
        }
        self.active = mode;
        Ok(())
    }

    /// Swap in a reloaded keymap; an active mode it no longer defines falls
    /// back to normal.
    pub fn replace(&mut self, keymap: Keymap) {
        if !keymap.modes.contains_key(&self.active) {
            self.active = ModeName::normal();
        }
        self.keymap = keymap;
    }

    /// The active mode's name, outside normal.
    pub fn shown_mode(&self) -> Option<&ModeName> {
        Some(&self.active).filter(|mode| mode.0 != ModeName::NORMAL)
    }
}
