const path = require('path')
const { get_equity_assets,
        get_calendar,
        get_bars }= require('./alpaca.js')
const fs = require('fs').promises

const LOOKBACK_DAYS = 50
const TRADING_PERIODS = 21

const trading_days = async () => {
    const days_past = new Date()
    days_past.setDate(days_past.getDate() - LOOKBACK_DAYS)
    const calendar = (await get_calendar(days_past, new Date())).reverse();
    if (calendar[0].date === new Date().toISOString().slice(0, 10)) {
        return calendar.splice(1, TRADING_PERIODS + 1)
    }
    return calendar.splice(0, TRADING_PERIODS)
}

const check_for_folder = async (folder) => {
    const cache_folder = path.join(__dirname, 'cache', folder)
    try {
        await fs.access(cache_folder)
    } catch (e) {
        await fs.mkdir(cache_folder)
    }
}

const test_stocks = [
    'AAPL',
    'MSFT',
]

const main = async () => {
    let assets = await get_equity_assets()
    //assets = assets.filter(a => {return test_stocks.includes(a.symbol)})
    const days = await trading_days()
    try {
        await fs.access('cache')
    } catch (e) {
        await fs.mkdir('cache')
    }
    for (const asset of assets) {
        await check_for_folder(asset.symbol)
        const cache_folder = path.join(__dirname, 'cache', asset.symbol)
        for (const day of days) {
            const filename = path.join(cache_folder, `${day.date}.json`)
            try {
                await fs.access(filename)
            } catch (e) {
                const start_datetime = new Date(day.date)
                start_datetime.setUTCHours(day.session_open.slice(0, 2))
                start_datetime.setUTCMinutes(day.session_open.slice(2, 4))
                start_datetime.setUTCSeconds(0)

                const end_datetime = new Date(day.date)
                end_datetime.setUTCHours(day.session_close.slice(0, 2))
                end_datetime.setUTCMinutes(day.session_close.slice(2, 4))
                end_datetime.setUTCSeconds(0)

                let bars = await get_bars(asset.symbol, 
                    start_datetime, end_datetime, '1Min')
                bars = bars ? bars : []
                await fs.writeFile(filename, JSON.stringify(bars))
                console.log("Wrote ", filename)
            }
        }
    }
}

if (require.main === module) {
    main()
}
