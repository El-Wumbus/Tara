#![allow(clippy::upper_case_acronyms)]

use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{
    commands::common::{ends_with_any, equals_any, strip_suffixes},
    Error, Result,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ExchangeRateResponseMeta {
    last_updated_at: String,
}

// Echange rates are floating point numbers that represent
// value relative to USD. USD will always be 1.0
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ExchangeRateResponseDataInfo {
    code:  String,
    value: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExchangeRatesResponse {
    meta: ExchangeRateResponseMeta,
    data: ExchangeRateResponseData,
}

impl ExchangeRatesResponse {
    /// Makes an http reqest using the `api_key` and saves this JSON
    /// data to `ECHANGE_RATE_FILE`
    pub async fn fetch(api_key: &str) -> Result<Self> {
        // Construct request URL
        let url = format!(
            "https://api.currencyapi.com/v3/latest?apikey={api_key}&currencies={}",
            CURRENCIES_URL_PART.join("%2C")
        );

        event!(
            Level::INFO,
            "Fetched currency conversion data from api.currencyapi.com"
        );

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

#[derive(Debug, Clone, PartialEq)]
pub struct Converter {
    /// The exchange rates
    exchange_rates: ExchangeRates,

    /// The api key for the currency API
    api_key: String,

    /// The maximum valid age for the `exchange_rates` before being refreshed.
    max_age: Duration,
}

impl Converter {
    pub async fn new(api_key: String, max_age: Duration) -> Result<Self> {
        Ok(Self {
            exchange_rates: ExchangeRates::fetch(&api_key).await?,
            api_key,
            max_age,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Currency {
    converter: Converter,
    /// The currency of the value
    currency:  Name,

    /// The value of the currency stored in USD value
    value: f64,
}

impl Currency {
    pub fn change_currency(&mut self, currency: Name) { self.currency = currency; }

    pub fn get_converter(&self) -> Converter { self.converter.clone() }

    /// If the exchange rates are too old, refresh them.
    async fn refresh_exchange_rates(mut converter: Converter) -> Result<Converter> {
        let now = Utc::now().time();
        let when = converter.exchange_rates.when.time();
        let max_age = converter.max_age;
        let diff = when - now;

        if diff > max_age {
            let key = converter.api_key.as_str();
            converter.exchange_rates = ExchangeRates::fetch(key).await?;
        }
        Ok(converter)
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Store all currencies as USD
        let value = self.converter.exchange_rates.pick_from_name(self.currency) * self.value;

        write!(f, "{value:.2} {}", self.currency)
    }
}

macro_rules! make_currency_structures {
    ($(($currency: ident, $pretty_name:expr, $prefix:expr, $allowed_suffixes:expr)), *) => {

        static CURRENCIES_URL_PART: Lazy<Vec<&str>> = Lazy::new(|| vec![$(stringify!($currency)),*]);
        pub static SUPPORTED_CURRENCIES: Lazy<String> = Lazy::new(|| {
            vec![$($pretty_name),*]
                .into_iter()
                .map(|x| format!("- {x}"))
                .collect::<Vec<String>>()
                .join("\n")
        });

        #[derive(Serialize, Deserialize, Clone, Debug)]
        #[allow(non_snake_case)]
        struct ExchangeRateResponseData {
            $($currency: ExchangeRateResponseDataInfo),*
        }

        #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
        #[allow(non_snake_case)]
        pub struct ExchangeRates {
            /// When the exchange rates were last fetched
            when: DateTime<Utc>,

            $($currency: f64),*
        }

        impl ExchangeRates {
            pub async fn fetch(api_key: &str) -> Result<Self> {
                let resp = ExchangeRatesResponse::fetch(api_key).await?;
                Ok(Self {
                    /// When the exchange rates were last fetched
                    when: Utc::now(),
                    $($currency: resp.data.$currency.value),*
                })
            }

            #[inline]
            pub fn pick_from_name(&self, name: Name) -> f64 {
                match name {
                    $(Name::$currency => self.$currency),*
                }
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Name {
            $($currency),*
        }

        impl std::fmt::Display for Name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let s = match self {
                    $(Self::$currency => $pretty_name),*
                };

                write!(f, "{s}")
            }
        }

        impl Name {
            pub fn from_str(s: &str) -> Result<Self> {
                let s = s.trim().to_lowercase();
                match s {
                    $(_ if equals_any(&s, $allowed_suffixes) => {
                        Ok(Self::$currency)
                    })*
                    _ => Err(Error::CommandMisuse("Error: Invalid target currency".to_string())),
                }
            }
        }

        impl Currency {
            pub async fn from_str(s: &str, converter: Converter) -> Result<Self> {
                let mut s = s.to_lowercase();
                let currency;

                match &s {
                $(_ if ends_with_any(&s, $allowed_suffixes) || optionally_starts_with(&s, $prefix) => {
                        s = strip_suffixes(&s, $allowed_suffixes);
                        if let Some(prefix) = $prefix {
                            s = s.strip_prefix(prefix).unwrap_or(&s).to_string();
                        }
                        currency = Name::$currency;
                }) *
                _ => return Err(Error::ParseNumber(format!("\"{s}\": Invalid currency provided"))),
                }

                let mut value = s
                .trim()
                .parse()
                .map_err(|e| Error::ParseNumber(format!("\"{s}\": {e}")))?;

                // Store all currencies as USD
                let converter = Self::refresh_exchange_rates(converter).await?;

                value /= converter.exchange_rates.pick_from_name(currency);

                Ok(Currency {
                    converter,
                    currency,
                    value,
                })
            }
        }

    };
}

#[inline]
fn optionally_starts_with(s: &str, c: Option<char>) -> bool { c.map_or(false, |c| s.starts_with(c)) }

make_currency_structures!(
    (EUR, "Euro(s) [EUR]", Some('€'), &["eur", "euro", "euros"]),
    (USD, "US Dollar [USD]", Some('$'), &["usd", "dollar", "dollars"]),
    (CAD, "Canadian Dollar [CAD]", None::<char>, &["cad"]),
    (
        RUB,
        "Russian Ruble [RUB]",
        None::<char>,
        &["rub", "ruble", "rubles"]
    ),
    (JPY, "Yen [JPY]", Some('¥'), &["jpy", "yen",]),
    (AUD, "Austrialian Dollar [AUD]", None::<char>, &["aud"]),
    (AMD, "Armenian Dram [AMD]", None::<char>, &["amd", "dram"]),
    (PKR, "Pakistani rupee [PKR]", None::<char>, &["pkr"]),
    (
        GBP,
        "Brittish Pound [GBP]",
        Some('£'),
        &["gbp", "quid", "pound", "pounds", "sterling"]
    ),
    (
        CNY,
        "Chinese Yuan Renminbi [CNY]",
        None::<char>,
        &["cny", "renminbi", "yuán", "yuan"]
    )
);


pub async fn run(converter: Converter, input: String, target: &str) -> Result<(String, Converter)> {
    let mut value = Currency::from_str(&input, converter.clone()).await?;

    let initial_value = value.to_string();
    value.change_currency(Name::from_str(target)?);

    Ok((format!("{initial_value} → {value}"), value.get_converter()))
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::{Converter, Currency};

    #[tokio::test]
    async fn test_currency_parse_suffix() {
        let converter = Converter {
            exchange_rates: super::ExchangeRates {
                when: Utc::now(),
                USD:  1.0,
                EUR:  1.0,
                CAD:  1.0,
                RUB:  1.0,
                JPY:  1.0,
                AUD:  1.0,
                AMD:  1.0,
                GBP:  1.0,
                PKR:  1.0,
                CNY:  1.0,
            },
            api_key:        String::new(),
            max_age:        Duration::days(69),
        };


        let currency = Currency::from_str("182 USD", converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 dollar", currency.converter)
            .await
            .unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 dollars", currency.converter)
            .await
            .unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("182 EUR", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 euro", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 euros", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("182 CAD", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("182 RUB", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 ruble", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 rubles", currency.converter)
            .await
            .unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("182 JPY", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 yen", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("182 AUD", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("182 AMD", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 dram", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("182 GBP", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 quid", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 pound", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 sterling", currency.converter)
            .await
            .unwrap();
        assert_eq!(currency.value, 182.0);
        let currency = Currency::from_str("182 pounds", currency.converter)
            .await
            .unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("182 PKR", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);

        assert!(Currency::from_str("182", currency.converter).await.is_err());
    }

    #[tokio::test]
    async fn test_currency_parse_prefix() {
        let converter = Converter {
            exchange_rates: super::ExchangeRates {
                when: Utc::now(),
                USD:  1.0,
                EUR:  1.0,
                CAD:  1.0,
                RUB:  1.0,
                JPY:  1.0,
                AUD:  1.0,
                AMD:  1.0,
                GBP:  1.0,
                PKR:  1.0,
                CNY:  1.0,
            },
            api_key:        String::new(),
            max_age:        Duration::days(69),
        };

        let currency = Currency::from_str("$182", converter).await.unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("€182", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("¥182", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);

        let currency = Currency::from_str("£182", currency.converter).await.unwrap();
        assert_eq!(currency.value, 182.0);
    }
}
