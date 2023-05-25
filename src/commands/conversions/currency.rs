use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{commands::core::strip_suffixes, Error, Result};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExchangeRatesResponse
{
    meta: ExchangeRateResponseMeta,
    data: ExchangeRateResponseData,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
struct ExchangeRateResponseData
{
    // Euro
    EUR: ExchangeRateResponseDataInfo,
    /// U.S. Dollar
    USD: ExchangeRateResponseDataInfo,
    /// Canadian Dollar
    CAD: ExchangeRateResponseDataInfo,
    /// Russian Ruble
    RUB: ExchangeRateResponseDataInfo,
    /// YEN
    JPY: ExchangeRateResponseDataInfo,
    /// Austrialian Dollar
    AUD: ExchangeRateResponseDataInfo,
    /// Armenian Dram
    AMD: ExchangeRateResponseDataInfo,
    /// Brittish Pound
    GBP: ExchangeRateResponseDataInfo,
    /// Pakistani rupee
    PKR: ExchangeRateResponseDataInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ExchangeRateResponseMeta
{
    last_updated_at: String,
}

// Echange rates are floating point numbers that represent
// value relative to USD. USD will always be 1.0
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ExchangeRateResponseDataInfo
{
    code:  String,
    value: f64,
}

impl ExchangeRatesResponse
{
    /// Makes an http reqest using the `api_key` and saves this JSON
    /// data to `ECHANGE_RATE_FILE`
    pub async fn fetch(api_key: String) -> Result<Self>
    {
        // Construct request URL
        let url = format!(
            "https://api.currencyapi.com/v3/latest?apikey={api_key}&currencies=EUR%2CUSD%2CCAD%2CRUB%2CJPY%2CAUD%2CAMD%2CGBP%2CPKR",
        );
        log::info!("Fetched currency conversion data from api.currencyapi.com");

        // Get the response
        let resp = reqwest::get(url)
            .await
            .map_err(Error::HttpRequest)?
            .json::<Self>()
            .await
            .map_err(|e| Error::JsonParse(e.to_string()))?;
        Ok(resp)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ExchangeRates
{
    /// When the exchange rates were last fetched
    when: DateTime<Utc>,

    // Euro
    eur: f64,

    /// U.S. Dollar
    usd: f64,

    /// Canadian Dollar
    cad: f64,

    /// Russian Ruble
    rub: f64,

    /// YEN
    jpy: f64,

    /// Austrialian Dollar
    aud: f64,

    /// Armenian Dram
    amd: f64,

    /// Brittish Pound
    gbp: f64,

    /// Pakistani rupee
    pkr: f64,
}

impl ExchangeRates
{
    pub async fn fetch(api_key: String) -> Result<Self>
    {
        let resp = ExchangeRatesResponse::fetch(api_key).await?;
        Ok(Self {
            /// When the exchange rates were last fetched
            when: Utc::now(),

            // Euro
            eur: resp.data.EUR.value,
            /// U.S. Dollar
            usd: resp.data.USD.value,
            /// Canadian Dollar
            cad: resp.data.CAD.value,
            /// Russian Ruble
            rub: resp.data.RUB.value,
            /// YEN
            jpy: resp.data.JPY.value,
            /// Austrialian Dollar
            aud: resp.data.AUD.value,
            /// Armenian Dram
            amd: resp.data.AMD.value,
            /// Brittish Pound
            gbp: resp.data.GBP.value,
            // Pakistani rupee
            pkr: resp.data.PKR.value,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Name
{
    // Euro
    Eur,

    /// U.S. Dollar
    Usd,

    /// Canadian Dollar
    Cad,

    /// Russian Ruble
    Rub,

    /// YEN
    Jpy,

    /// Austrialian Dollar
    Aud,

    /// Armenian Dram
    Amd,

    /// Brittish Pound
    Gbp,

    /// Pakistani rupee
    Pkr,
}

impl std::fmt::Display for Name
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let s = match self {
            Self::Usd => "Dollar(s) [USD]",
            Self::Eur => "Euro(s) [EUR]",
            Self::Cad => "Canadian Dollar(s) [CAD]",
            Self::Rub => "Ruble(s) [RUB]",
            Self::Jpy => "Yen [JPY]",
            Self::Aud => "Austriallian Dollar(s) [AUD]",
            Self::Amd => "Dram [AMD]",
            Self::Gbp => "Brittish Pound(s) [GBP]",
            Self::Pkr => "Pakistani rupee(s) [PKR]",
        };

        write!(f, "{s}")
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Currency
{
    converter: Converter,
    /// The currency of the value
    currency:  Name,

    /// The value of the currency stored in USD value
    value: f64,
}

impl Currency
{
    pub fn change_currency(&mut self, currency: Name) { self.currency = currency; }

    pub fn get_converter(&self) -> Converter { self.converter.clone() }

    pub async fn from_str(s: &str, converter: Converter) -> Result<Self>
    {
        let mut s = s.to_string();
        let currency;
        let mut value;
        match s {
            _ if s.ends_with("usd") || s.ends_with("dollar") || s.starts_with('$') => {
                s = strip_suffixes(s, &["usd", "dollar"]);
                s = match s.strip_prefix('$') {
                    Some(s) => s,
                    None => &s,
                }
                .to_string();
                currency = Name::Usd;
            }
            _ if s.ends_with("quid")
                || s.ends_with("pound")
                || s.ends_with("pounds")
                || s.ends_with("sterling")
                || s.ends_with("gbp")
                || s.starts_with('£') =>
            {
                s = strip_suffixes(s, &["quid", "pound", "pounds", "sterling", "gbp"]);
                s = match s.strip_prefix('£') {
                    Some(s) => s,
                    None => &s,
                }
                .to_string();
                currency = Name::Gbp;
            }
            _ if s.ends_with("eur") || s.ends_with("euro") || s.starts_with('€') => {
                s = strip_suffixes(s, &["eur", "eruo"]);
                s = match s.strip_prefix('€') {
                    Some(s) => s,
                    None => &s,
                }
                .to_string();
                currency = Name::Eur;
            }
            _ if s.ends_with("rub") || s.ends_with("ruble") => {
                s = strip_suffixes(s, &["ruble", "rub"]);
                currency = Name::Rub;
            }
            _ if s.ends_with("amd") || s.ends_with("dram") => {
                s = strip_suffixes(s, &["amd", "dram"]);
                currency = Name::Amd;
            }

            _ if s.ends_with("cad") => {
                s = strip_suffixes(s, &["cad"]);
                currency = Name::Cad;
            }
            _ if s.ends_with("aud") => {
                s = strip_suffixes(s, &["aud"]);
                currency = Name::Aud;
            }
            _ if s.ends_with("yen") || s.ends_with("jpy") || s.starts_with('¥') => {
                s = strip_suffixes(s, &["yen", "jpy"]);
                s = match s.strip_prefix('¥') {
                    Some(s) => s,
                    None => &s,
                }
                .to_string();
                currency = Name::Jpy;
            }
            _ if s.ends_with("pkr") || s.ends_with("pakistani rupee") => {
                s = strip_suffixes(s, &["pkr", "pakistani rupee"]);
                currency = Name::Pkr;
            }
            _ => return Err(Error::ParseNumber(format!("\"{s}\": Invalid unit provided"))),
        };

        value = s
            .trim()
            .parse()
            .map_err(|e| Error::ParseNumber(format!("\"{s}\": {e}")))?;

        // Store all currencies as USD
        let converter = Self::refresh_exchange_rates(converter).await?;

        let exchange_rates = converter.exchange_rates;
        value /= match currency {
            Name::Usd => exchange_rates.usd,
            Name::Eur => exchange_rates.eur,
            Name::Cad => exchange_rates.cad,
            Name::Rub => exchange_rates.rub,
            Name::Jpy => exchange_rates.jpy,
            Name::Aud => exchange_rates.aud,
            Name::Amd => exchange_rates.amd,
            Name::Gbp => exchange_rates.gbp,
            Name::Pkr => exchange_rates.pkr,
        };

        Ok(Currency {
            converter,
            currency,
            value,
        })
    }

    /// If the exchange rates are too old, refresh them.
    async fn refresh_exchange_rates(mut converter: Converter) -> Result<Converter>
    {
        let now = Utc::now().time();
        let when = converter.exchange_rates.when.time();
        let max_age = converter.max_age;
        let diff = when - now;

        if diff > max_age {
            let key = converter.api_key.clone();
            converter.exchange_rates = ExchangeRates::fetch(key).await?;
        }
        Ok(converter)
    }
}

impl std::fmt::Display for Currency
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        // Store all currencies as USD
        let exchange_rates = self.converter.exchange_rates;
        let value = match self.currency {
            Name::Usd => exchange_rates.usd,
            Name::Eur => exchange_rates.eur,
            Name::Cad => exchange_rates.cad,
            Name::Rub => exchange_rates.rub,
            Name::Jpy => exchange_rates.jpy,
            Name::Aud => exchange_rates.aud,
            Name::Amd => exchange_rates.amd,
            Name::Gbp => exchange_rates.gbp,
            Name::Pkr => exchange_rates.pkr,
        } * self.value;

        write!(f, "{value:.2} {}", self.currency)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Converter
{
    /// The exchange rates
    exchange_rates: ExchangeRates,

    /// The api key for the currency API
    api_key: String,

    /// The maximum valid age for the `exchange_rates` before being refreshed.
    max_age: Duration,
}

impl Converter
{
    pub async fn new(api_key: String, max_age: Duration) -> Result<Self>
    {
        Ok(Self {
            exchange_rates: ExchangeRates::fetch(api_key.clone()).await?,
            api_key,
            max_age,
        })
    }
}

pub async fn run(converter: Converter, input: String, target: String) -> Result<(String, Converter)>
{
    let mut value = Currency::from_str(&input, converter.clone()).await?;

    let initial_value = value.to_string();

    value.change_currency(match &*target.trim().to_lowercase() {
        "$" | "usd" | "dollar" => Name::Usd,
        "€" | "eur" | "euro" => Name::Eur,
        "cad" => Name::Cad,
        "rub" | "ruble" => Name::Rub,
        "¥" | "yen" | "jpy" => Name::Jpy,
        "aud" => Name::Aud,
        "amd" | "dram" => Name::Amd,
        "pound" | "sterling" | "quid" => Name::Gbp,
        "pakistani rupee" | "pkr" => Name::Pkr,
        _ => return Err(Error::CommandMisuse("Error: Invalid target currency".to_string())),
    });

    Ok((format!("{initial_value} → {value}"), value.get_converter()))
}
