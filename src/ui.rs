use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use himalaya_lib::Backend;
use tokio::sync::RwLock;
use tracing::{info, trace};
use tui::{
    backend::{Backend as TuiBackend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use unicode_truncate::UnicodeTruncateStr;

use crate::email::{backend, get_emails};
use crate::{
    app::{App, AppFocus},
    email::EmailFlag,
};

pub async fn run(tick_rate: Duration) -> anyhow::Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = Arc::new(RwLock::new(App::new()));

    let state = app.clone();
    tokio::spawn(async move {
        trace!("Starting email reader task");
        {
            let mut app = state.write().await;
            app.loading = true;
        }

        info!("Reading emails...");
        let emails = get_emails().expect("should be able to get emails");
        info!("Emails: {emails:#?}");

        {
            let mut app = state.write().await;
            app.emails = emails;
            app.loading = false;
        }
        trace!("Finished email reader task");
    });

    run_app(&mut terminal, &app, tick_rate).await?;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

pub async fn run_app<B: TuiBackend>(
    terminal: &mut Terminal<B>,
    app_arc: &Arc<RwLock<App>>,
    tick_rate: Duration,
) -> anyhow::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        {
            {
                let mut app = app_arc.write().await;
                terminal.draw(|f| ui(f, &mut app))?;
            }

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if crossterm::event::poll(timeout)? {
                if let Event::Key(event) = event::read()? {
                    trace!("event = {:?}", event);

                    match event.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(' ') => {
                            let mut app = app_arc.write().await;
                            app.toggle_spam();
                            app.down();
                        }
                        KeyCode::Char('s') => {
                            {
                                let mut app = app_arc.write().await;
                                app.loading = true;
                            }

                            {
                                let mut app = app_arc.write().await;
                                app.move_to_spam();
                            }

                            {
                                let mut app = app_arc.write().await;
                                app.loading = false;
                            }
                        }
                        KeyCode::Char('e') => {
                            {
                                let mut app = app_arc.write().await;
                                app.loading = true;
                            }

                            {
                                let mut app = app_arc.write().await;
                                app.archive();
                            }

                            {
                                let mut app = app_arc.write().await;
                                app.loading = false;
                            }
                        }
                        KeyCode::Char('f') => {
                            let backend = backend()?;
                            let folders = backend.list_folders()?;
                            let folders = folders.to_vec();
                            info!("Folders: {folders:#?}");
                        }
                        KeyCode::Char('d') => {
                            let app = app_arc.read().await;
                            app.dump_emails();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let mut app = app_arc.write().await;
                            app.down()
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            let mut app = app_arc.write().await;
                            app.up()
                        }
                        KeyCode::PageDown => {
                            let mut app = app_arc.write().await;
                            app.page_down()
                        }
                        KeyCode::PageUp => {
                            let mut app = app_arc.write().await;
                            app.page_up()
                        }
                        KeyCode::Left | KeyCode::Right => {
                            let mut app = app_arc.write().await;
                            app.focus_next()
                        }
                        KeyCode::Home => {
                            let mut app = app_arc.write().await;
                            app.home()
                        }
                        KeyCode::End => {
                            let mut app = app_arc.write().await;
                            app.end()
                        }
                        KeyCode::Enter => {
                            {
                                let mut app = app_arc.write().await;
                                app.loading = true;
                            }

                            let state = app_arc.clone();
                            tokio::spawn(async move {
                                let mut app = state.write().await;
                                let mut email = app.selected_email();
                                // FIXME handle error
                                email.load().unwrap();

                                app.show_email(email);
                                app.loading = false;
                            });
                        }
                        KeyCode::Esc => {
                            let mut app = app_arc.write().await;
                            app.focus = AppFocus::EmailList;
                            app.open_email = None
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            trace!("Starting tick...");
            // let mut app = app_arc.write().await;
            // app.on_tick(&events);
            // events.clear();
            last_tick = Instant::now();
            trace!("Tick done...");
        }
    }

    Ok(())
}

fn ui<B: TuiBackend>(f: &mut Frame<B>, mut app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.size());
    let body_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    let header = Block::default()
        .borders(Borders::BOTTOM)
        .title(Spans::from(vec![
            Span::styled("Poste.rs", Style::default().fg(Color::Green)),
            Span::raw(" 0.1"),
        ]));

    let width = f.size().width as usize;
    let max_width = width - 2 - 6;
    let email_body_height = body_chunks[0].height as usize - 2;

    app.max_width = max_width;
    app.email_page_size = email_body_height;

    let items = app
        .emails
        .iter()
        .map(|email| {
            let mark = if email.flag == EmailFlag::Spam {
                "◼"
            } else {
                " "
            };

            let from = email.from_name.clone().unwrap_or(email.from_addr.clone());
            let from = from
                .unicode_pad(25, unicode_truncate::Alignment::Left, true)
                .to_string();

            let subject_width = max_width - 25 - 16 - 3 - 2;
            let subject = email
                .subject
                .clone()
                .unicode_pad(subject_width, unicode_truncate::Alignment::Left, true)
                .to_string();

            let date = email.date.format("%d/%m/%Y %I:%M%P").to_string();

            ListItem::new(Spans::from(vec![
                Span::raw(" "),
                Span::styled(mark, Style::default().fg(Color::Red)),
                Span::raw(" "),
                Span::styled(from, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(subject),
                Span::raw(" "),
                Span::raw(date),
            ]))
        })
        .collect::<Vec<_>>();

    let style = Style::default();
    let style = if app.focus == AppFocus::EmailList {
        style.add_modifier(Modifier::BOLD)
    } else {
        style.fg(Color::DarkGray)
    };

    let emails = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Emails")
                .border_style(style),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow))
        .highlight_symbol(">>");

    let mut emails_state = ListState::default();
    emails_state.select(Some(app.selected_email));

    f.render_stateful_widget(emails, body_chunks[0], &mut emails_state);

    match app.open_email {
        Some(ref email) => {
            let body = match app.email_viewport() {
                Some(body) => body,
                None => "No body".to_string(),
            };

            use ansi_to_tui::IntoText;

            let text = body.into_text().unwrap();
            let style = Style::default();
            let style = if app.focus == AppFocus::EmailBody {
                style.add_modifier(Modifier::BOLD)
            } else {
                style.fg(Color::DarkGray)
            };
            let details = Paragraph::new(text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(email.subject.clone())
                        .border_style(style),
                )
                .alignment(Alignment::Left);
            f.render_widget(details, body_chunks[1]);
        }
        None => {
            let details = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Details");
            f.render_widget(details, body_chunks[1]);
        }
    };

    let state = if app.loading { "LOADING" } else { "NORMAL" };
    let progress = if app.loading {
        " ... ".to_string()
    } else if app.focus == AppFocus::EmailList {
        format!(
            " {current}/{len} ",
            current = app.selected_email + 1,
            len = app.emails.len()
        )
    } else {
        format!(
            " {current}/{len} ",
            current = app.email_offset + 1,
            len = app.email_line_count().unwrap() + 1
        )
    };

    let footer = Block::default()
        .borders(Borders::NONE)
        .title(Spans::from(vec![
            Span::styled(
                format!(" {state} "),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                progress,
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

    f.render_widget(header, chunks[0]);
    f.render_widget(footer, chunks[2]);
}
