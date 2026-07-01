use std::collections::HashMap;

use reqwest::{
    Client, RequestBuilder, Response,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum Type {
    Head,
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

pub(crate) async fn excute_webhook(url: &str, rtype: &Type, headers: &Option<HashMap<String, String>>, json: &Option<String>, client: Client) -> Result<Response, Error> {
    let mut request: RequestBuilder;
    match rtype {
        Type::Head => {
            request = client.head(url);
        }
        Type::Get => {
            request = client.get(url);
        }
        Type::Post => {
            request = client.post(url);
        }
        Type::Put => {
            request = client.put(url);
        }
        Type::Patch => {
            request = client.patch(url);
        }
        Type::Delete => {
            request = client.delete(url);
        }
    };
    if let Some(headers) = &headers {
        let mut headersmap = HeaderMap::new();
        for (k, v) in headers {
            headersmap.insert(
                HeaderName::from_bytes(k.as_bytes())?,
                HeaderValue::from_str(v.as_str())?,
            );
        }
        request = request.headers(headersmap)
    };
    if let Some(json) = &json {
        request = request.json(&json);
    };
    let resp = request.send().await?;

    Ok(resp)
}
