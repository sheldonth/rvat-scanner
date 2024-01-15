use reqwest::{ClientBuilder, header, Response};
use chrono::{DateTime, FixedOffset};
use std::env;
use serde::Deserialize;

fn load_env_var(key: &str) -> String {
    match env::var(key) {
        Ok(val) => val,
        Err(e) => panic!("couldn't interpret {}: {}", key, e),
    }
}

  //{
    //"date": "2023-12-13",
    //"open": "09:30",
    //"close": "16:00",
    //"session_open": "0400",
    //"session_close": "2000",
    //"settlement_date": "2023-12-15"
  //}

#[derive(Deserialize, Debug, Clone)]
pub struct Calendar {
    pub date: String,
    pub open: String,
    pub close: String,
    pub session_open: String,
    pub session_close: String,
    pub settlement_date: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Bar {
    pub t:DateTime<FixedOffset>,
    pub o:serde_json::Value,
    pub h:serde_json::Value,
    pub l:serde_json::Value,
    pub c:serde_json::Value,
    pub v:serde_json::Value
}

#[derive(Deserialize, Debug, Clone)]
pub struct BarResponse {
    symbol: String,
    bars: Vec<Bar>,
}


#[derive(Debug)]
pub enum AlpacaClientError {
    ReqwestError(reqwest::Error),
}

impl From<reqwest::Error> for AlpacaClientError {
    fn from(e: reqwest::Error) -> Self {
        Self::ReqwestError(e)
    }
}

// async alpaca client
pub struct AlpacaClient {
    pub client:reqwest::Client,
}

impl AlpacaClient {
    pub fn new() -> Result<AlpacaClient, AlpacaClientError> {
        let mut headers = header::HeaderMap::new();
        headers.insert("APCA-API-KEY-ID", header::HeaderValue::from_str(&load_env_var("APCA_API_KEY_ID")).unwrap());
        headers.insert("APCA-API-SECRET-KEY", header::HeaderValue::from_str(&load_env_var("APCA_API_SECRET_KEY")).unwrap());
        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()?;
        Ok(AlpacaClient { client })
    }
}

pub fn get_calendar(start:DateTime<FixedOffset>, end:DateTime<FixedOffset>) -> Vec<Calendar>{
    let mut headers = header::HeaderMap::new();
    headers.insert("APCA-API-KEY-ID", header::HeaderValue::from_str(&load_env_var("APCA_API_KEY_ID")).unwrap());
    headers.insert("APCA-API-SECRET-KEY", header::HeaderValue::from_str(&load_env_var("APCA_API_SECRET_KEY")).unwrap());
    reqwest::blocking::Client::new()
        .get("https://api.alpaca.markets/v2/calendar")
        .query(&[("start", start.to_rfc3339()), ("end", end.to_rfc3339())])
        .headers(headers)
        .send()
        .unwrap()
        .json::<Vec<Calendar>>()
        .unwrap()
}

pub fn get_bars(ticker:&str, timeframe:&str, start:DateTime<FixedOffset>, end:DateTime<FixedOffset>, limit:&str) -> BarResponse{
    let mut headers = header::HeaderMap::new();
    headers.insert("APCA-API-KEY-ID", header::HeaderValue::from_str(&load_env_var("APCA_API_KEY_ID")).unwrap());
    headers.insert("APCA-API-SECRET-KEY", header::HeaderValue::from_str(&load_env_var("APCA_API_SECRET_KEY")).unwrap());
    let mut resp = reqwest::blocking::Client::new()
        .get(format!("https://data.alpaca.markets/v2/stocks/{ticker}/bars"))
        .query(&[("limit", limit), ("timeframe", timeframe), ("adjustment", "all")])
        .headers(headers)
        .send()
        .unwrap()
        .json::<BarResponse>()
        .unwrap();
    resp.bars.reverse(); // reverse the bars so they start with most recent
    resp
}

#[cfg(test)]
mod tests {

    //#[tokio::test]
    #[test]
    fn get_calendar() {
        let end = chrono::DateTime::parse_from_rfc3339("2023-01-12T00:00:00-05:00").unwrap();
        let start = chrono::DateTime::parse_from_rfc3339("2021-01-10T00:00:00-05:00").unwrap();
        let calendar = super::get_calendar(start, end);
        assert!(calendar.len() > 0, "Calendar is empty");
    }

    #[test]
    fn test_get_bars() {

        //let bars = super::get_bars("AAPL", "1Min", "1000");
        //println!("{:#?}", bars.bars);
    }
}

