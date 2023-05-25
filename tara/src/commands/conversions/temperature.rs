use std::{fmt, str::FromStr};

use crate::{commands::CommandResponse, Error};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Temperature {
    temp: f64,
    kind: TemperatureUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
enum TemperatureUnit {
    Kelvin,
    Celsius,
    Fahrenheit,
}

impl FromStr for Temperature {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.to_lowercase();
        let kind;
        s = match s {
            _ if s.ends_with('c') || s.ends_with("celsius") || s.ends_with("cel") => {
                kind = TemperatureUnit::Celsius;
                match s.strip_suffix('c') {
                    Some(x) => x.to_string(),
                    None => {
                        match s.strip_suffix("celsius") {
                            Some(x) => x.to_string(),
                            None => {
                                match s.strip_suffix("cel") {
                                    Some(x) => x.to_string(),
                                    None => s,
                                }
                            }
                        }
                    }
                }
            }

            _ if s.ends_with('f') || s.ends_with("fahrenheit") || s.ends_with("fah") => {
                kind = TemperatureUnit::Fahrenheit;
                match s.strip_suffix('f') {
                    Some(x) => x.to_string(),
                    None => {
                        match s.strip_suffix("fahrenheit") {
                            Some(x) => x.to_string(),
                            None => {
                                match s.strip_suffix("fah") {
                                    Some(x) => x.to_string(),
                                    None => s,
                                }
                            }
                        }
                    }
                }
            }

            _ if s.ends_with('k') || s.ends_with("kelvin") || s.ends_with("kel") => {
                kind = TemperatureUnit::Kelvin;
                match s.strip_suffix('k') {
                    Some(x) => x.to_string(),
                    None => {
                        match s.strip_suffix("kelvin") {
                            Some(x) => x.to_string(),
                            None => {
                                match s.strip_suffix("kel") {
                                    Some(x) => x.to_string(),
                                    None => s,
                                }
                            }
                        }
                    }
                }
            }

            _ => return Err(Error::CommandMisuse("INVALID UNIT".to_string())),
        };

        Ok(Self {
            kind,
            temp: match s.trim().parse() {
                Ok(x) => {
                    match kind {
                        TemperatureUnit::Kelvin => x,
                        TemperatureUnit::Celsius => x + 273.15,
                        TemperatureUnit::Fahrenheit => (x - 32.0) * 5.0 / 9.0 + 273.15,
                    }
                }
                Err(e) => return Err(Error::ParseNumber(format!("\"{}\": {e}", s.trim()))),
            },
        })
    }
}

impl std::fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (temp, unit) = match self.kind {
            TemperatureUnit::Kelvin => (self.temp, "Kelvin"),
            TemperatureUnit::Celsius => (self.temp - 273.15, "Celsius"),
            TemperatureUnit::Fahrenheit => ((self.temp - 273.15) * 9.0 / 5.0 + 32.0, "Fahrenheit"),
        };

        write!(f, "{temp:.2} {unit}")
    }
}

impl Temperature {
    pub fn as_cel(&mut self) -> &mut Self {
        self.kind = TemperatureUnit::Celsius;
        self
    }

    pub fn as_kel(&mut self) -> &mut Self {
        self.kind = TemperatureUnit::Kelvin;
        self
    }

    pub fn as_fah(&mut self) -> &mut Self {
        self.kind = TemperatureUnit::Fahrenheit;
        self
    }
}

pub fn convert(input: &str, output: &str) -> crate::Result<CommandResponse> {
    let mut temperature = Temperature::from_str(input)?;
    Ok(match output {
        "k" | "kel" | "kelvin" => temperature.as_kel(),
        "c" | "cel" | "celsius" => temperature.as_cel(),
        "f" | "fah" | "fahrenheit" => temperature.as_fah(),
        _ => &mut temperature,
    }
    .to_string()
    .into())
}
