use std::fs::File;
use std::io::{self, Write};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    self, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use super::PagerEvent;
use super::render::{Renderer, content_height};
use super::viewport::{Document, Layout, RowKey};
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
    let mut renderer = Renderer::new();

    loop {
        let mut quit = false;
        while event::poll(Duration::ZERO)? {
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
                quit = true;
                break;
            }
        }
        if quit {
            break;
        }

        let mut refresh = false;
        loop {
            match events.try_recv() {
                Ok(PagerEvent::Data) => refresh = true,
                Ok(PagerEvent::Done) => {
                    refresh = true;
                    done = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    refresh = true;
                    done = true;
                    break;
                }
            }
        }
        if refresh {
            extend_with_new_sources(document, &mut layout)?;
            dirty = true;
        }

        if done && !layout.truncated && layout.rows.len() <= usize::from(height) {
            return Ok(FinalView {
                output_height: layout.rows.len(),
                layout,
                offset: 0,
            });
        }

        if dirty {
            offset = offset.min(max_offset(&layout, height));
            anchor = row_key(&layout, offset);
            renderer.draw(
                document, output, &layout, offset, width, height, language, done,
            )?;
            dirty = false;
        }

        let _ = event::poll(INPUT_POLL_INTERVAL)?;
    }

    Ok(FinalView {
        layout,
        offset,
        output_height: content_height(height),
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
                set_offset(
                    layout,
                    offset,
                    anchor,
                    dirty,
                    offset.saturating_sub(1),
                    *height,
                );
            }
            KeyCode::Down | KeyCode::Char('j') => {
                set_offset(
                    layout,
                    offset,
                    anchor,
                    dirty,
                    offset.saturating_add(1),
                    *height,
                );
            }
            KeyCode::PageUp => {
                set_offset(
                    layout,
                    offset,
                    anchor,
                    dirty,
                    offset.saturating_sub(content_height(*height)),
                    *height,
                );
            }
            KeyCode::PageDown => {
                set_offset(
                    layout,
                    offset,
                    anchor,
                    dirty,
                    offset.saturating_add(content_height(*height)),
                    *height,
                );
            }
            KeyCode::Home => {
                set_offset(layout, offset, anchor, dirty, 0, *height);
            }
            KeyCode::End => {
                set_offset(
                    layout,
                    offset,
                    anchor,
                    dirty,
                    max_offset(layout, *height),
                    *height,
                );
            }
            _ => {}
        },
        Event::Resize(new_width, new_height) => {
            *width = new_width;
            *height = new_height;
            document.refresh()?;
            *layout = document.layout(usize::from(*width))?;
            *offset = anchor
                .map_or(0, |key| layout.locate(key))
                .min(max_offset(layout, *height));
            *anchor = row_key(layout, *offset);
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

fn set_offset(
    layout: &Layout,
    offset: &mut usize,
    anchor: &mut Option<RowKey>,
    dirty: &mut bool,
    next: usize,
    height: u16,
) {
    let next = next.min(max_offset(layout, height));
    if next == *offset {
        return;
    }
    *offset = next;
    *anchor = row_key(layout, next);
    *dirty = true;
}

fn max_offset(layout: &Layout, height: u16) -> usize {
    layout.rows.len().saturating_sub(content_height(height))
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

    fn layout(rows: usize) -> Layout {
        Layout {
            rows: (0..rows)
                .map(|source| RowKey {
                    source,
                    target: None,
                })
                .collect(),
            truncated: false,
            width: 80,
        }
    }

    #[test]
    fn offset_stops_when_the_last_row_reaches_the_viewport_bottom() {
        let layout = layout(10);

        assert_eq!(content_height(5), 4);
        assert_eq!(max_offset(&layout, 5), 6);
        assert_eq!(max_offset(&layout, 12), 0);
        assert_eq!(max_offset(&layout, 1), 9);
    }

    #[test]
    fn navigation_at_a_viewport_boundary_does_not_request_a_redraw() {
        let layout = layout(10);
        let mut offset = max_offset(&layout, 5);
        let mut anchor = row_key(&layout, offset);
        let mut dirty = false;

        set_offset(&layout, &mut offset, &mut anchor, &mut dirty, 9, 5);

        assert_eq!(offset, 6);
        assert_eq!(anchor, row_key(&layout, 6));
        assert!(!dirty);
    }
}
