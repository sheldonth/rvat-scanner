This is a "relative volume at time" heat map written in terminal UI.
It uses rust for speed.

When the program starts, we load a JSON cache which contains minute bars for the
last X days for all the symbols.

Rust Challenges:

Program Specification

When the program starts, download the latest 21 trading days from the calendar
API.

Iterate those to load from the cache.
1. Need the daily change % for each issue in the list
2. need a % progress indicator for where we are in the current pass.
3. Possibly another thread that updates those who are on the LIST more
frequently than the primary worker thread.
4. Make build_cache.js delete cache entries older than 25 days.


