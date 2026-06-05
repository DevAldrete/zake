use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::{AppState, FocusPane, LayoutMode, Mode};
use crate::index::NoteIndex;

pub fn run(mut app: AppState) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_loop(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut AppState) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| draw(frame, app))?;
        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                handle_key(terminal, app, key)?;
            }
        }
    }
    Ok(())
}

fn handle_key(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut AppState,
    key: KeyEvent,
) -> Result<()> {
    match &mut app.mode {
        Mode::Command(input) => match key.code {
            KeyCode::Esc => app.mode = Mode::Normal,
            KeyCode::Enter => {
                let command = input.clone();
                app.mode = Mode::Normal;
                if command.trim() == "open" {
                    if let Err(err) = edit_with_terminal_restore(terminal, app) {
                        app.message = err.to_string();
                    }
                } else if let Err(err) = app.run_command(&command) {
                    app.message = err.to_string();
                }
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(ch) => input.push(ch),
            _ => {}
        },
        Mode::Find {
            query,
            previous_selection,
            selected_match,
        } => match key.code {
            KeyCode::Esc => {
                app.selected_note = *previous_selection;
                app.mode = Mode::Normal;
                app.message = "find cancelled".to_string();
            }
            KeyCode::Enter => {
                let matches = find_matches(&app.index, query);
                if let Some(note_idx) = matches.get(*selected_match) {
                    app.selected_note = *note_idx;
                }
                app.mode = Mode::Normal;
                app.focus = FocusPane::Notes;
                app.message = "find selected".to_string();
            }
            KeyCode::Backspace => {
                query.pop();
                *selected_match = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let len = find_matches(&app.index, query).len();
                *selected_match = move_find_selection(*selected_match, len, 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let len = find_matches(&app.index, query).len();
                *selected_match = move_find_selection(*selected_match, len, -1);
            }
            KeyCode::Char('o') => {
                let matches = find_matches(&app.index, query);
                if let Some(note_idx) = matches.get(*selected_match) {
                    app.selected_note = *note_idx;
                    app.mode = Mode::Normal;
                    if let Err(err) = edit_with_terminal_restore(terminal, app) {
                        app.message = err.to_string();
                    }
                }
            }
            KeyCode::Char(ch) => {
                query.push(ch);
                *selected_match = 0;
            }
            _ => {}
        },
        Mode::ConfirmDelete { note } => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let note = note.clone();
                app.mode = Mode::Normal;
                if let Err(err) = crate::note::delete_note(&note) {
                    app.message = err.to_string();
                } else {
                    app.message = format!("deleted {}", note.path.display());
                    app.refresh();
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.mode = Mode::Normal;
                app.message = "delete cancelled".to_string();
            }
            _ => {}
        },
        Mode::Normal => match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char(':') => app.mode = Mode::Command(String::new()),
            KeyCode::Enter => run_enter_action(terminal, app),
            KeyCode::Char('/') | KeyCode::Char('f') => {
                app.mode = Mode::Find {
                    query: String::new(),
                    previous_selection: app.selected_note,
                    selected_match: app.selected_note,
                };
                app.focus = FocusPane::Notes;
                app.message = "type to fuzzy-find notes".to_string();
            }
            KeyCode::Char('?') => app.message = "Keys: Enter act | Tab/Shift-Tab pane | w layout | m zoom | n new | l link | h history | d diff | z snapshot | s stage | u unstage | / find | : command".to_string(),
            KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
            KeyCode::Tab => app.next_focus(),
            KeyCode::BackTab => app.previous_focus(),
            KeyCode::Char('w') => app.cycle_layout(),
            KeyCode::Char('m') => app.toggle_zoom(),
            KeyCode::Char('r') => {
                app.refresh();
                app.message = "refreshed".to_string();
            }
            KeyCode::Char('n') => app.mode = Mode::Command("new ".to_string()),
            KeyCode::Char('l') => app.mode = Mode::Command(app.command_for_selected_link()),
            KeyCode::Char('h') => {
                if let Err(err) = app.run_command("history 25") {
                    app.message = err.to_string();
                }
            }
            KeyCode::Char('d') => {
                if let Err(err) = app.run_command("diff") {
                    app.message = err.to_string();
                }
            }
            KeyCode::Char('z') => {
                if let Err(err) = app.snapshot_now() {
                    app.message = err.to_string();
                }
            }
            KeyCode::Char('s') => {
                if let Err(err) = app.run_command("stage") {
                    app.message = err.to_string();
                }
            }
            KeyCode::Char('u') => {
                if let Err(err) = app.run_command("unstage") {
                    app.message = err.to_string();
                }
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Err(err) = app.run_command("stage-all") {
                    app.message = err.to_string();
                }
            }
            _ => {}
        },
    };
    Ok(())
}

fn run_enter_action(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut AppState) {
    let result = match app.focus {
        FocusPane::Notes | FocusPane::Metadata => edit_with_terminal_restore(terminal, app),
        FocusPane::Git => app.toggle_selected_git(),
        FocusPane::Search if !app.search_hits.is_empty() => edit_search_hit(terminal, app),
        FocusPane::Search => {
            app.message = "history rows are read-only; use h/history or d/diff".to_string();
            Ok(())
        }
    };
    if let Err(err) = result {
        app.message = err.to_string();
    }
}

fn edit_search_hit(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut AppState,
) -> Result<()> {
    suspend_terminal(terminal, |app| app.open_selected_search_hit(), app)
}

fn edit_with_terminal_restore(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut AppState,
) -> Result<()> {
    suspend_terminal(terminal, |app| app.edit_selected(), app)
}

fn suspend_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    action: impl FnOnce(&mut AppState) -> Result<()>,
    app: &mut AppState,
) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    let result = action(app);
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    result
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &AppState) {
    let root = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(root);
    draw_body(frame, app, vertical[0]);
    draw_status(frame, app, vertical[1]);
}

fn draw_body(frame: &mut ratatui::Frame<'_>, app: &AppState, area: Rect) {
    if let Some(pane) = app.zoomed {
        draw_pane(frame, app, pane, area);
        return;
    }

    match app.layout {
        LayoutMode::Columns => {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(28),
                    Constraint::Percentage(28),
                    Constraint::Percentage(22),
                    Constraint::Percentage(22),
                ])
                .split(area);
            draw_notes(frame, app, cols[0]);
            draw_metadata(frame, app, cols[1]);
            draw_git(frame, app, cols[2]);
            draw_search(frame, app, cols[3]);
        }
        LayoutMode::Workbench => {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
                .split(area);
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(44),
                    Constraint::Percentage(28),
                    Constraint::Percentage(28),
                ])
                .split(cols[1]);
            draw_notes(frame, app, cols[0]);
            draw_metadata(frame, app, right[0]);
            draw_git(frame, app, right[1]);
            draw_search(frame, app, right[2]);
        }
        LayoutMode::GitWide => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(area);
            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
                .split(rows[0]);
            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[1]);
            draw_notes(frame, app, top[0]);
            draw_metadata(frame, app, top[1]);
            draw_git(frame, app, bottom[0]);
            draw_search(frame, app, bottom[1]);
        }
        LayoutMode::Stack => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(28),
                    Constraint::Percentage(28),
                    Constraint::Percentage(22),
                    Constraint::Percentage(22),
                ])
                .split(area);
            draw_notes(frame, app, rows[0]);
            draw_metadata(frame, app, rows[1]);
            draw_git(frame, app, rows[2]);
            draw_search(frame, app, rows[3]);
        }
    }
}

fn draw_pane(frame: &mut ratatui::Frame<'_>, app: &AppState, pane: FocusPane, area: Rect) {
    match pane {
        FocusPane::Notes => draw_notes(frame, app, area),
        FocusPane::Metadata => draw_metadata(frame, app, area),
        FocusPane::Git => draw_git(frame, app, area),
        FocusPane::Search => draw_search(frame, app, area),
    }
}

fn draw_notes(frame: &mut ratatui::Frame<'_>, app: &AppState, area: Rect) {
    let find_state = match &app.mode {
        Mode::Find {
            query,
            selected_match,
            ..
        } => Some((query.as_str(), *selected_match)),
        _ => None,
    };
    let note_indices = find_state.map_or_else(
        || (0..app.index.notes.len()).collect::<Vec<_>>(),
        |(query, _)| app.find_matches(query),
    );
    let items = note_indices
        .iter()
        .enumerate()
        .filter_map(|(row_idx, note_idx)| {
            let note = app.index.notes.get(*note_idx)?;
            let marker = if find_state.is_some_and(|(_, selected_match)| selected_match == row_idx)
                || (find_state.is_none() && *note_idx == app.selected_note)
            {
                ">"
            } else {
                " "
            };
            Some(ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::raw(&note.meta.title),
            ])))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(block(
            "Notes  Enter edit  / find",
            app.focus == FocusPane::Notes,
        )),
        area,
    );
}

fn draw_metadata(frame: &mut ratatui::Frame<'_>, app: &AppState, area: Rect) {
    let lines = if let Some(note) = app.selected_note() {
        vec![
            Line::from(vec![
                Span::styled("Title: ", label()),
                Span::raw(&note.meta.title),
            ]),
            Line::from(vec![
                Span::styled("Type: ", label()),
                Span::raw(&note.meta.kind),
            ]),
            Line::from(vec![
                Span::styled("Tags: ", label()),
                Span::raw(note.meta.tags.join(", ")),
            ]),
            Line::from(vec![
                Span::styled("Links: ", label()),
                Span::raw(note.meta.links.join(", ")),
            ]),
            Line::from(vec![
                Span::styled("Path: ", label()),
                Span::raw(note.path.display().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Created: ", label()),
                Span::raw(note.meta.created_at.to_rfc3339()),
            ]),
            Line::from(vec![
                Span::styled("Updated: ", label()),
                Span::raw(note.meta.updated_at.to_rfc3339()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Broken: ", label()),
                Span::raw(
                    app.index
                        .broken_links
                        .get(&note.path)
                        .map(|v| v.join(", "))
                        .unwrap_or_default(),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Actions: ", label()),
                Span::raw("Enter edit | l link command | :rename | :move | :delete"),
            ]),
        ]
    } else {
        vec![Line::from("No notes yet. Press n or run :new <title>.")]
    };

    frame.render_widget(
        Paragraph::new(lines)
            .block(block(
                "Metadata  Enter edit",
                app.focus == FocusPane::Metadata,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_git(frame: &mut ratatui::Frame<'_>, app: &AppState, area: Rect) {
    let mut items = app
        .git
        .files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let marker = if idx == app.selected_git { ">" } else { " " };
            let xy = format!(
                "{}{}",
                file.staged.unwrap_or(' '),
                file.unstaged.unwrap_or(' ')
            );
            ListItem::new(format!("{marker} {xy} {}", file.path.display()))
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        items.push(ListItem::new("clean"));
    }
    frame.render_widget(
        List::new(items).block(block(
            "Git  Enter toggle  s stage  u unstage  z snapshot",
            app.focus == FocusPane::Git,
        )),
        area,
    );
}

fn draw_search(frame: &mut ratatui::Frame<'_>, app: &AppState, area: Rect) {
    let mut lines = app
        .search_hits
        .iter()
        .enumerate()
        .map(|(idx, hit)| {
            let marker = if idx == app.selected_search { ">" } else { " " };
            ListItem::new(format!(
                "{marker} {}:{} {}",
                hit.path.display(),
                hit.line,
                hit.text
            ))
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        lines.extend(app.history.iter().enumerate().map(|(idx, line)| {
            let marker = if idx == app.selected_search { ">" } else { " " };
            ListItem::new(format!("{marker} {line}"))
        }));
    }
    frame.render_widget(
        List::new(lines).block(block(
            "Search / History  Enter hit  h log  d diff",
            app.focus == FocusPane::Search,
        )),
        area,
    );
}

fn draw_status(frame: &mut ratatui::Frame<'_>, app: &AppState, area: Rect) {
    let text = match &app.mode {
        Mode::Normal => format!(
            "{} | pane {} | layout {} | {} notes | {} changes",
            app.message,
            app.focus.label(),
            app.layout.label(),
            app.index.notes.len(),
            app.git.files.len()
        ),
        Mode::Command(input) => format!(":{input}"),
        Mode::Find { query, .. } => format!("/{query}"),
        Mode::ConfirmDelete { note } => format!(
            "Delete \"{}\" at {}? y/N",
            note.meta.title,
            note.path.display()
        ),
    };
    frame.render_widget(
        Paragraph::new(Text::from(text))
            .block(Block::default().borders(Borders::ALL).title("Command"))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn block(title: &'static str, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(style)
}

fn label() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

fn move_find_selection(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    current.saturating_add_signed(delta).min(len - 1)
}

fn find_matches(index: &NoteIndex, query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..index.notes.len()).collect();
    }
    index
        .fuzzy_notes(query)
        .into_iter()
        .map(|matched| matched.index)
        .collect()
}
