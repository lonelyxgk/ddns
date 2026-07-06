use std::collections::HashMap;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use url::Url;
use validator::{Validate, ValidateArgs, ValidationError};

use crate::config::get_config;
use crate::provider::Provider;
use crate::types::placeholder_resolve_async;
use crate::webhook::{Type, excute_webhook};

#[derive(Serialize, Deserialize, Debug, Validate)]
pub(crate) struct Config {
    #[validate(nested)]
    pub(crate) config: GConfig,
    pub(crate) dns: Option<HashMap<String, Dns>>,
    #[validate(nested)]
    pub(crate) webhook: Option<HashMap<String, WebHook>>,
}

#[derive(Serialize, Deserialize, Debug, Validate)]
#[validate(schema(function = "validate_getip"))]
pub(crate) struct GetIP {
    #[validate(url)]
    pub(crate) url: String,
    #[serde(rename = "type")]
    pub(crate) rtype: GetIPType,
    pub(crate) params: Option<String>
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GetIPType {
    Text,
    Json
}

#[derive(Serialize, Deserialize, Debug, Validate)]
pub(crate) struct GConfig {
    pub(crate) ipv4: Vec<GetIP>,
    pub(crate) ipv6: Vec<GetIP>,
    pub(crate) monitorip: u64,
    pub(crate) syncip: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Dns {
    pub(crate) provider: Provider,
    pub(crate) records: Vec<Resolv>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum ResolvType {
    A,
    Aaaa,
    Cname,
    Srv,
}

impl ResolvType {
    pub(crate) fn as_string(&self) -> String {
        match &self {
            ResolvType::A => String::from("A"),
            ResolvType::Aaaa => String::from("AAAA"),
            ResolvType::Cname => String::from("CNAME"),
            ResolvType::Srv => String::from("SRV")
        }
    }
}

type WebHookMap = HashMap<String, WebHook>;
#[derive(Serialize, Deserialize, Debug, Validate, Clone)]
#[validate(context = WebHookMap)]
pub(crate) struct Resolv {
    pub(crate) domain: String,
    #[serde(rename = "type")]
    pub(crate) rtype: ResolvType,
    pub(crate) hostname: String,
    pub(crate) target: String,
    #[validate(custom(function = "validate_triggers", use_context))]
    pub(crate) triggers: Option<Vec<String>>,
}

impl Resolv {
    pub(crate) async fn excute_triggers(&self) {
        if let Some(triggers) = &self.triggers {
            let mut tasks = tokio::task::JoinSet::new();
            let client = reqwest::Client::new();
            for trigger in triggers {
                #[allow(clippy::collapsible_if)]
                if let Some(webhooks) = &get_config().webhook {
                    if let Some(webhook) = webhooks.get(trigger) {
                    let url = match placeholder_resolve_async(&webhook.url).await {
                        Ok(ok) => ok,
                        Err(err) => {
                            error!("resolve url failed: {:?}", err);
                            continue;
                        }
                    };
                    let json = match &webhook.json {
                        Some(v) => match placeholder_resolve_async(v).await {
                            Ok(v) => Some(v),
                            Err(err) => {
                                error!("resolve json failed: {:?}", err);
                                continue;
                            }
                        },
                        None => None,
                    };

                    {
                        let client = client.clone();
                        tasks.spawn(async move {
                            match excute_webhook(&url, &webhook.rtype, &webhook.headers, &json, client).await {
                                Ok(resp) => {
                                    if !resp.status().is_success() {
                                        warn!("excute_triggers: {:?} resp: {:?}", &webhook, resp);
                                    } else {
                                        info!("excute_triggers: {:?} resp: {:?}", &webhook, resp);
                                    };
                                }
                                Err(err) => {
                                    error!("excute_triggers: {:?} error: {:?}", &webhook, err);
                                }
                            }
                        });
                    };

                    while let Some(result) = tasks.join_next().await {
                        if let Err(err) = result {
                            error!("excute_triggers -> join_error: {:?}", err);
                        }
                    }
                };
                };
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Validate)]
pub(crate) struct WebHook {
    #[serde(rename = "type")]
    pub(crate) rtype: Type,
    #[validate(url)]
    pub(crate) url: String,
    pub(crate) headers: Option<HashMap<String, String>>,
    #[validate(custom(function = "validate_is_value"))]
    pub(crate) json: Option<String>,
}

pub(super) fn check(config: &Config) -> anyhow::Result<()> {
    config.validate()?;

    if config.config.monitorip.gt(&config.config.syncip) {
        return Err(anyhow!("monitorip must be less than syncip"));
    };

    if let Some(dnss) = &config.dns {
        for dns in dnss.values() {
            for resolv in &dns.records {
                if let Some(webhooks) = &config.webhook {
                    resolv.validate_with_args(webhooks)?;
                }
            };
        }
    };

    Ok(())
}

fn validate_triggers(
    triggers: &Vec<String>,
    context: &HashMap<String, WebHook>,
) -> Result<(), ValidationError> {
    for trigger in triggers {
        if context.get(trigger.as_str()).is_none() {
            return Err(ValidationError::new("invalid_tigger")
                .with_message(format!("This trigger does not exist: {}", trigger).into()));
        }
    }

    Ok(())
}

fn validate_is_value(content: &String) -> Result<(), ValidationError> {
    serde_json::from_str::<serde_json::Value>(content).map_err(|err| {
        ValidationError::new("invalid_json")
            .with_message(format!("field {} is not a json: {}", content, err).into())
    })?;

    Ok(())
}

#[allow(unused)]
fn validate_urls(urls: &[String]) -> Result<(), ValidationError> {
    for (i, url_str) in urls.iter().enumerate() {
        match Url::parse(url_str) {
            Ok(_) => continue,
            Err(e) => {
                return Err(ValidationError::new("invalid_url")
                    .with_message(format!("item {} is not a valid URL: {}", i, e).into()));
            }
        }
    }
    Ok(())
}

fn validate_getip(getip: &GetIP) -> Result<(), ValidationError> {
    if getip.rtype.eq(&GetIPType::Json) {
        if let Some(params) = &getip.params {
            if params.is_empty() {
                return Err(ValidationError::new("parsms is_empty"));
            }
        } else {
            return Err(ValidationError::new("parsms is none"));
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Read};

    #[test]
    fn parse_test() {
        let mut file = File::open("examples/config.toml").unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        println!("{}", buf);

        let result = toml::from_str::<Config>(&buf).unwrap();

        assert!(check(&result).is_ok());
        
        dbg!(result);
    }
}
