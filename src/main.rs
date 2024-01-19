use std::io;
use std::fs::{self, DirEntry, ReadDir};
use std::path::{Path};
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
use chrono::Timelike;
use chrono::{DateTime, Local, NaiveTime, TimeZone, FixedOffset};
use chrono_tz::America::New_York;
use chrono::{Utc, Offset};
use rvat_scanner::alpaca::Bar;
use rvat_scanner::alpaca;
use std::collections::HashSet;

static LIST_ITEM_HEIGHT:u16 = 50;
static THREADS:usize = 5;

use serde::Deserialize;
#[derive(Deserialize, Debug, Clone)]
pub struct Ticker {
    ticker:String
}

use lazy_static::lazy_static;
lazy_static! {
    pub static ref SYMBOLS:Vec<String> = read_cache_folders(Path::new("cache")).unwrap();
    pub static ref EXCLUDED_SYMBOLS:Vec<Ticker> = serde_json::from_str(
        fs::read_to_string("excluded_tickers.json").unwrap().as_str()
    ).unwrap();
}

fn read_cache_folders(folder_path:&Path) -> io::Result<Vec<String>> {
    let mut symbols:Vec<String> = Vec::new();
    // read the folders in the '../cache' directory
    let entries:ReadDir = fs::read_dir(folder_path)?;
    for entry in entries {
        let entry:DirEntry = entry?;
        // each folder name is a symbol
        let folder_name:String = entry.file_name().into_string().unwrap();
        // ignore .DS_Store files
        if folder_name == ".DS_Store" {
            continue;
        }
        symbols.push(folder_name);
    }
    assert!(symbols.len() > 0, "no symbols found in cache, run node build_cache.js first");
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

struct Analysis {
    symbol:String,
    average_dvat:u64,
    analysis_dvat:u64,
    score:f64,
    pnl_change_percent:f64,
    created_at:DateTime<FixedOffset>
}

struct App { 
    items: StatefulList<Analysis>,
    title: String
}


impl App {
    fn new() -> App {
        App {
            items: StatefulList::with_items(vec![ ]),
            title: String::from("RVAT Scanner")
        }
    }

    fn add_analysis(&mut self, mut item:Analysis) {
        // find the entry in items for the ticker
        let mut index:usize = 0;
        let mut found:bool = false;
        for i in &self.items.items {
            if i.symbol == item.symbol {
                found = true;
                break;
            }
            index += 1;
        }
        if found {
            // update the entry, but preserve the created_at time
            item.created_at = self.items.items[index].created_at;
            self.items.items[index] = item;

        } else {
            // check if item is bigger than at least 1 item in the list
            let mut index:usize = 0;
            let mut found:bool = false;
            for i in &self.items.items {
                if item.score > i.score {
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
            self.items.items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
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
//fn time_in_new_york (hour_minute:&str) -> DateTime<FixedOffset> {
    //let naive_time = NaiveTime::parse_from_str(hour_minute, "%H:%M").unwrap();
    ////let local_date = Local::today().naive_local();
    //let local_date = Local::now().naive_local().date();
    //let local_datetime = local_date.and_time(naive_time);
    //// TODO: need a more robust solution for handling DST.
    ////let ny_offset = FixedOffset::west(5 * 3600); // UTC-5 hours for Eastern Standard Time
    //let ny_offset = FixedOffset::west_opt(5 * 3600).unwrap(); // UTC-5 hours for Eastern Daylight Time
    //let ny_datetime = ny_offset.from_local_datetime(&local_datetime).unwrap();
    //ny_datetime
//}

fn time_in_new_york(hour_minute: &str) -> DateTime<FixedOffset> {
    let naive_time = NaiveTime::parse_from_str(hour_minute, "%H:%M").unwrap();
    let local_date = Local::now().naive_local().date();
    let local_datetime = local_date.and_time(naive_time);

    // Find the equivalent UTC datetime
    let utc_datetime = Utc.from_local_datetime(&local_datetime).unwrap();
    let naive_utc_datetime = utc_datetime.naive_utc();
    // Convert the UTC datetime to New York time, considering DST
    let ny_datetime_with_tz = New_York.from_utc_datetime(&naive_utc_datetime);

    // Extract the fixed offset
    let fixed_offset = ny_datetime_with_tz.offset().fix();

    // Apply the fixed offset to the original naive datetime
    fixed_offset.from_local_datetime(&local_datetime).unwrap()
}


fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    /*mut */app: Arc<Mutex<App>>,
    tick_rate: Duration,
) -> io::Result<()> {
    let symbol_index:usize = 0;
    let symbol_index_ptr = Arc::new(Mutex::new(symbol_index));
    let loops:usize = 0;
    let loops_ptr = Arc::new(Mutex::new(loops));
    let excluded_symbols:HashSet<&str> = HashSet::<&str>::from_iter(EXCLUDED_SYMBOLS.iter().map(|t| t.ticker.as_str()));
    let excluded_symbols_ptr = Arc::new(Mutex::new(excluded_symbols));
    fn next_symbol(symbol_index_ptr:Arc<Mutex<usize>>, loops_ptr: Arc<Mutex<usize>>) -> (usize, String) {
        let mut symbol_index = symbol_index_ptr.lock().unwrap();
        *symbol_index += 1;
        if *symbol_index >= SYMBOLS.len() {
            *symbol_index = 0;
            let mut loops = loops_ptr.lock().unwrap();
            *loops += 1;
        }
        (*symbol_index, SYMBOLS[*symbol_index].clone())
    }
    for _ in 0..THREADS {
        let app_clone = app.clone();
        let symbol_index_ptr = symbol_index_ptr.clone();
        let loops_ptr = loops_ptr.clone();
        let excluded_symbols_ptr = excluded_symbols_ptr.clone();
        thread::spawn(move || {
            let now = chrono::DateTime::from(chrono::Utc::now());
            let start = now - chrono::Duration::days(60);
            let trading_days = alpaca::get_calendar(
                start, now);
            let analysis_day = trading_days[0].clone();
            let reference_days = trading_days[1..18].to_vec();
            loop {
                let (symbol_index, symbol) = next_symbol(symbol_index_ptr.clone(), loops_ptr.clone());
                if excluded_symbols_ptr.lock().unwrap().contains(symbol.as_str()) {
                    continue;
                }
                let progress = (symbol_index as f64 / SYMBOLS.len() as f64) * 100.0;
                let progress = (progress * 10.0).round() / 10.0;
                let progress_string = format!("{}%", progress);
                let title = format!("RVAT Scanner {} {} ({})", &analysis_day.date.as_str(), 
                                    progress_string, loops_ptr.lock().unwrap());
                app_clone.lock().unwrap().set_title(title.as_str());
                let mut volumes:Vec<u64> = Vec::new();
                for reference_day in &reference_days {
                    let key = format!("{}.json", reference_day.date);
                    let bar_data_path = format!("cache/{}/{}", symbol, key);
                    let bar_data = match fs::read_to_string(bar_data_path.clone()) {
                        Ok(bar_data) => bar_data,
                        Err(_) => {
                            continue;
                        }
                    };
                    let bars:Vec<Bar> = match serde_json::from_str(&bar_data) {
                        Ok(bars) => bars,
                        Err(_) => {
                            continue;
                        }
                    };
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
                }

                let average_dvat:f64 = volumes.iter().sum::<u64>() as f64 / volumes.len() as f64;
                let mut session_open_new_york_time:String = analysis_day.session_open.clone();
                session_open_new_york_time.insert(2, ':');
                let mut session_close_new_york_time = analysis_day.session_close.clone();
                session_close_new_york_time.insert(2, ':');
                let analysis_day_bars = alpaca::get_bars( symbol.as_str(),
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
                // find the % change from the 0th bar to the last bar
                let mut pnl_change_percent:f64 = 0.0;
                if analysis_day_bars.get_bars().len() == 0 {
                    continue;
                }
                let first_bar = &analysis_day_bars.get_bars()[0].c;
                let last_bar =  &analysis_day_bars.get_bars()[analysis_day_bars.get_bars().len() - 1].c;
                match first_bar.as_f64() {
                    Some(first_bar) => {
                        match last_bar.as_f64() {
                            Some(last_bar) => {
                                pnl_change_percent = (first_bar - last_bar) / first_bar;
                            },
                            None => { }
                        }
                    },
                    None => { }
                }
                /*
                 * where do you cut off average_dvat?
                 * this value is the average of the last 17 days
                 * if it's absurdly low and the stock is highly illiquid,
                 * we get a false positive high score.
                 * a score of 35513855 / 16164 = 2195.5 is absurdly high and 
                 * what we are looking for.
                 *
                 * 61000 / 20 = 3005 is a better score but it's because the 
                 * divisor is so low
                 *
                 * let's start with 350
                 * now trying 1000
                 */
                if average_dvat < 1000 as f64 {
                    continue;
                }
                if analysis_dvat == 0 as u64 {
                    continue;
                }
                app_clone.lock().unwrap().add_analysis(Analysis {
                    symbol:String::from(symbol),
                    average_dvat:average_dvat as u64,
                    analysis_dvat:analysis_dvat as u64,
                    score:analysis_dvat as f64 / average_dvat as f64,
                    pnl_change_percent,
                    created_at:chrono::Utc::now().into()
                });
            }
        });
    }

    let mut last_tick = Instant::now();
    loop {

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

fn duration_to_human_readable(dur:chrono::Duration) -> String {
    let hours = dur.num_hours();
    if hours >= 1 {
        return format!("{}h", hours);
    }
    else {
        return format!("{}m", dur.num_minutes());
    }
}

fn count_to_human_readable(arg:u64) -> String {
    if arg > 1000000000 {
        return format!("{:.2}B", arg as f64 / 1000000000.0);
    }
    if arg > 1000000 {
        return format!("{:.2}M", arg as f64 / 1000000.0);
    }
    if arg > 1000 {
        return format!("{:.2}K", arg as f64 / 1000.0);
    }
    return format!("{}", arg);
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Create a chunk with 100% horizontal screen space
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    let items: Vec<ListItem> = app
        .items
        .items
        .iter()
        .map(|i| {
            let pnl_style = if i.pnl_change_percent >= 0.0 {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };

            let age = chrono::Utc::now().signed_duration_since(i.created_at);
            let age_string = duration_to_human_readable(age);

            let pnl_change_percent = Span::styled(
                format!("{:>10.2}%", i.pnl_change_percent * 100.0),
                pnl_style,
            );

            let line_text = Spans::from(vec![
                Span::raw(format!("{:<10} {:>8} {:>10} {:>8.2} {:>4}", 
                                  i.symbol, 
                                  count_to_human_readable(i.analysis_dvat), 
                                  count_to_human_readable(i.average_dvat), 
                                  i.score, age_string)),
                pnl_change_percent,
            ]);
            ListItem::new(line_text).style(Style::default().fg(Color::White))
        })
        .collect();

    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(app.title.as_str()))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(items, chunks[0], &mut app.items.state);
}


