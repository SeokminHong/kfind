use std::io::{self, Write};

use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{Clear, ClearType, ScrollDown, ScrollUp};

use super::viewport::{Document, Layout, truncate_end};
use crate::Language;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FrameState {
    offset: usize,
    rows: usize,
    width: u16,
    height: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Redraw {
    Full,
    Rows { start: usize, end: usize },
    ScrollUp(usize),
    ScrollDown(usize),
    Status,
}

pub(super) struct Renderer {
    previous: Option<FrameState>,
}

impl Renderer {
    pub(super) const fn new() -> Self {
        Self { previous: None }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw(
        &mut self,
        document: &mut Document,
        output: &mut io::Stdout,
        layout: &Layout,
        offset: usize,
        width: u16,
        height: u16,
        language: Language,
        done: bool,
    ) -> io::Result<()> {
        let visible = content_height(height);
        let current = FrameState {
            offset,
            rows: layout.rows.len(),
            width,
            height,
        };
        match redraw(self.previous, current, visible) {
            Redraw::Full => {
                queue!(output, MoveTo(0, 0), Clear(ClearType::All))?;
                draw_rows(document, output, layout, offset, width, 0, visible)?;
            }
            Redraw::Rows { start, end } => {
                draw_rows(document, output, layout, offset, width, start, end)?;
            }
            Redraw::ScrollUp(rows) => {
                queue!(output, ScrollUp(rows as u16))?;
                draw_rows(
                    document,
                    output,
                    layout,
                    offset,
                    width,
                    visible - rows,
                    visible,
                )?;
            }
            Redraw::ScrollDown(rows) => {
                queue!(output, ScrollDown(rows as u16))?;
                draw_rows(document, output, layout, offset, width, 0, rows)?;
            }
            Redraw::Status => {}
        }

        if height > 1 {
            let current_row = if layout.rows.is_empty() {
                0
            } else {
                offset + 1
            };
            let status = status_text(language, done, current_row, layout.rows.len());
            let (status, _) = truncate_end(&status, usize::from(width));
            queue!(
                output,
                MoveTo(0, height - 1),
                SetAttribute(Attribute::Reverse),
                Print(status),
                SetAttribute(Attribute::Reset),
                Clear(ClearType::UntilNewLine)
            )?;
        }
        output.flush()?;
        self.previous = Some(current);
        Ok(())
    }
}

pub(super) fn content_height(height: u16) -> usize {
    if height > 1 {
        usize::from(height - 1)
    } else {
        1
    }
}

fn redraw(previous: Option<FrameState>, current: FrameState, visible: usize) -> Redraw {
    let Some(previous) = previous else {
        return Redraw::Full;
    };
    if previous.width != current.width || previous.height != current.height {
        return Redraw::Full;
    }
    if previous.offset == current.offset {
        if previous.rows < current.rows {
            let start = previous.rows.saturating_sub(current.offset).min(visible);
            let end = current.rows.saturating_sub(current.offset).min(visible);
            return if start < end {
                Redraw::Rows { start, end }
            } else {
                Redraw::Status
            };
        }
        return if previous.rows == current.rows {
            Redraw::Status
        } else {
            Redraw::Full
        };
    }
    if previous.rows != current.rows {
        return Redraw::Full;
    }
    if current.offset > previous.offset {
        let rows = current.offset - previous.offset;
        return if rows <= visible {
            Redraw::ScrollUp(rows)
        } else {
            Redraw::Full
        };
    }
    let rows = previous.offset - current.offset;
    if rows <= visible {
        Redraw::ScrollDown(rows)
    } else {
        Redraw::Full
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_rows(
    document: &mut Document,
    output: &mut io::Stdout,
    layout: &Layout,
    offset: usize,
    width: u16,
    start: usize,
    end: usize,
) -> io::Result<()> {
    for screen_row in start..end {
        queue!(output, MoveTo(0, screen_row as u16))?;
        if let Some(row) = layout.rows.get(offset + screen_row) {
            let text = document.render_row(*row, usize::from(width))?;
            queue!(output, Print(text), Clear(ClearType::UntilNewLine))?;
        } else {
            queue!(output, Clear(ClearType::CurrentLine))?;
        }
    }
    Ok(())
}

fn status_text(language: Language, done: bool, current: usize, total: usize) -> String {
    match (language, done) {
        (Language::Korean, false) => {
            format!("검색 중 · {current}/{total}  ↑↓/jk 이동  q/Esc 종료")
        }
        (Language::Korean, true) => format!("{current}/{total}  ↑↓/jk 이동  q/Esc 종료"),
        (Language::English, false) => {
            format!("searching · {current}/{total}  ↑↓/jk move  q/Esc quit")
        }
        (Language::English, true) => format!("{current}/{total}  ↑↓/jk move  q/Esc quit"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_text_tracks_search_completion_and_locale() {
        assert!(status_text(Language::Korean, false, 1, 3).starts_with("검색 중"));
        assert!(status_text(Language::English, false, 1, 3).starts_with("searching"));
        assert!(!status_text(Language::English, true, 1, 3).contains("searching"));
    }

    #[test]
    fn redraw_scrolls_only_the_rows_exposed_by_navigation() {
        let previous = FrameState {
            offset: 10,
            rows: 100,
            width: 80,
            height: 25,
        };

        assert_eq!(
            redraw(
                Some(previous),
                FrameState {
                    offset: 11,
                    ..previous
                },
                24,
            ),
            Redraw::ScrollUp(1)
        );
        assert_eq!(
            redraw(
                Some(previous),
                FrameState {
                    offset: 9,
                    ..previous
                },
                24,
            ),
            Redraw::ScrollDown(1)
        );
    }

    #[test]
    fn appended_rows_redraw_only_when_they_enter_the_viewport() {
        let previous = FrameState {
            offset: 0,
            rows: 20,
            width: 80,
            height: 25,
        };

        assert_eq!(
            redraw(
                Some(previous),
                FrameState {
                    rows: 22,
                    ..previous
                },
                24,
            ),
            Redraw::Rows { start: 20, end: 22 }
        );

        let below_viewport = FrameState {
            rows: 100,
            ..previous
        };
        assert_eq!(
            redraw(
                Some(below_viewport),
                FrameState {
                    rows: 102,
                    ..below_viewport
                },
                24,
            ),
            Redraw::Status
        );
    }
}
