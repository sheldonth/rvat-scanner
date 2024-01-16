use std::io;
use std::fs::{self, DirEntry, ReadDir};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
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
use chrono::{DateTime, Timelike, FixedOffset};
use chrono::{NaiveTime, Local, TimeZone};
use rvat_scanner::alpaca::Bar;

use rvat_scanner::alpaca;

type DayBars = HashMap<String, Vec<Bar>>;
type SymbolDays = HashMap<String, Vec<DayBars>>;

static LIST_ITEM_HEIGHT:u16 = 20;

use lazy_static::lazy_static;
lazy_static! {
    pub static ref SYMBOL_DAYS:SymbolDays = read_cache_folders(Path::new("cache")).unwrap();
}

fn read_cache_folders(folder_path:&Path) -> io::Result<SymbolDays> {
    let mut symbols:SymbolDays = HashMap::new();
    // read the folders in the '../cache' directory
    let entries:ReadDir = fs::read_dir(folder_path)?;
    for entry in entries {
        let entry:DirEntry = entry?;
        // each folder name is a symbol
        let folder_name:String = entry.file_name().into_string().unwrap();
        // read all the files in in the entry folder
        // ignore .DS_Store files
        if folder_name == ".DS_Store" {
            continue;
        }
        let files:ReadDir = fs::read_dir(entry.path())?;
        let mut v:Vec<DayBars> = Vec::new();
        // iterate files
        let mut bars_for_date:DayBars = HashMap::new();
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
            let bars:Vec<Bar> = match serde_json::from_str(&file_contents) {
                Ok(bars) => bars,
                Err(e) => {
                    println!("error deserializing file: {} {} {}", e, file_name, file_path.display());
                    continue;
                }
            };
            // print file contents
            bars_for_date.insert(file_name, bars);
        }
        v.push(bars_for_date);
        assert!(v.len() > 0, "no files found in folder: {}", folder_name);
        symbols.insert(folder_name, v);
    }
    assert!(symbols.len() > 0, "no symbols found in cache");
    Ok(symbols)
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

struct App { // Ticker, Average, TodayAnalysis, TodayScore
    items: StatefulList<(String, i64, i64, f64)>,
    title: String
}


impl App {
    fn new() -> App {
        App {
            items: StatefulList::with_items(vec![ ]),
            title: String::from("RVAT Scanner")
        }
    }

    fn add_item(&mut self, item:(String, i64, i64, f64)) {
        // find the entry in items for the ticker
        let mut index:usize = 0;
        let mut found:bool = false;
        for i in &self.items.items {
            if i.0 == item.0 {
                found = true;
                break;
            }
            index += 1;
        }
        if found {
            // update the entry
            self.items.items[index] = item;
        } else {
            // check if item is bigger than at least 1 item in the list
            let mut index:usize = 0;
            let mut found:bool = false;
            for i in &self.items.items {
                if item.3 > i.3 {
                    found = true;
                    break;
                }
                index += 1;
            }
            if found {
                self.items.items.insert(index, item);
            } else {
                self.items.items.push(item);
            }
            // trim to LIST_ITEM_HEIGHT and sort by score
            if self.items.items.len() > LIST_ITEM_HEIGHT as usize {
                self.items.items.truncate(LIST_ITEM_HEIGHT as usize);
            }
            self.items.items.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap());
        }
    }

    fn set_title(&mut self, title:&str) {
        self.title = String::from(title);
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
    //let app = App::new();
    let app = Arc::new(Mutex::new(App::new()));
    let res = run_app(&mut terminal, app.clone(), tick_rate);

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

// hour_minute is like "04:00"
// reurn DateTime<FixedOffset> in New York
fn time_in_new_york (hour_minute:&str) -> DateTime<FixedOffset> {
    let naive_time = NaiveTime::parse_from_str(hour_minute, "%H:%M").unwrap();
    //let local_date = Local::today().naive_local();
    let local_date = Local::now().naive_local().date();
    let local_datetime = local_date.and_time(naive_time);
    // TODO: need a more robust solution for handling DST.
    //let ny_offset = FixedOffset::west(5 * 3600); // UTC-5 hours for Eastern Standard Time
    let ny_offset = FixedOffset::west_opt(5 * 3600).unwrap(); // UTC-5 hours for Eastern Daylight Time
    let ny_datetime = ny_offset.from_local_datetime(&local_datetime).unwrap();
    ny_datetime
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    /*mut */app: Arc<Mutex<App>>,
    tick_rate: Duration,
) -> io::Result<()> {
    let app_clone = app.clone();
    thread::spawn(move || {
        let symbols:Vec<&str> = SYMBOL_DAYS.keys().map(|s| s.as_str()).collect();
        let now = chrono::DateTime::from(chrono::Utc::now());
        let start = now - chrono::Duration::days(60);
        let trading_days = alpaca::get_calendar(
            start, now);
        let analysis_day = trading_days[0].clone();
        app_clone.lock().unwrap().set_title(format!("RVAT Scanner {}", &analysis_day.date).as_str());
        let reference_days = trading_days[1..21].to_vec();
        let mut symbol_index:usize = 0;
        while symbol_index < symbols.len() {
            let symbol:&str = symbols[symbol_index];
            let days:&Vec<DayBars> = SYMBOL_DAYS.get(symbol).unwrap();
            let mut volumes:Vec<u64> = Vec::new();
            for reference_day in &reference_days {
                let key = format!("{}.json", reference_day.date);
                match days[0].get(&key) {
                    Some(bars) => {
                        let utc_hour = chrono::Utc::now().hour();
                        let utc_minute = chrono::Utc::now().minute();
                        let mut volume:u64 = 0;
                        for bar in bars {
                            let bar_hour = bar.t.hour();
                            let bar_minute = bar.t.minute();
                            if bar_hour < utc_hour {
                                match bar.v.as_u64() {
                                    Some(v) => {
                                        volume += v as u64;
                                    },
                                    None => {
                                        println!("volume is not an u64");
                                    }
                                }
                            }
                            if bar_hour == utc_hour && bar_minute <= utc_minute {
                                match bar.v.as_u64() {
                                    Some(v) => {
                                        volume += v as u64;
                                    },
                                    None => {
                                        println!("volume is not an u64");
                                    }
                                }
                            }
                        }
                        volumes.push(volume);
                    },
                    None => {
                        println!("missing bars for {} on {}", symbol, reference_day.date);
                    }
                }
            }
            let average_dvat:f64 = volumes.iter().sum::<u64>() as f64 / volumes.len() as f64;
            let mut session_open_new_york_time:String = analysis_day.session_open.clone();
            session_open_new_york_time.insert(2, ':');
            let mut session_close_new_york_time = analysis_day.session_close.clone();
            session_close_new_york_time.insert(2, ':');
            let analysis_day_bars = alpaca::get_bars(   symbol,
                                                        "1Min",
                                                        time_in_new_york(session_open_new_york_time.as_str()),
                                                        time_in_new_york(session_close_new_york_time.as_str()),
                                                        "1000");

            let mut analysis_dvat:u64 = 0;
            for bar in analysis_day_bars.get_bars() {
                match bar.v.as_u64() {
                    Some(v) => {
                        analysis_dvat += v as u64;
                    },
                    None => {
                        println!("volume is not an u64");
                    }
                }
            }
            //println!("{}: average_dvat: {}, analysis_dvat: {}", symbol, average_dvat, analysis_dvat);
            if average_dvat == 0 as f64 {
                symbol_index += 1;
                continue;
            }
            app_clone.lock().unwrap().add_item((String::from(symbol), 
                                                average_dvat as i64, 
                                                analysis_dvat as i64, 
                                                analysis_dvat as f64 / average_dvat as f64));
            symbol_index += 1;
        }
    });

    let mut last_tick = Instant::now();
    loop {

        //terminal.draw(|f| ui(f, &mut app))?;
        terminal.draw(|f| {
            let mut app = app.lock().unwrap();
            ui(f, &mut app)
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Left => app.lock().unwrap().items.unselect(),
                    KeyCode::Down => app.lock().unwrap().items.next(),
                    KeyCode::Up => app.lock().unwrap().items.previous(),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.lock().unwrap().on_tick();
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
            let line_text = format!("{}: {} / {} = {:.2}", i.0, i.2, i.1, i.3);
            let lines = vec![Spans::from(Span::raw(line_text))];
            //for _ in 0..i.1 {
                //lines.push(Spans::from(Span::styled(
                    //"Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
                    //Style::default().add_modifier(Modifier::ITALIC),
                //)));
            //}
            ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::Black))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(app.title.as_str()))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // We can now render the item list
    f.render_stateful_widget(items, chunks[0], &mut app.items.state);
}


