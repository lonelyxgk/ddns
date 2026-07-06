use http::StatusCode;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::Error;
use crate::config::parse::Resolv;
use crate::types::placeholder_resolve_async;

#[allow(unused)]
#[derive(Debug, Clone, Deserialize)]
struct DomainResponse {
    status: String,
    domain: DomainInfo
}

#[allow(unused)]
#[derive(Debug, Clone, Deserialize)]
struct DomainInfo {
    domain: String,
    status: String,
    tld: String,
    #[serde(rename = "createDate")]
    create_date: String,
    #[serde(rename = "expireDate")]
    expire_date: String,
    #[serde(rename = "securityLock")]
    security_lock: String,
    #[serde(rename = "whoisPrivacy")]
    whois_privacy: String,
    #[serde(rename = "autoRenew")]
    auto_renew: i32,
    #[serde(rename = "apiAccess")]
    api_access: i32,
    #[serde(rename = "notLocal")]
    not_local: i32,
}

#[allow(unused)]
#[derive(Debug, Clone, Deserialize)]
struct GetDomainRecords {
    status: String,
    cloudflare: String,
    records: Vec<String>
}

#[allow(unused)]
#[derive(Debug, Clone, Deserialize)]
struct Records {
    id: String,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
    ttl: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateDNSRequest<'a> {
    apikey: &'a str,
    secretapikey: &'a str,
    name: &'a str,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
    ttl: i32,
    #[serde(rename = "dryRun")]
    dry_run: bool
}

#[derive(Debug, Clone, Serialize)]
struct UpdateDNSRequest<'a> {
    apikey: &'a str,
    secretapikey: &'a str,
    content: String,
    ttl: i32,
}

async fn get_domain_info(key: &str, secret: &str, client:Client, resolv: &Resolv) -> Result<DomainResponse, Error> {
    
    let resp = client.get(format!("https://api.porkbun.com/api/json/v3/domain/get/{}", resolv.domain))
        .header("X-API-Key", key)
        .header("X-Secret-API-Key",secret)
        .send().await?;

    if resp.status().eq(&StatusCode::NOT_FOUND) {
        return Err(Error::General(format!("domain not exist: {}", resolv.domain)));
    } else if !resp.status().is_success() {
        return Err(Error::General(format!("request fail: {}", resp.text().await?)));
    }

    let resp = resp.json::<DomainResponse>().await?;

    Ok(resp)
}

async fn check_record_exist(key: &str, secret: &str, client:Client, resolv: &Resolv) -> Result<GetDomainRecords, Error> {
    let resp = client.get(format!("https://api.porkbun.com/api/json/v3/dns/retrieveByNameType/{}/{}/{}", resolv.domain, resolv.rtype.as_string(), resolv.hostname))
        .header("X-API-Key", key)
        .header("X-Secret-API-Key",secret)
        .send().await?;

    let resp = resp.json::<GetDomainRecords>().await?;

    Ok(resp)
}

async fn overwrite_and_create_record(
    key: &str, secret: &str,
    client: Client,
    resolv: &Resolv
) -> Result<Response, Error> {

    let exist_resp = check_record_exist(key, secret, client.clone(), resolv).await?;
    
    let resp: Response;
    if exist_resp.records.is_empty() {
        let request = CreateDNSRequest {
            apikey: key,
            secretapikey: secret,
            name: &resolv.hostname,
            record_type: resolv.rtype.as_string(),
            content: placeholder_resolve_async(&resolv.target).await.map_err(|err| Error::General(format!("{}", err)))?,
            ttl: 0,
            dry_run: false
        };

        resp = client.post(format!("https://api.porkbun.com/api/json/v3/dns/create/{}", resolv.domain))
            .header("Content-Type", "application/json")
            .json(&request)
            .send().await?;
    } else {
        let request = UpdateDNSRequest {
            apikey: key,
            secretapikey: secret,
            content: placeholder_resolve_async(&resolv.target).await.map_err(|err| Error::General(format!("{}", err)))?,
            ttl: 0,
        };

        resp = client.post(format!("https://api.porkbun.com/api/json/v3/dns/editByNameType/{}/{}/{}", resolv.domain, resolv.rtype.as_string(), resolv.hostname))
            .header("Content-Type", "application/json")
            .json(&request)
            .send().await?;
    };

    Ok(resp)
}


pub(super) async fn request(key: String, secret: String, resolv: Resolv) -> Result<(), Error> {
    let client = Client::new();

    let resp = get_domain_info(&key, &secret, client.clone(), &resolv).await?;

    if resp.domain.api_access.ne(&1) {
        return Err(Error::General(format!("domain disable api_access: {}", resolv.domain)));
    }

    let resp = overwrite_and_create_record(&key, &secret, client, &resolv).await?;
    
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