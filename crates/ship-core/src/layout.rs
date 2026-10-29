//! Pane membership and order, stored once as leaves of a binary split tree.

use std::{iter, num::NonZeroUsize};

use mitsein::{iter1::IntoIterator1, vec_deque1::VecDeque1};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{id::IdOf, model::Pane, prelude::*};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Layout {
    Slot(Slot),
    Split(Split),
}

/// A place in the layout holding one or more panes. Only the front is drawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Slot {
    /// Front first. Never empty.
    #[schema(value_type = Vec<Pane>)]
    panes: VecDeque1<Pane>,
}

/// Next makes [X, A, B] into [A, B, X].
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum CycleDirection {
    Next,
    Prev,
}

impl Slot {
    pub fn new(pane: Pane) -> Self {
        Self {
            panes: VecDeque1::from_one(pane),
        }
    }

    pub fn front(&self) -> &Pane {
        self.panes.front()
    }

    /// Front first.
    pub fn panes(&self) -> impl Iterator<Item = &Pane> {
        self.panes.iter1().into_iter()
    }

    pub fn panes_mut(&mut self) -> impl Iterator<Item = &mut Pane> {
        self.panes.iter1_mut().into_iter()
    }

    pub fn len(&self) -> NonZeroUsize {
        self.panes.len()
    }

    pub fn contains(&self, pane: IdOf<Pane>) -> bool {
        self.panes().any(|item| item.id == pane)
    }

    /// The pane directly behind `pane`, if any.
    pub fn behind(&self, pane: IdOf<Pane>) -> Option<&Pane> {
        let index = self.panes().position(|item| item.id == pane)?;
        self.panes.get(index + 1)
    }

    /// Takes `pane` out, or returns the slot unchanged if it isn't here.
    pub fn take(mut self, pane: IdOf<Pane>) -> Result<Taken, Slot> {
        let Some(index) = self.panes().position(|item| item.id == pane) else {
            return Err(self);
        };
        match self.panes.remove_if_many(index).or_get() {
            Some(Ok(pane)) => Ok(Taken::Pane { pane, rest: self }),
            Some(Err(_)) => Ok(Taken::Last(self.panes.into_iter1().first())),
            None => unreachable!("the pane's index was found above"),
        }
    }
}

/// What `Slot::take` leaves.
pub enum Taken {
    Pane { pane: Pane, rest: Slot },
    Last(Pane),
}

/// First is left or above second, and receives `ratio` of the split area.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Split {
    pub axis: Axis,
    pub ratio: Ratio,
    #[schema(no_recursion)]
    pub first: Box<Layout>,
    #[schema(no_recursion)]
    pub second: Box<Layout>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// A finite first-child share of the total, strictly between zero and one.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "f32", into = "f32")]
#[schema(value_type = f32, exclusive_minimum = 0, exclusive_maximum = 1)]
pub struct Ratio(f32);

impl Ratio {
    pub const HALF: Self = Self(0.5);
}

impl TryFrom<f32> for Ratio {
    type Error = AppError;

    fn try_from(value: f32) -> Result<Self> {
        if value.is_finite() && value > 0.0 && value < 1.0 {
            Ok(Self(value))
        } else {
            Err(err!(
                Validation,
                "split ratio must be finite and between zero and one"
            ))
        }
    }
}

impl From<Ratio> for f32 {
    fn from(ratio: Ratio) -> Self {
        ratio.0
    }
}

impl Layout {
    /// Every pane, slot by slot in layout order, each slot front first.
    pub fn panes(&self) -> Box<dyn Iterator<Item = &Pane> + '_> {
        match self {
            Self::Slot(slot) => Box::new(slot.panes()),
            Self::Split(split) => Box::new(split.first.panes().chain(split.second.panes())),
        }
    }

    pub fn panes_mut(&mut self) -> Box<dyn Iterator<Item = &mut Pane> + '_> {
        match self {
            Self::Slot(slot) => Box::new(slot.panes_mut()),
            Self::Split(split) => Box::new(split.first.panes_mut().chain(split.second.panes_mut())),
        }
    }

    pub fn pane(&self, id: IdOf<Pane>) -> Option<&Pane> {
        self.panes().find(|pane| pane.id == id)
    }

    pub fn pane_mut(&mut self, id: IdOf<Pane>) -> Option<&mut Pane> {
        self.panes_mut().find(|pane| pane.id == id)
    }

    /// Slots in layout order: first child before second.
    pub fn slots(&self) -> Box<dyn Iterator<Item = &Slot> + '_> {
        match self {
            Self::Slot(slot) => Box::new(iter::once(slot)),
            Self::Split(split) => Box::new(split.first.slots().chain(split.second.slots())),
        }
    }

    pub fn slot(&self, pane: IdOf<Pane>) -> Option<&Slot> {
        self.slots().find(|slot| slot.contains(pane))
    }

    pub fn slot_mut(&mut self, pane: IdOf<Pane>) -> Option<&mut Slot> {
        match self {
            Self::Slot(slot) => slot.contains(pane).then_some(slot),
            Self::Split(split) => split
                .first
                .slot_mut(pane)
                .or_else(|| split.second.slot_mut(pane)),
        }
    }
}
