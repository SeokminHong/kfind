use std::fs::File;
use std::io::{self, Write};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{
    self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};
use crossterm::{execute, queue};

use super::PagerEvent;
use super::viewport::{Document, Layout, RowKey, truncate_end};
use crate::Language;

const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(16);

pub(super) fn present_live(
    file: File,
    events: Receiver<PagerEvent>,
    ready: Sender<io::Result<()>>,
) -> io::Result<()> {
    let mut document = match Document::open(file) {
        Ok(document) => document,
        Err(error) => return reject_start(ready, error),
    };
    let (width, height) = match terminal::size() {
        Ok(size) => size,
        Err(error) => return reject_start(ready, error),
    };
    let mut output = io::stdout();
    let guard = match ScreenGuard::enter(&mut output) {
        Ok(guard) => guard,
        Err(error) => return reject_start(ready, error),
    };
    if ready.send(Ok(())).is_err() {
        return Ok(());
    }

    let language = Language::from_env();
    let result = run_tui(&mut document, &events, &mut output, language, width, height);
    drop(guard);

    match result {
        Ok(view) => write_layout(&mut document, &view.layout, view.offset, view.output_height),
        Err(_) => {
            let _ = document.refresh();
            write_full(&mut document)
        }
    }
}

fn reject_start(ready: Sender<io::Result<()>>, error: io::Error) -> io::Result<()> {
    let reported = io::Error::new(error.kind(), error.to_string());
    let _ = ready.send(Err(reported));
    Err(error)
}

struct FinalView {
    layout: Layout,
    offset: usize,
    output_height: usize,
}

fn run_tui(
    document: &mut Document,
    events: &Receiver<PagerEvent>,
    output: &mut io::Stdout,
    language: Language,
    mut width: u16,
    mut height: u16,
) -> io::Result<FinalView> {
    let mut layout = document.layout(usize::from(width))?;
    let mut offset = 0;
    let mut anchor = row_key(&layout, offset);
    let mut done = false;
    let mut dirty = true;

    loop {
        if event::poll(Duration::ZERO)?
            && handle_input(
                document,
                event::read()?,
                &mut layout,
                &mut offset,
                &mut anchor,
                &mut width,
                &mut height,
                &mut dirty,
            )?
        {
            break;
        }

        match events.try_recv() {
            Ok(PagerEvent::Data) => {
                extend_with_new_sources(document, &mut layout)?;
                dirty = true;
            }
            Ok(PagerEvent::Done) => {
                extend_with_new_sources(document, &mut layout)?;
                done = true;
                dirty = true;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                extend_with_new_sources(document, &mut layout)?;
                done = true;
                dirty = true;
            }
        }

        if done && !layout.truncated && layout.rows.len() <= usize::from(height) {
            return Ok(FinalView {
                output_height: layout.rows.len(),
                layout,
                offset: 0,
            });
        }

        if dirty {
            offset = offset.min(layout.rows.len().saturating_sub(1));
            draw(
                document, output, &layout, offset, width, height, language, done,
            )?;
            dirty = false;
        }

        if !event::poll(INPUT_POLL_INTERVAL)? {
            continue;
        }
        if handle_input(
            document,
            event::read()?,
            &mut layout,
            &mut offset,
            &mut anchor,
            &mut width,
            &mut height,
            &mut dirty,
        )? {
            break;
        }
    }

    Ok(FinalView {
        layout,
        offset,
        output_height: page_height(height),
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_input(
    document: &mut Document,
    event: Event,
    layout: &mut Layout,
    offset: &mut usize,
    anchor: &mut Option<RowKey>,
    width: &mut u16,
    height: &mut u16,
    dirty: &mut bool,
) -> io::Result<bool> {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                *offset = offset.saturating_sub(1);
                *anchor = row_key(layout, *offset);
                *dirty = true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                *offset = (*offset + 1).min(layout.rows.len().saturating_sub(1));
                *anchor = row_key(layout, *offset);
                *dirty = true;
            }
            KeyCode::PageUp => {
                *offset = offset.saturating_sub(page_height(*height));
                *anchor = row_key(layout, *offset);
                *dirty = true;
            }
            KeyCode::PageDown => {
                *offset = (*offset + page_height(*height)).min(layout.rows.len().saturating_sub(1));
                *anchor = row_key(layout, *offset);
                *dirty = true;
            }
            KeyCode::Home => {
                *offset = 0;
                *anchor = row_key(layout, *offset);
                *dirty = true;
            }
            KeyCode::End => {
                *offset = layout.rows.len().saturating_sub(1);
                *anchor = row_key(layout, *offset);
                *dirty = true;
            }
            _ => {}
        },
        Event::Resize(new_width, new_height) => {
            *width = new_width;
            *height = new_height;
            extend_with_new_sources(document, layout)?;
            *layout = document.layout(usize::from(*width))?;
            *offset = anchor.map_or(0, |key| layout.locate(key));
            *dirty = true;
        }
        _ => {}
    }
    Ok(false)
}

fn extend_with_new_sources(document: &mut Document, layout: &mut Layout) -> io::Result<()> {
    let sources = document.refresh()?;
    document.extend_layout(layout, sources)
}

fn row_key(layout: &Layout, offset: usize) -> Option<RowKey> {
    layout.rows.get(offset).copied()
}

fn page_height(height: u16) -> usize {
    if height > 1 {
        usize::from(height - 1)
    } else {
        1
    }
}

#[allow(clippy::too_many_arguments)]
fn draw(
    document: &mut Document,
    output: &mut io::Stdout,
    layout: &Layout,
    offset: usize,
    width: u16,
    height: u16,
    language: Language,
    done: bool,
) -> io::Result<()> {
    queue!(output, MoveTo(0, 0), Clear(ClearType::All))?;
    let visible = page_height(height);
    for (screen_row, row) in layout.rows.iter().skip(offset).take(visible).enumerate() {
        let text = document.render_row(*row, usize::from(width))?;
        queue!(
            output,
            MoveTo(0, screen_row as u16),
            Print(text),
            Clear(ClearType::UntilNewLine)
        )?;
    }

    if height > 1 {
        let current = if layout.rows.is_empty() {
            0
        } else {
            offset + 1
        };
        let status = status_text(language, done, current, layout.rows.len());
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
    output.flush()
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

fn write_full(document: &mut Document) -> io::Result<()> {
    let mut output = io::stdout().lock();
    for source in 0..document.source_count() {
        writeln!(output, "{}", document.full_row(source)?)?;
    }
    Ok(())
}

fn write_layout(
    document: &mut Document,
    layout: &Layout,
    offset: usize,
    limit: usize,
) -> io::Result<()> {
    let mut output = io::stdout().lock();
    for row in layout.rows.iter().skip(offset).take(limit) {
        writeln!(output, "{}", document.render_row(*row, layout.width)?)?;
    }
    Ok(())
}

struct ScreenGuard;

impl ScreenGuard {
    fn enter(output: &mut io::Stdout) -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(output, EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for ScreenGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
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
}
