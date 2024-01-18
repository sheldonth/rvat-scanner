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
2. Make build_cache.js delete cache entries older than X days.
3. Commafy big numbers

Can we make this faster by loading the latest daily bar instead of summing the
1min bars? No. Daily bar prints at 9:30am. The only way to see premarket volume
is asking for the 1minute bars since 4:00am in America/New_York tz and summing
the volume in each 1min bar.

Current market cycle time is 31m6s

