use serde::{Deserialize, Serialize};
use tracing::error;

use crate::config::parse::Resolv;

mod cloudflare;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "serect")]
pub(crate) enum Provider {
    CloudFlare { key: String },
}

pub(crate) async fn task_scheduler(provider: &Provider, resolv: &Resolv) {
    let resolv = resolv.clone();
    match provider {
        Provider::CloudFlare { key } => {
            let key = key.clone();
            tokio::spawn(async move {
                if let Err(err) = cloudflare::request(key.clone(), resolv).await {
                    error!("cloudflare: {:?}", &err);
                }
            });
        }
    };
}
