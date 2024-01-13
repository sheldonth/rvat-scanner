const got = require('./got.js')
const qs = require('querystring')
const assert = require('assert')
const data_host = 'https://data.alpaca.markets'
const api_host = 'https://api.alpaca.markets'
assert(process.env['APCA_API_KEY_ID'], "Must set APCA_API_KEY_ID")
assert(process.env['APCA_API_SECRET_KEY'], "Must set APCA_API_SECRET_KEY")
const auth_headers = {
    'APCA-API-KEY-ID':process.env['APCA_API_KEY_ID'],
    'APCA-API-SECRET-KEY':process.env['APCA_API_SECRET_KEY']
}
const tz = (date) => {
    const timezoneOffsetInMinutes = date.getTimezoneOffset();
    const timezoneOffsetHours = Math.abs(Math.floor(timezoneOffsetInMinutes / 60));
    const timezoneOffsetMinutes = Math.abs(timezoneOffsetInMinutes % 60);
    const timezoneSign = timezoneOffsetInMinutes < 0 ? '+' : '-';
    const timezone = `${timezoneSign}${timezoneOffsetHours.toString().padStart(2, '0')}:${timezoneOffsetMinutes.toString().padStart(2, '0')}`;
    return timezone
}

function ISODateString(d){
    function pad(n){return n<10 ? '0'+n : n}
    return d.getUTCFullYear()+'-'
      + pad(d.getUTCMonth()+1)+'-'
      + pad(d.getUTCDate())+'T'
      + pad(d.getUTCHours())+':'
      + pad(d.getUTCMinutes())+':'
      + pad(d.getUTCSeconds())
      + tz(d)
}

const get_account_history = async function(period, timeframe, date_end) {
    const request = await got(api_host+`/v2/account/portfolio/history?` +
        qs.encode({period, timeframe, date_end}), {
        headers:auth_headers
    })
    assert(request.statusCode == 200, `Failed to load account history ${JSON.stringify(request)}`)
    return JSON.parse(request.body)
}

const get_latest_quote = async function(symbol) {
    const request = await got(data_host+`/v2/stocks/${symbol}/quotes/latest`, {
        headers:auth_headers
    })
    assert([200].includes(request.statusCode), `Failed to load latest quote ${JSON.stringify(request)}`)
    return JSON.parse(request.body)
}


const get_latest_multitrade = async function(symbols) {
    assert(Array.isArray(symbols), "Must pass array to multitrade")

    const params = {
        'symbols':symbols.join(',')
    }
    const request = await got(data_host+`/v2/stocks/trades/latest?${qs.encode(params)}`, {
        headers:auth_headers
    })
    assert(request.statusCode == 200, `Failed to get latest trade ${JSON.stringify(request)}`)
    return JSON.parse(request.body)
}

const get_latest_trade = async function(symbol) {
    const request = await got(data_host+`/v2/stocks/${symbol}/trades/latest`, {
        headers:auth_headers
    })
    assert(request.statusCode == 200, `Failed to get latest trade ${JSON.stringify(request)}`)
    return JSON.parse(request.body)
}

const get_snapshot = async function(symbol) {
    const request = await got(data_host+`/v2/stocks/${symbol}/snapshot`, {
        headers:auth_headers
    })
    const response = JSON.parse(request.body)

    const daily_is_today = moment().isSame(response.dailyBar.t, 'day')

    if (daily_is_today) {
        return response.prevDailyBar
    }
    else {
        return response.dailyBar
    }
} 

const get_asset = async function(asset_id) {
    const request = await got(api_host+`/v2/assets/${asset_id}`, {
        headers:auth_headers
    })
    if (request.statusCode == 429) {
        throw new Error('Rate Limit Exceeded')
    }
    return JSON.parse(request.body)
}

const big_board = [
    'ARCA',
    'NASDAQ',
    'NYSE',
    'BATS'
]

const get_equity_assets = async function() {
    const request = await got(api_host + '/v2/assets', {
        headers:auth_headers
    })
    const assets = JSON.parse(request.body)
        .filter((a) => {return a.tradable == true})
        .filter((a) => {return a.status == 'active'})
        .filter((a) => {return big_board.includes(a.exchange)})
    return assets
}

const get_bars = async (symbol, start, end, timeframe='1Min') => {
    const parameters = {
        timeframe: timeframe,
        start: ISODateString(start),
        end: ISODateString(end),
        limit: 10000,
        adjustment: 'all'
    }
    const url = `${data_host}/v2/stocks/${symbol}/bars` + 
        `?${new URLSearchParams(parameters)}`
    const response = await got(url, {
        headers: auth_headers
    })
    const data = JSON.parse(response.body)
    let bars = data.bars
    while(data.next_page_token) {
        const url = `${data_host}/v2/stocks/${symbol}/bars` + 
            `?${new URLSearchParams(parameters)}&page_token=${data.next_page_token}`
        const response = await got(url, {
            headers: auth_headers
        })
        const data = JSON.parse(response.body)
        bars = bars.concat(data.bars)
    }
    return bars
}


const get_calendar = async function(start, end) {
    const params = qs.encode({
        start:ISODateString(start),
        end:ISODateString(end)
    })
    const request = await got(api_host+`/v2/calendar?${params}`, {
        headers:auth_headers
    })
    return JSON.parse(request.body)
}

const get_clock = async function() {
    const request = await got(api_host+"/v2/clock", {
        headers:auth_headers
    })
    return JSON.parse(request.body)
}

module.exports = {
    get_account_history,
    get_latest_quote,
    get_latest_multitrade,
    get_latest_trade,
    get_snapshot,
    get_asset,
    get_equity_assets,
    get_calendar,
    get_clock,
    get_bars
}

