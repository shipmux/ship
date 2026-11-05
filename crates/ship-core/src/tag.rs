//! Tags: short display text that scripts put on tabs and panes.

use std::{
    fmt,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use indexmap::IndexMap;
use ratatui_core::style::Color;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{
        Ref, RefOr,
        schema::{ObjectBuilder, OneOfBuilder, Schema, Type},
    },
};

use crate::{model::disarm, prelude::*};

/// Any non-empty string without `=` or control characters (FR-006).
/// `.` means nothing to Ship; `plugin.key` is a documented convention.
/// No serde derives: fields holding one use `serde_as` with
/// `DisplayFromStr`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TagName(String);

impl TagName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for TagName {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self> {
        if value.is_empty() {
            return Err(err!(Validation, "tag name must not be empty"));
        }
        if value.contains('=') {
            return Err(err!(Validation, "tag name must not contain '='"));
        }
        if value.chars().any(char::is_control) {
            return Err(err!(
                Validation,
                "tag name must not contain control characters"
            ));
        }
        Ok(Self(value.to_owned()))
    }
}

impl fmt::Display for TagName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A tag's value and optional colors. One shape in `get` output, PATCH
/// bodies and `watch`. Writing a tag replaces all three (FR-009).
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Tag {
    #[serde_as(as = "DisplayFromStr")]
    pub value: TagValue,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::protocol::present"
    )]
    #[schema(nullable = false)]
    pub fg: Option<NamedColor>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::protocol::present"
    )]
    #[schema(nullable = false)]
    pub bg: Option<NamedColor>,
}

/// Any string without control characters, `""` included, with
/// bidirectional formatting characters removed (amendment P1). Cleaned as
/// it's parsed, so a value that differs only by those characters is no
/// change. No serde derives, like `TagName`.
#[derive(Clone, Debug, PartialEq, Eq, ToSchema)]
pub struct TagValue(String);

impl TagValue {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for TagValue {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self> {
        if value.chars().any(char::is_control) {
            return Err(err!(
                Validation,
                "tag value must not contain control characters"
            ));
        }
        Ok(Self(disarm(value.to_owned())))
    }
}

impl fmt::Display for TagValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The 16 terminal colors, by their ANSI names. Its own enum because both
/// colors core already has accept more: `screen::Color` takes any palette
/// index or RGB, and ratatui's `Color` takes hex, indexes and `reset`, under
/// names where `white` means bright white.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NamedColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

/// Onto ratatui's names: `white` is `Gray`, `bright_black` is `DarkGray`,
/// `bright_white` is `White`.
impl From<NamedColor> for Color {
    fn from(color: NamedColor) -> Self {
        match color {
            NamedColor::Black => Self::Black,
            NamedColor::Red => Self::Red,
            NamedColor::Green => Self::Green,
            NamedColor::Yellow => Self::Yellow,
            NamedColor::Blue => Self::Blue,
            NamedColor::Magenta => Self::Magenta,
            NamedColor::Cyan => Self::Cyan,
            NamedColor::White => Self::Gray,
            NamedColor::BrightBlack => Self::DarkGray,
            NamedColor::BrightRed => Self::LightRed,
            NamedColor::BrightGreen => Self::LightGreen,
            NamedColor::BrightYellow => Self::LightYellow,
            NamedColor::BrightBlue => Self::LightBlue,
            NamedColor::BrightMagenta => Self::LightMagenta,
            NamedColor::BrightCyan => Self::LightCyan,
            NamedColor::BrightWhite => Self::White,
        }
    }
}

/// A tab's or pane's tags. Its equality ignores order, so re-setting a tag
/// in place is still a no-op. Named so the `serde_as` key adapter is
/// written once, rather than on `Pane` and `Tab`.
#[serde_as]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tags(#[serde_as(as = "IndexMap<DisplayFromStr, _>")] IndexMap<TagName, Tag>);

/// By hand like `TagPatch`'s: the derive's map path trips
/// `unused_qualifications` in code an `expect` can't reach.
impl PartialSchema for Tags {
    fn schema() -> RefOr<Schema> {
        ObjectBuilder::new()
            .additional_properties(Some(Ref::from_schema_name(Tag::name())))
            .into()
    }
}

impl ToSchema for Tags {
    fn schemas(schemas: &mut Vec<(String, RefOr<Schema>)>) {
        schemas.push((Tag::name().into(), Tag::schema()));
        Tag::schemas(schemas);
    }
}

impl Deref for Tags {
    type Target = IndexMap<TagName, Tag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tags {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Tags {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Apply a PATCH body's `tags`: `None` (`tags: null`) removes all.
    pub fn patch(&mut self, patch: Option<TagPatch>) {
        let Some(TagPatch(entries)) = patch else {
            return self.0.clear();
        };
        for (name, tag) in entries {
            match tag {
                Some(tag) => {
                    self.0.insert(name, tag);
                }
                None => {
                    self.0.shift_remove(&name);
                }
            }
        }
    }
}

/// PATCH's `tags`: a tag sets its name, `null` removes it, and an absent
/// name is unchanged (RFC 7396). Named because `serde_as` on `SetPane`'s
/// `Option<Option<…>>` would read `tags: null` as absent, so that field
/// keeps `double_option` and the key adapter lives in here.
#[serde_as]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TagPatch(#[serde_as(as = "IndexMap<DisplayFromStr, _>")] IndexMap<TagName, Option<Tag>>);

/// An object of nullable `Tag`s, by hand: the derive builds a `oneOf` for
/// the `Option`, which utoipa can't take as `additionalProperties`.
impl PartialSchema for TagPatch {
    fn schema() -> RefOr<Schema> {
        let tag = OneOfBuilder::new()
            .item(Ref::from_schema_name(Tag::name()))
            .item(ObjectBuilder::new().schema_type(Type::Null));
        ObjectBuilder::new()
            .additional_properties(Some(Schema::OneOf(tag.build())))
            .into()
    }
}

impl ToSchema for TagPatch {
    fn schemas(schemas: &mut Vec<(String, RefOr<Schema>)>) {
        schemas.push((Tag::name().into(), Tag::schema()));
        Tag::schemas(schemas);
    }
}

/// `--tag` entries collect into a patch; the last occurrence of a name wins.
impl FromIterator<(TagName, Option<Tag>)> for TagPatch {
    fn from_iter<I: IntoIterator<Item = (TagName, Option<Tag>)>>(entries: I) -> Self {
        Self(entries.into_iter().collect())
    }
}
