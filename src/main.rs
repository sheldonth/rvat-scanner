use std::io;
use std::fs::{self, DirEntry, ReadDir};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::Deserialize;
use chrono::FixedOffset;
use chrono::DateTime;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    time::{Duration, Instant},
};
use std::thread;
use std::sync::{Arc, Mutex};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame, Terminal,
};
#[derive(Deserialize, Debug, Clone)]
pub struct Bar {
    pub t:DateTime<FixedOffset>,
    pub o:serde_json::Value,
    pub h:serde_json::Value,
    pub l:serde_json::Value,
    pub c:serde_json::Value,
    pub v:serde_json::Value
}

type BarsForDate = HashMap<String, Vec<Bar>>;
type SymbolBars = HashMap<String, Vec<BarsForDate>>;

fn read_cache_folders(folder_path:&Path) -> io::Result<SymbolBars> {
    let mut symbols:SymbolBars = HashMap::new();
    // read the folders in the '../cache' directory
    let entries:ReadDir = fs::read_dir(folder_path)?;
    for entry in entries {
        let entry:DirEntry = entry?;
        // each folder name is a symbol
        let folder_name:String = entry.file_name().into_string().unwrap();
        // read all the files in in the entry folder
        let files:ReadDir = fs::read_dir(entry.path())?;
        let mut v:Vec<BarsForDate> = Vec::new();
        // iterate files
        for file in files {
            let file:DirEntry = file?;
            // get file name
            let file_name:String = file.file_name().into_string().unwrap();
            // get file path
            let file_path_buf:PathBuf = file.path();
            let file_path:&Path = file_path_buf.as_path();
            // read file
            let file_contents:String = fs::read_to_string(file_path).unwrap();
            // deserialize file contents
            let bars:Vec<Bar> = serde_json::from_str(&file_contents).unwrap();
            // print file contents
            let mut bars_for_date:BarsForDate = HashMap::new();
            bars_for_date.insert(file_name, bars);
            v.push(bars_for_date);
        }
        symbols.insert(folder_name, v);
    }
    Ok(symbols)
}

fn load_data_from_cache() -> Result<SymbolBars, Box<dyn Error + Send + Sync>> {
    let cache_path = Path::new("cache");
    match read_cache_folders(cache_path) {
        Ok(symbols) => {
            Ok(symbols)
        },
        Err(e) => {
            Err(Box::new(e))
        }
    }
}

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }
    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    fn unselect(&mut self) {
        self.state.select(None);
    }
}

struct App<'a> {
    items: StatefulList<(&'a str, usize)>,
    data_cache: SymbolBars
}

impl<'a> App<'a> {
    fn new() -> App<'a> {
        let data_cache = load_data_from_cache();
        App {
            items: StatefulList::with_items(vec![
                ("Item0", 1),
                ("Item1", 4)
            ]),
            data_cache: data_cache.unwrap()
        }
    }

    fn on_tick(&mut self) {

    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);

    let worker_handle = thread::spawn(move || {
        // iterate each symbol in app.data_cache and calculate the average volume by this time of
        // day
    });
    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Left => app.items.unselect(),
                    KeyCode::Down => app.items.next(),
                    KeyCode::Up => app.items.previous(),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .items
        .items
        .iter()
        .map(|i| {
            let mut lines = vec![Spans::from(i.0)];
            for _ in 0..i.1 {
                lines.push(Spans::from(Span::styled(
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
                    Style::default().add_modifier(Modifier::ITALIC),
                )));
            }
            ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::Black))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // We can now render the item list
    f.render_stateful_widget(items, chunks[0], &mut app.items.state);
}


