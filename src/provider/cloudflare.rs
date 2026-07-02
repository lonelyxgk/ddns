//! Cloudflare
//! Permissions
//! Zone Read
//! DNS Read
//! DNS Write

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tracing::{info, warn};

use crate::{
    config::
        parse::{Resolv, ResolvType}, error::Error, types::placeholder_resolve_async
};

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct CloudflareResponse {
    result: Vec<Zone>,
    result_info: ResultInfo,
    success: bool,
    errors: Vec<serde_json::Value>,
    messages: Vec<serde_json::Value>,
}

impl PagesResult for CloudflareResponse {
    type Item = Zone;

    fn result_info(&self) -> &ResultInfo {
        &self.result_info
    }

    fn success(&self) -> bool {
        self.success
    }

    fn errors(&self) -> &Vec<serde_json::Value> {
        &self.errors
    }

    fn merge(&mut self, merge: Vec<Self::Item>) {
        self.result.extend(merge);
    }

    fn inner_result(self) -> Vec<Self::Item> {
        self.result
    }
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct Zone {
    id: String,
    name: String,
    status: String,
    paused: bool,
    #[serde(rename = "type")]
    zone_type: String,
    permissions: Vec<String>,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct Records {
    result: Vec<Record>,
    result_info: ResultInfo,
    success: bool,
    errors: Vec<serde_json::Value>,
    messages: Vec<serde_json::Value>,
}

impl PagesResult for Records {
    type Item = Record;

    fn result_info(&self) -> &ResultInfo {
        &self.result_info
    }

    fn success(&self) -> bool {
        self.success
    }

    fn inner_result(self) -> Vec<Self::Item> {
        self.result
    }

    fn errors(&self) -> &Vec<serde_json::Value> {
        &self.errors
    }

    fn merge(&mut self, merge: Vec<Self::Item>) {
        self.result.extend(merge);
    }
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct Record {
    id: String,
    name: String,
    #[serde(rename = "type")]
    record_type: RecordType,
    content: String,
    proxiable: bool,
    proxied: bool,
    ttl: i32,
    #[serde(default)]
    settings: HashMap<String, serde_json::Value>,
    #[serde(default)]
    meta: HashMap<String, serde_json::Value>,
    comment: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    created_on: DateTime<Utc>,
    modified_on: DateTime<Utc>,
    #[serde(default)]
    comment_modified_on: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum RecordType {
    A,
    Aaaa,
    Cname,
    Srv,
    #[serde(other)]
    Other,
}

impl RecordType {
    fn from(value: &ResolvType) -> Self {
        match value {
            ResolvType::A => Self::A,
            ResolvType::Aaaa => Self::Aaaa,
            ResolvType::Cname => Self::Cname,
            ResolvType::Srv => Self::Srv,
        }
    }
    #[allow(clippy::inherent_to_string)]
    fn to_string(&self) -> String {
        match self {
            RecordType::A => String::from("A"),
            RecordType::Aaaa => String::from("AAAA"),
            RecordType::Cname => String::from("CNAME"),
            RecordType::Srv => String::from("SRV"),
            RecordType::Other => { unreachable!() },
        }
    }
}

#[derive(Serialize, Debug)]
struct UploadRecord {
    name: String,
    ttl: i32,
    #[serde(rename = "type")]
    record_type: String,
    comment: String,
    content: String,
    private_routing: bool,
    proxied: bool,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct ResultInfo {
    page: i32,
    per_page: i32,
    total_pages: i32,
    count: i32,
    total_count: i32,
}

trait PagesResult {
    type Item;

    fn result_info(&self) -> &ResultInfo;
    fn success(&self) -> bool;
    fn errors(&self) -> &Vec<serde_json::Value>;
    fn inner_result(self) -> Vec<Self::Item>;
    fn merge(&mut self, merge: Vec<Self::Item>);
}

async fn pages_api_request<T>(
    url: &str,
    auth: &str,
    per_page: usize,
    page: usize,
) -> Result<T, Error>
where
    T: DeserializeOwned + PagesResult,
{
    let client = Client::new();

    let mut aresp = get_next_page::<T>(client.clone(), url, auth, per_page, page).await?;

    if aresp.success().ne(&true) {
        return Err(Error::Response(format!(
            "success is false: {:?}",
            aresp.errors()
        )));
    };

    {
        let page = aresp.result_info().page;
        let total_pages = aresp.result_info().total_pages;
        if page.ne(&total_pages) {
            for current_page in (page + 1)..=total_pages {
                let resp =
                    get_next_page::<T>(client.clone(), url, auth, per_page, current_page as usize).await?;

                if resp.success().ne(&true) {
                    return Err(Error::Response(format!(
                        "success is false: {:?}",
                        resp.errors()
                    )));
                };

                aresp.merge(resp.inner_result());
            }
        };
    };

    Ok(aresp)
}

async fn get_next_page<T>(
    client: Client,
    url: &str,
    auth: &str,
    per_page: usize,
    page: usize,
) -> Result<T, Error>
where
    T: DeserializeOwned + PagesResult,
{
    let resp = client
        .get(url)
        .query(&[("per_page", per_page), ("page", page)])
        .bearer_auth(auth)
        .send()
        .await?;

    let resp = resp.json::<T>().await?;
    Ok(resp)
}

async fn overwrite_and_create_record(
    zone_id: &str,
    auth: &str,
    records: &Records,
    resolv: &Resolv,
) -> Result<Response, Error> {
    let full_record_name = format!("{}.{}", resolv.hostname, resolv.domain);
    let record = records
        .result
        .iter()
        .find(|v| v.name == full_record_name && v.record_type == RecordType::from(&resolv.rtype));
    let client = Client::new();
    let rtype = RecordType::from(&resolv.rtype);
    let upload_record = UploadRecord {
        name: full_record_name,
        ttl: 1,
        record_type: rtype.to_string(),
        comment: String::from(""),
        content: placeholder_resolve_async(&resolv.target).await.map_err(|err| Error::General(format!("{}", err)))?,
        private_routing: false,
        proxied: false,
    };
    let resp: Response;
    if let Some(record) = record {
        resp = client
            .put(format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                zone_id, record.id
            ))
            .header("Content-Type", "application/json")
            .bearer_auth(auth)
            .json(&upload_record)
            .send()
            .await?;
    } else {
        resp = client
            .post(format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
                zone_id
            ))
            .header("Content-Type", "application/json")
            .bearer_auth(auth)
            .json(&upload_record)
            .send()
            .await?;
    };
    Ok(resp)
}

pub(super) async fn request(auth: String, resolv: Resolv) -> Result<(), Error> {
    let resp = pages_api_request::<CloudflareResponse>(
        "https://api.cloudflare.com/client/v4/zones",
        &auth,
        100,
        1,
    )
    .await?;

    let zone = resp
        .result
        .iter()
        .find(|v| v.name == resolv.domain)
        .ok_or(Error::General(format!("domain not exist: {}", resolv.domain)))?;

    let records = pages_api_request::<Records>(
        &format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            zone.id
        ),
        &auth,
        100,
        1,
    )
    .await?;

    let resp = overwrite_and_create_record(&zone.id, &auth, &records, &resolv).await?;
    
    if !resp.status().is_success() {
        warn!("{}.{} update fail", &resolv.domain, &resolv.hostname)
    } else {
        info!("{}.{} updated", &resolv.hostname, &resolv.domain)
    }

    tokio::spawn(async move {
        resolv.excute_triggers().await;
    });

    Ok(())
}
