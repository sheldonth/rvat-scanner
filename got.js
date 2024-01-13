/*
 * This is a minimal rewrite of a promise-based http request
 * The essence of the api is
 * require('./ate-got.js')
 * await got(url, {opts}) returns a response object
 * response.body is response body.
 * response.statusCode is the status code
 * 
 * some opts:
 * cookieJar: a tough-cookie object to be used for the request
 * method:http method
 * form: object which is urlform encoded body
 * timeout: http timeout in milliseconds
 * headers: key-value http raw headers
 */

const https = require('https')
const http = require('http')
const assert = require('assert')
const querystring = require('querystring')

const httpsKAAgent = new https.Agent({
    keepAlive:true,
    keepAliveMsecs:60000,
    //maxSockets:2,
    //maxTotalSockets:50000,
    scheduling:'fifo',
    timeout:120000 // defines global timeout
})

const httpKAAgent = new http.Agent({
    keepAlive:true,
    keepAliveMsecs:60000,
    //maxSockets:2,
    //maxTotalSockets:50000,
    scheduling:'fifo',
    timeout:120000 // defines global timeout
})

const global_user_agent = ''
const global_default_timeout = 120000

const wait = async function(num_seconds) {
    return new Promise((resolve, _) => {
        setTimeout(resolve, num_seconds * 1000)
    })
}

module.exports = async function(url, opts = {headers: {}}, retryCount = 0) {
    try {
        const result = await inner(url, opts)
        return result
    }
    catch (e) {
        console.log(e)
        const value = JSON.parse(e.value)
        if (['ECONNRESET'].includes(value.code) && retryCount < 3) {
            retryCount++
            await wait(retryCount)
            return await module.exports(url, opts, retryCount)
        }
        else {
            console.log(`ate-got.js Unhandled Error`)
            console.log(e)
            throw e
        }
    }
}

const zlib = require('zlib')

const inner = async function(url, opts) {
    return new Promise(function(accept, reject) {
        const encoding = opts.hasOwnProperty('encoding') ? opts.encoding : 'utf8'
        const headers = {
            ...opts.headers,
            'User-Agent':global_user_agent,
            'Accept-Encoding':'gzip, deflate, br',
        }
        const node_opts = {
            headers,
            agent:url.startsWith('https') ? httpsKAAgent : httpKAAgent,
            timeout:opts.hasOwnProperty('timeout') ? opts.timeout : global_default_timeout,
            method:opts.hasOwnProperty('method') ? opts.method : "GET",
        }
        if (opts.hasOwnProperty('cookieJar')) {
            opts.cookieJar.getCookies(url, (err, cookies) => {
                assert(err === null, `CookieJar Error ${err}`)
                node_opts.headers['cookie'] = cookies.join('; ')
            })
        }
        if (['POST', 'PATCH'].includes(node_opts['method'])) {
            if (opts.hasOwnProperty('form')) {
                const postData = querystring.stringify(opts.form)
                node_opts.headers['Content-Type'] = 'application/x-www-form-urlencoded'
                node_opts.headers['Content-Length'] = Buffer.byteLength(postData)
            }
            if (opts.hasOwnProperty('json')) {
                const postData = JSON.stringify(opts.json)
                node_opts.headers['Content-Type'] = 'application/json'
                node_opts.headers['Content-Length'] = Buffer.byteLength(postData)
            }
        }
        let req
        if (url.startsWith('https')) {
            req = https.request(url, node_opts)
        }
        else {
            req = http.request(url, node_opts)
        }
        req.on('response', (resp) => {
            let output
            if (resp.headers['content-encoding'] === 'gzip') {
                const gunzip = zlib.createGunzip()
                resp.pipe(gunzip)
                output = gunzip
            }
            else if (resp.headers['content-encoding'] === 'deflate') {
                const inflate = zlib.createInflate()
                resp.pipe(inflate)
                output = inflate
            }
            else if (resp.headers['content-encoding'] === 'br') {
                const inflate = zlib.createBrotliDecompress()
                resp.pipe(inflate)
                output = inflate
            }
            else {
                output = resp
            }
            output.setEncoding(encoding)
            const body = []
            output.on('data', (chunk) => {
                if (encoding == 'binary') {
                    body.push(Buffer.from(chunk, 'binary'))
                }
                else {
                    body.push(chunk.toString('utf8'))
                }
            })
            output.on('end', () => {
                if (resp.headers.hasOwnProperty('set-cookie')) {
                    let cookies = []
                    if (resp.headers['set-cookie'] instanceof Array) {
                        cookies = resp.headers['set-cookie'].map(Cookie.parse)
                    }
                    else {
                        cookies = [Cookie.parse(resp.headers['set-cookie'])]
                    }
                    for (const c of cookies) {
                        if (opts.hasOwnProperty('cookieJar')) {
                            opts.cookieJar.setCookieSync(c, url)
                        }
                        else {
                            //console.log(`WARN: Skipped a cookie, missing cookiejar`)
                        }
                    }
                }
                accept({
                    headers:resp.headers,
                    statusCode:resp.statusCode,
                    body:encoding == 'utf8' ? body.join('') : Buffer.concat(body),
                    url
                })
            })
            resp.on('aborted', () => {
                reject({
                    type:'abort',
                    value:JSON.stringify({
                        url,
                        code:'ECONNRESET'
                    })
                })
            })
        })
        req.on('close', () => {
             
        })
        req.on('timeout', () => {
            req.destroy(['ETIMEDOUT'])
        })
        req.on('error', (error) => {
            reject({
                type:'http-error',
                value:JSON.stringify(error),
                url:url
            }) 
        })
        
        if (['POST', 'PATCH'].includes(node_opts['method'])) {
            if (opts.hasOwnProperty('form')) {
                req.write(querystring.stringify(opts.form))
            }

            if (opts.hasOwnProperty('json')) {
                req.write(JSON.stringify(opts.json))
            }
            if (opts.hasOwnProperty('stream')) {
                opts.stream.pipe(req)
                opts.stream.on('end', () => {
                    req.end()
                })
            }
        }
        if (!opts.hasOwnProperty('stream')) {
            req.end()
        }
    })
}














