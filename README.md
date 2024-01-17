This is a "relative volume at time" equity scanner with a terminal UI.
It uses rust for speed in an attempt to cover the entirety of publicly traded US
stocks as fast as possible.

It requires a JSON cache of the 1 minute bars for the last 21 days for all US
stocks in the `cache` folder.

To load the cache run 

`node build_cache.js`

The program has no dependencies so no npm installation is needed.

To build the rust client run
`cargo run`

The rust client loads alpaca.markets API keys from the runtime environment.

Please set:

```
export APCA_API_KEY_ID='YOURKEYID'
export APCA_API_SECRET_KEY='YOURSECRET'
```

Immediate Work Items:
1. Need the daily change % for each issue in the list
3. Possibly another thread that updates those who are on the LIST more
frequently than the primary worker thread.
4. Make build_cache.js delete cache entries older than 25 days.
5. Commafy


Can we make this faster by loading the latest daily bar instead of summing the
1min bars?
