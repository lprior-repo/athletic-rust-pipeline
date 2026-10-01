use census_domain::school_directory::{CityName, StreetLine, ZipCode};
use census_domain::UsJurisdiction;
use serde::Deserialize;

use super::key::SecretKey;
use super::transport::Transport;

const ENDPOINT: &str = "https://apis.usps.com/addresses/v3/address";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationQuery {
    pub street: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationOutcome {
    Validated {
        street: Option<StreetLine>,
        city: Option<CityName>,
        state: Option<UsJurisdiction>,
        zip: Option<ZipCode>,
        delivery_point: Option<String>,
        confirmation: Option<String>,
    },
    Rejected {
        code: Option<String>,
        message: Option<String>,
    },
    Unparsed {
        detail: String,
    },
    Transport {
        detail: String,
    },
}

pub struct UspsValidator<T: Transport> {
    token: SecretKey,
    transport: T,
}

impl<T: Transport> UspsValidator<T> {
    pub fn new(token: SecretKey, transport: T) -> Self {
        Self { token, transport }
    }

    pub async fn validate(&self, query: &ValidationQuery) -> ValidationOutcome {
        let url = match request_url(query) {
            Ok(url) => url,
            Err(detail) => return ValidationOutcome::Unparsed { detail },
        };
        let authorization = format!("Bearer {}", self.token.expose());
        let body = match self
            .transport
            .get_json(url.as_str(), Some(authorization.as_str()))
            .await
        {
            Ok(body) => body,
            Err(failure) => {
                return ValidationOutcome::Transport {
                    detail: self.token.redact(failure.detail.as_str()),
                };
            }
        };
        interpret(body.as_str())
    }
}

fn request_url(query: &ValidationQuery) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(ENDPOINT).map_err(|error| error.to_string())?;
    url.query_pairs_mut()
        .append_pair("streetAddress", query.street.as_str())
        .append_pair("city", query.city.as_str())
        .append_pair("state", query.state.as_str())
        .append_pair("zip", query.postal_code.as_str());
    Ok(url)
}

fn interpret(body: &str) -> ValidationOutcome {
    let response: Response = match serde_json::from_str(body) {
        Ok(response) => response,
        Err(error) => {
            return ValidationOutcome::Unparsed {
                detail: error.to_string(),
            };
        }
    };
    if let Some(error) = response.error {
        return ValidationOutcome::Rejected {
            code: error.code,
            message: error.message,
        };
    }
    let Some(address) = response.address else {
        return ValidationOutcome::Unparsed {
            detail: "response carried neither an address nor an error".to_string(),
        };
    };
    validated(address, response.additional_info)
}

fn validated(address: AddressEntry, additional: Option<AdditionalInfo>) -> ValidationOutcome {
    let street = match present(address.street_address) {
        Some(raw) => match StreetLine::parse(raw.as_str()) {
            Ok(street) => Some(street),
            Err(error) => {
                return ValidationOutcome::Unparsed {
                    detail: error.to_string(),
                };
            }
        },
        None => None,
    };
    let city = match present(address.city) {
        Some(raw) => match CityName::parse(raw.as_str()) {
            Ok(city) => Some(city),
            Err(error) => {
                return ValidationOutcome::Unparsed {
                    detail: error.to_string(),
                };
            }
        },
        None => None,
    };
    let state = present(address.state).and_then(|raw| UsJurisdiction::from_code(raw.as_str()));
    let zip = match present(address.zip_code) {
        Some(code) => {
            let plus4 = present(address.zip_plus4);
            match ZipCode::of(code.as_str(), plus4.as_deref()) {
                Ok(zip) => Some(zip),
                Err(error) => {
                    return ValidationOutcome::Unparsed {
                        detail: error.to_string(),
                    };
                }
            }
        }
        None => None,
    };
    if street.is_none() && city.is_none() && state.is_none() && zip.is_none() {
        return ValidationOutcome::Unparsed {
            detail: "address object carried no usable field".to_string(),
        };
    }
    let (delivery_point, confirmation) = match additional {
        Some(info) => (present(info.delivery_point), present(info.confirmation)),
        None => (None, None),
    };
    ValidationOutcome::Validated {
        street,
        city,
        state,
        zip,
        delivery_point,
        confirmation,
    }
}

fn present(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    #[serde(default)]
    address: Option<AddressEntry>,
    #[serde(default)]
    error: Option<ErrorEntry>,
    #[serde(default)]
    additional_info: Option<AdditionalInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressEntry {
    #[serde(default)]
    street_address: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default, rename = "ZIPCode")]
    zip_code: Option<String>,
    #[serde(default, rename = "ZIPPlus4")]
    zip_plus4: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorEntry {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdditionalInfo {
    #[serde(default)]
    delivery_point: Option<String>,
    #[serde(default, rename = "DPVConfirmation")]
    confirmation: Option<String>,
}
