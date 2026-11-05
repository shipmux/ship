//! Sidebar lines fitted to their width, shared by the tab and agent
//! sections (amendment P4).

use ratatui::text::Span;

const ELLIPSIS: &str = "…";

/// One span of a value, and how it gives up width.
pub(super) struct Piece {
    pub span: Span<'static>,
    pub cut: Cut,
}

#[derive(Clone, Copy)]
pub(super) enum Cut {
    /// Keeps its width, like an icon or the zoom marker.
    Keep,
    /// Loses its start, like a path.
    Start,
    /// Loses its end.
    End,
}

impl Piece {
    pub fn keep(span: Span<'static>) -> Self {
        Self {
            span,
            cut: Cut::Keep,
        }
    }

    pub fn end(span: Span<'static>) -> Self {
        Self {
            span,
            cut: Cut::End,
        }
    }
}

/// `values` joined by `separator` in `width` cells. Kept pieces hold their
/// width, and the rest take turns a cell at a time, as in Herdr, so short
/// values stay whole and long ones are cut with `…` in their own style. A
/// value left no cells goes with its separator.
pub(super) fn line(
    values: Vec<Vec<Piece>>,
    separator: &'static str,
    width: usize,
) -> Vec<Span<'static>> {
    let pieces: Vec<&Piece> = values.iter().flatten().collect();
    let mut shares: Vec<usize> = pieces
        .iter()
        .map(|piece| match piece.cut {
            Cut::Keep => piece.span.width(),
            Cut::Start | Cut::End => 0,
        })
        .collect();
    let separators = values.len().saturating_sub(1) * Span::raw(separator).width();
    let mut room = width.saturating_sub(shares.iter().sum::<usize>() + separators);
    while room > 0 {
        let before = room;
        for (share, piece) in shares.iter_mut().zip(&pieces) {
            if room > 0 && *share < piece.span.width() {
                *share += 1;
                room -= 1;
            }
        }
        if room == before {
            break;
        }
    }
    let mut shares = shares.into_iter();
    let mut spans = Vec::new();
    for value in values {
        let fitted: Vec<_> = value
            .into_iter()
            .zip(shares.by_ref())
            .filter(|(_, share)| *share > 0)
            .map(|(piece, share)| cut(piece, share))
            .collect();
        if fitted.is_empty() {
            continue;
        }
        if !spans.is_empty() {
            spans.push(Span::raw(separator));
        }
        spans.extend(fitted);
    }
    spans
}

fn cut(piece: Piece, share: usize) -> Span<'static> {
    let content = match piece.cut {
        Cut::Keep => return piece.span,
        Cut::Start => shorten(&piece.span.content, share),
        Cut::End => truncate_end(&piece.span.content, share),
    };
    Span::styled(content, piece.span.style)
}

/// `path` cut from the left behind an ellipsis to fit `room` cells.
fn shorten(path: &str, room: usize) -> String {
    if Span::raw(path).width() <= room {
        return path.to_owned();
    }
    let room = room.saturating_sub(Span::raw(ELLIPSIS).width());
    let start = path
        .char_indices()
        .map(|(index, _)| index)
        .find(|index| Span::raw(&path[*index..]).width() <= room)
        .unwrap_or(path.len());
    format!("{ELLIPSIS}{}", &path[start..])
}

/// `text` cut at the end behind an ellipsis to fit `room` cells.
fn truncate_end(text: &str, room: usize) -> String {
    if Span::raw(text).width() <= room {
        return text.to_owned();
    }
    let room = room.saturating_sub(Span::raw(ELLIPSIS).width());
    let end = text
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| Span::raw(&text[..*index]).width() <= room)
        .last()
        .unwrap_or(0);
    format!("{}{ELLIPSIS}", &text[..end])
}
