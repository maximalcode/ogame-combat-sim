use std::error::Error as StdError;
use std::fmt;
use std::io;

use reqwest::StatusCode;

use crate::Endpoint;

/// Failure to validate, fetch, cache or parse one metadata response.
#[derive(Debug)]
pub enum Error {
    InvalidUniverse(String),
    InvalidContact(String),
    BuildClient(reqwest::Error),
    Cache {
        endpoint: Endpoint,
        source: io::Error,
    },
    Request {
        endpoint: Endpoint,
        source: reqwest::Error,
    },
    HttpStatus {
        endpoint: Endpoint,
        status: StatusCode,
    },
    Decode {
        endpoint: Endpoint,
        source: std::string::FromUtf8Error,
    },
    Parse {
        endpoint: &'static str,
        source: quick_xml::DeError,
    },
    StaleResponse {
        endpoint: Endpoint,
        timestamp: u64,
    },
    InvalidLifeform(String),
    Clock(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUniverse(value) => write!(
                formatter,
                "invalid universe {value:?}; expected a server name such as s1-en"
            ),
            Self::InvalidContact(value) => write!(
                formatter,
                "invalid User-Agent contact {value:?}; expected an https:// or mailto: address"
            ),
            Self::BuildClient(_) => formatter.write_str("build OGame XML HTTP client"),
            Self::Cache { endpoint, .. } => write!(formatter, "cache {endpoint}"),
            Self::Request { endpoint, .. } => write!(formatter, "request {endpoint}"),
            Self::HttpStatus { endpoint, status } => {
                write!(formatter, "request {endpoint}: server returned {status}")
            }
            Self::Decode { endpoint, .. } => {
                write!(formatter, "decode {endpoint} as UTF-8")
            }
            Self::Parse { endpoint, .. } => write!(formatter, "parse {endpoint}"),
            Self::StaleResponse {
                endpoint,
                timestamp,
            } => write!(
                formatter,
                "response root for {endpoint} is stale (timestamp {timestamp})"
            ),
            Self::InvalidLifeform(message) => {
                write!(
                    formatter,
                    "invalid serverData.xml lifeform settings: {message}"
                )
            }
            Self::Clock(_) => formatter.write_str("read the system clock"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::BuildClient(source) | Self::Request { source, .. } => Some(source),
            Self::Cache { source, .. } | Self::Clock(source) => Some(source),
            Self::Decode { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::InvalidUniverse(_)
            | Self::InvalidContact(_)
            | Self::HttpStatus { .. }
            | Self::StaleResponse { .. }
            | Self::InvalidLifeform(_) => None,
        }
    }
}
