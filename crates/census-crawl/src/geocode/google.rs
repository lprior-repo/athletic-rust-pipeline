use census_domain::school_directory::Coordinates;
use serde::Deserialize;

use super::key::SecretKey;
use super::transport::Transport;

const ENDPOINT: &str = "https://maps.googleapis.com/maps/api/geocode/json";
const COORDINATE_DECIMALS: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeocodeQuery {
    pub street: String,
    pub city: String,
    pub state: String,
    pub postal_code: Option<String>,
}

impl GeocodeQuery {
    pub fn address(&self) -> String {
        let mut address = format!("{}, {}, {}", self.street, self.city, self.state);
        if let Some(postal_code) = &self.postal_code {
            address.push(' ');
            address.push_str(postal_code);
        }
        address
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeocodeOutcome {
    Found {
        coordinates: Coordinates,
        formatted_address: Option<String>,
        location_type: Option<String>,
    },
    ZeroResults,
    Denied,
    InvalidRequest,
    OverQueryLimit,
    OverDailyLimit,
    UnknownStatus {
        status: String,
    },
    Unusable {
        detail: String,
    },
    Transport {
        detail: String,
    },
}

pub struct GoogleGeocoder<T: Transport> {
    key: SecretKey,
    transport: T,
}

impl<T: Transport> GoogleGeocoder<T> {
    pub fn new(key: SecretKey, transport: T) -> Self {
        Self { key, transport }
    }

    pub async fn geocode(&self, query: &GeocodeQuery) -> GeocodeOutcome {
        let url = match request_url(&self.key, query) {
            Ok(url) => url,
            Err(detail) => return GeocodeOutcome::Unusable { detail },
        };
        let body = match self.transport.get_json(url.as_str(), None).await {
            Ok(body) => body,
            Err(failure) => {
                return GeocodeOutcome::Transport {
                    detail: self.key.redact(failure.detail.as_str()),
                };
            }
        };
        interpret(body.as_str())
    }
}

fn request_url(key: &SecretKey, query: &GeocodeQuery) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(ENDPOINT).map_err(|error| error.to_string())?;
    url.query_pairs_mut()
        .append_pair("address", query.address().as_str())
        .append_pair("key", key.expose());
    Ok(url)
}

fn interpret(body: &str) -> GeocodeOutcome {
    let response: Response = match serde_json::from_str(body) {
        Ok(response) => response,
        Err(error) => {
            return GeocodeOutcome::Unusable {
                detail: error.to_string(),
            };
        }
    };
    match response.status.as_str() {
        "OK" => found(&response),
        "ZERO_RESULTS" => GeocodeOutcome::ZeroResults,
        "REQUEST_DENIED" => GeocodeOutcome::Denied,
        "INVALID_REQUEST" => GeocodeOutcome::InvalidRequest,
        "OVER_QUERY_LIMIT" => GeocodeOutcome::OverQueryLimit,
        "OVER_DAILY_LIMIT" => GeocodeOutcome::OverDailyLimit,
        other => GeocodeOutcome::UnknownStatus {
            status: other.to_string(),
        },
    }
}

fn found(response: &Response) -> GeocodeOutcome {
    let Some(first) = response.results.first() else {
        return GeocodeOutcome::Unusable {
            detail: "status OK carried no result".to_string(),
        };
    };
    let geometry = first.geometry.as_ref();
    let location = geometry.and_then(|geometry| geometry.location.as_ref());
    let Some(location) = location else {
        return GeocodeOutcome::Unusable {
            detail: "status OK carried no geometry location".to_string(),
        };
    };
    let latitude = format!("{:.*}", COORDINATE_DECIMALS, location.lat);
    let longitude = format!("{:.*}", COORDINATE_DECIMALS, location.lng);
    match Coordinates::parse(latitude.as_str(), longitude.as_str()) {
        Ok(coordinates) => GeocodeOutcome::Found {
            coordinates,
            formatted_address: first.formatted_address.clone(),
            location_type: geometry.and_then(|geometry| geometry.location_type.clone()),
        },
        Err(error) => GeocodeOutcome::Unusable {
            detail: error.to_string(),
        },
    }
}

#[derive(Deserialize)]
struct Response {
    status: String,
    #[serde(default)]
    results: Vec<ResultEntry>,
}

#[derive(Deserialize)]
struct ResultEntry {
    #[serde(default)]
    formatted_address: Option<String>,
    #[serde(default)]
    geometry: Option<Geometry>,
}

#[derive(Deserialize)]
struct Geometry {
    #[serde(default)]
    location: Option<Location>,
    #[serde(default)]
    location_type: Option<String>,
}

#[derive(Deserialize)]
struct Location {
    lat: f64,
    lng: f64,
}
