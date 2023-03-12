use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{future::FutureExt, select, StreamExt};
use futures_timer::Delay;
use tokio::{
    runtime::Handle,
    sync::{mpsc, RwLock},
};
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

use crate::event::EventHandler;
use crate::{
    app::{App, AppFocus},
    email::EmailFlag,
    event::EventType,
};

pub async fn run(tick_rate: Duration) -> anyhow::Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    info!("Starting UI run");

    let app = Arc::new(RwLock::new(App::new()));
    let (tx, mut rx) = mpsc::channel::<EventType>(16);

    let cloned_app = app.clone();
    tokio::spawn(async move {
        info!("Starting handler process");
        let handler = EventHandler::new(cloned_app);
        while let Some(event) = rx.recv().await {
            trace!("Received: {event:#?}");
            handler.execute(event).await;
        }
        info!("Finished handler process");
    });

    tx.send(EventType::StartLoading).await?;
    tx.send(EventType::LoadEmails).await?;
    tx.send(EventType::FinishLoading).await?;

    info!("Starting app process");
    info!("Running app");
    run_app(&mut terminal, app, tick_rate, tx.clone()).await?;

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
    app: Arc<RwLock<App>>,
    tick_rate: Duration,
    channel: mpsc::Sender<EventType>,
) -> anyhow::Result<()> {
    info!("Starting app loop");
    let mut last_tick = Instant::now();
    let mut last_update = None;
    let mut last_size = None;
    loop {
        let mut delay = Delay::new(tick_rate).fuse();
        let mut reader = EventStream::new();

        {
            {
                let app = app.read().await;
                let size = terminal.size().unwrap();

                if last_size != Some(size) {
                    last_update = None;
                    last_size = Some(size);
                }

                if last_update != Some(app.last_update) || last_update.is_none() {
                    trace!("Rendering...");
                    terminal.draw(|f| ui(f, &app, &channel))?;
                    last_update = Some(app.last_update);
                }
            }

            let mut event = reader.next().fuse();

            select! {
                _ = delay => {},
                maybe_event = event => {
                    match maybe_event {
                        Some(Ok(Event::Key(event))) => {
                            if !handle_keypress(event, &app, &channel).await? {
                                break;
                            }
                        },
                        Some(Err(e)) => println!("Error: {:?}\r", e),
                        None => {},
                        _ => {},
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

async fn handle_keypress(
    event: event::KeyEvent,
    app: &Arc<RwLock<App>>,
    channel: &mpsc::Sender<EventType>,
) -> anyhow::Result<bool> {
    trace!("event = {:?}", event);

    let app = app.read().await;

    match event.code {
        KeyCode::Char('q') => return Ok(false),
        KeyCode::Char('e') => {
            channel.send(EventType::StartLoading).await?;
            channel.send(EventType::Archive).await?;
            channel.send(EventType::FinishLoading).await?;
        }
        KeyCode::Char(' ') => {
            channel.send(EventType::ToggleSpam).await?;
            channel.send(EventType::Down).await?;
        }
        KeyCode::Char('s') => {
            channel.send(EventType::StartLoading).await?;
            channel.send(EventType::MoveToSpam).await?;
            channel.send(EventType::FinishLoading).await?;
        }
        KeyCode::Char('r') => {
            channel.send(EventType::StartLoading).await?;
            channel.send(EventType::RefreshEmails).await?;
            channel.send(EventType::FinishLoading).await?;
        }
        KeyCode::Char('d') => {
            app.dump_emails();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            channel.send(EventType::Up).await?;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            channel.send(EventType::Down).await?;
        }
        KeyCode::PageUp => {
            channel.send(EventType::PageUp).await?;
        }
        KeyCode::PageDown => {
            channel.send(EventType::PageDown).await?;
        }
        KeyCode::Left | KeyCode::Right => {
            channel.send(EventType::FocusNext).await?;
        }
        KeyCode::Home => {
            channel.send(EventType::Home).await?;
        }
        KeyCode::End => {
            channel.send(EventType::End).await?;
        }
        KeyCode::Enter => {
            channel.send(EventType::StartLoading).await?;
            channel.send(EventType::OpenEmail).await?;
            channel.send(EventType::FinishLoading).await?;
        }
        KeyCode::Esc => match app.focus {
            AppFocus::EmailList => {
                channel.send(EventType::CloseEmail).await?;
            }
            AppFocus::EmailBody => {
                channel.send(EventType::FocusNext).await?;
            }
        },
        _ => {}
    };

    Ok(true)
}

fn ui<B: TuiBackend>(f: &mut Frame<B>, app: &App, channel: &mpsc::Sender<EventType>) {
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

    if app.max_width != max_width {
        tokio::task::block_in_place(move || {
            Handle::current().block_on(async move {
                channel
                    .send(EventType::SetMaxWidth(max_width))
                    .await
                    .unwrap();
                channel
                    .send(EventType::SetEmailPageSize(email_body_height))
                    .await
                    .unwrap();
            });
        });
    }

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
