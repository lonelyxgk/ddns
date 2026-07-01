// placeholder
// ${ipv4}
// ${ipv6}
// ${currentdomain}
// ${timestamp}

use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;

use crate::{config::GIP, error::Error};

pub(crate) enum IPType {
    IPV4,
    IPV6,
}

impl fmt::Display for IPType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            IPType::IPV4 => write!(f, "IPv4"),
            IPType::IPV6 => write!(f, "IPv6"),
        }
    }
}

pub(crate) async fn placeholder_resolve_async(input: &str) -> anyhow::Result<String> {
    let input = input.to_owned();
    let result = tokio::task::spawn_blocking(move || placeholder_resolve(&input))
        .await
        .context("placeholder_resolve panicked")?;

    Ok(result?)
}

fn placeholder_resolve(content: &str) -> Result<String, Error> {
    let tokens = parse(content)?;

    let mut parsed = String::with_capacity(content.len());
    for ref token in tokens {
        match token {
            Token::Text(s) => {
                parsed.push_str(s);
            }
            Token::Var(s, _, _) => match s.as_str() {
                "ipv4" => {
                    parsed.push_str(
                        &GIP.read()
                            .map_err(|err| Error::General(format!("get ipv4 fail: {}", err)))?
                            .ipv4
                            .to_string(),
                    );
                }
                "ipv6" => {
                    parsed.push_str(
                        &GIP.read()
                            .map_err(|err| Error::General(format!("get ipv6 fail: {}", err)))?
                            .ipv6
                            .to_string(),
                    );
                }
                "timestamp" => {
                    parsed.push_str(
                        &SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map_err(|err| Error::General(format!("get timestamp fail: {}", err)))?
                            .as_secs()
                            .to_string(),
                    );
                }
                _ => {
                    parsed.push_str(s);
                }
            },
        }
    }

    Ok(parsed)
}

#[derive(Debug)]
#[allow(dead_code)]
enum Token {
    Text(String),
    Var(String, usize, usize),
}

fn parse(input: &str) -> Result<Vec<Token>, Error> {
    let mut chars = input.chars().peekable();
    let mut result = Vec::new();
    let mut buf = String::with_capacity(input.len());
    let mut pos: usize = 0;

    while let Some(c) = chars.next() {
        if c.eq(&'$') {
            let Some(c) = chars.next() else {
                // 没有下一个字符
                return Err(Error::General(format!("next char is none : {}", pos)));
            };
            pos += 1;

            if c.eq(&'$') {
                buf.push('$');
                continue;
            }

            if c.ne(&'{') {
                // 开头字符匹配失败{
                return Err(Error::General(format!("{{ failed : {}", pos)));
            }

            let start = pos;
            if !buf.is_empty() {
                result.push(Token::Text(buf.clone()));
                buf.clear();
            }

            loop {
                let Some(c) = chars.next() else {
                    // 尾部字符匹配失败}
                    return Err(Error::General(format!("}} failed : {}", pos)));
                };
                if c.eq(&'}') {
                    pos += 1;
                    let end = pos;
                    result.push(Token::Var(buf.clone(), start, end));
                    buf.clear();
                    break;
                }
                buf.push(c);
                pos += 1;
            }
        } else {
            buf.push(c);
            pos += 1;
        };
    }
    if !buf.is_empty() {
        result.push(Token::Text(buf));
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTENT: &str = "rdhghthdhgh${ipv4}hghfh$${ipv4}fghfghf${ipv4}adsdasdsad${ipv6}awdadadada${ipv6}ads$${ipv6}dadadwdawadwadade$${timestamp}ewadsafaffsf";

    #[test]
    fn test_parse() {
        let parsed = placeholder_resolve(CONTENT).unwrap();

        assert_eq!(
            parsed,
            "rdhghthdhgh0.0.0.0hghfh${ipv4}fghfghf0.0.0.0adsdasdsad::awdadadada::ads${ipv6}dadadwdawadwadade${timestamp}ewadsafaffsf"
        );
    }
}
