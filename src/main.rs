use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};

use app::{App, AppEvent};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dotenvy::dotenv;
use tokio::{sync::RwLock, task};
use tracing::{error, info, trace};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tui::{
    backend::{Backend as TuiBackend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use unicode_truncate::UnicodeTruncateStr;

use crate::email::get_emails;

mod app;
mod auth;
mod email;

struct OAuth2 {
    user: String,
    access_token: String,
}

impl imap::Authenticator for OAuth2 {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "posters.log");
    tracing_subscriber::fmt().with_writer(file_appender).init();

    task::block_in_place(move || {
        let res = auth::auth();
        if let Err(err) = res {
            error!("Auth error: {err}");
        }
    });

    let tick_rate = Duration::from_millis(80);
    run(tick_rate).await?;

    Ok(())
}

async fn run(tick_rate: Duration) -> anyhow::Result<()> {
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

async fn run_app<B: TuiBackend>(
    terminal: &mut Terminal<B>,
    app_arc: &Arc<RwLock<App>>,
    tick_rate: Duration,
) -> anyhow::Result<()> {
    let mut last_tick = Instant::now();
    let mut events = vec![];

    loop {
        {
            let app = app_arc.read().await;
            terminal.draw(|f| ui(f, &app))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if crossterm::event::poll(timeout)? {
                if let Event::Key(event) = event::read()? {
                    trace!("event = {:?}", event);
                    match event.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Down
                        | KeyCode::Up
                        | KeyCode::PageDown
                        | KeyCode::PageUp
                        | KeyCode::Home
                        | KeyCode::End
                        | KeyCode::Enter => events.push(AppEvent::Key(event)),
                        _ => (),
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            trace!("Starting tick...");
            if !events.is_empty() {
                trace!("Sending events: {events:#?}");
            }
            let mut app = app_arc.write().await;
            app.on_tick(&events);
            events.clear();
            last_tick = Instant::now();
            trace!("Tick done...");
        }
    }

    Ok(())
}

fn ui<B: TuiBackend>(f: &mut Frame<B>, app: &App) {
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

    let header = Block::default()
        .borders(Borders::BOTTOM)
        .title(Spans::from(vec![
            Span::styled("Poste.rs", Style::default().fg(Color::Green)),
            Span::raw(" 0.1"),
        ]));

    let width = f.size().width as usize;
    let max_width = width - 2 - 6;
    let items = app
        .emails
        .iter()
        .map(|email| {
            let from = email.from_name.clone().unwrap_or(email.from_addr.clone());
            let from = from
                .unicode_pad(25, unicode_truncate::Alignment::Left, true)
                .to_string();

            let subject_width = max_width - 25 - 16 - 3;
            let subject = email
                .subject
                .clone()
                .unicode_pad(subject_width, unicode_truncate::Alignment::Left, true)
                .to_string();

            let date = email.date.format("%d/%m/%Y %I:%M%P").to_string();

            ListItem::new(Spans::from(vec![
                Span::raw(" "),
                Span::styled(from, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(subject),
                Span::raw(" "),
                Span::raw(date),
            ]))
        })
        .collect::<Vec<_>>();

    let emails = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Emails"))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow))
        .highlight_symbol(">>");
    let body_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    let mut emails_state = ListState::default();
    emails_state.select(Some(app.selected_email));

    f.render_stateful_widget(emails, body_chunks[0], &mut emails_state);

    match app.open_email {
        Some(ref email) => {
            let body = match email.clone().body {
                Some(body) => {
                    let res = html2text::from_read(body.as_bytes(), max_width);
                    res
                }
                None => "No body".to_string(),
            };

            use ansi_to_tui::IntoText;

            let text = body.into_text().unwrap();
            info!("text = {:#?}", text);
            let details = Paragraph::new(text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(email.subject.clone()),
                )
                .alignment(Alignment::Left);
            f.render_widget(details, body_chunks[1]);
        }
        None => {
            let details = Block::default().borders(Borders::ALL).title("Details");
            f.render_widget(details, body_chunks[1]);
        }
    };

    let footer = Block::default()
        .borders(Borders::NONE)
        .title(Spans::from(vec![
            Span::styled(
                " NORMAL ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    " {current}/{len} ",
                    current = app.selected_email + 1,
                    len = app.emails.len()
                ),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

    f.render_widget(header, chunks[0]);
    f.render_widget(footer, chunks[2]);
}
