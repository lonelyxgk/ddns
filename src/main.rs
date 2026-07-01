use std::fs::File;
use std::io::Write;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::process::exit;
use std::time::Duration;

use anyhow::{Context, anyhow};
use clap::{Parser, Subcommand};
use tokio::join;
use tokio::signal::ctrl_c;
use tokio::{select, task::JoinSet};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod config;
mod error;
mod provider;
mod types;
mod webhook;

use crate::config::GIP;
use crate::config::parse::{GetIP, GetIPType};
use crate::config::{get_config, init_config};
use crate::provider::task_scheduler;
use crate::types::IPType;
use crate::config::EXAMPLE_CONFIG;

#[derive(Debug, Parser)]
#[command(about = "A lightweight DDNS", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "generate config.toml", long_about = None)]
    Generate,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_level(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_ansi(true)
        .without_time()
        .compact()
        .init();

    cmd_parse()?;

    let mut tasks: JoinSet<anyhow::Result<()>> = JoinSet::new();

    let (shutdown_tx, _shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);

    // update_ip_task
    {
        let mut monitor_tick =
            tokio::time::interval(Duration::from_secs(get_config().config.monitorip));

        let mut shutdown_rx = shutdown_tx.subscribe();

        tasks.spawn(async move {
            loop {
                select! {
                    _ = monitor_tick.tick() => {
                        let v4p: usize = rand::random_range(0..get_config().config.ipv4.len());
                        let v6p: usize = rand::random_range(0..get_config().config.ipv6.len());

                        let v4 = &get_config().config.ipv4[v4p];

                        let v6 = &get_config().config.ipv6[v6p];

                        let (ipv4, ipv6) = join!(update_ip(IPType::IPV4, v4), update_ip(IPType::IPV6, v6));

                        for (ip_type, result) in [
                            (IPType::IPV4, ipv4),
                            (IPType::IPV6, ipv6),
                        ] {
                            match result {
                                Ok(ip) => {
                                    info!("update {}: {}", ip_type, ip.trim());
                                }
                                Err(err) => {
                                    error!("update {}: {}", ip_type, err);
                                }
                            }
                        };
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }

            Ok(())
        });
    };

    // sync_dns_task
    {
        let mut sync_tick = tokio::time::interval(Duration::from_secs(get_config().config.syncip));

        let mut shutdown_rx = shutdown_tx.subscribe();

        // first start wait 10 secs update record
        tokio::time::sleep(Duration::from_secs(10)).await;

        tasks.spawn(async move {
            loop {
                select! {
                    _ = sync_tick.tick() => {
                        update_record().await;

                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }

            Ok(())
        });
    };

    select! {
        signal = ctrl_c() => {
            signal.expect("fail listen ctrl + c");
        },

        result = tasks.join_next() => {
            if let Some(join_res) = result {
                match join_res {
                    Ok(Ok(())) => (),
                    Ok(Err(err)) => return Err(err),
                    Err(err) => return Err(err.into()),
                }
            }
        }
    };

    let _ = shutdown_tx.send(());
    drop(shutdown_tx);

    if let Some(join_res) = tasks.join_next().await {
        match join_res {
            Ok(Ok(())) => (),
            Ok(Err(err)) => return Err(err),
            Err(err) => return Err(err.into()),
        }
    }

    Ok(())
}

fn cmd_parse() -> anyhow::Result<()> {
    let commands_parse = Cli::parse();

    if let Some(commands) = commands_parse.command {
        match commands {
            Commands::Generate => {
                let mut f = File::create_new("config.toml")?;
                f.write_all(EXAMPLE_CONFIG.as_bytes())?;
                info!("generate config.toml");
                exit(0);
            },
        }
    };

    init_config(&commands_parse.config)?;

    Ok(())
}

async fn update_ip(iptype: IPType, getip: &GetIP) -> anyhow::Result<String> {
    let resp = reqwest::get(getip.url.as_str()).await?;
    let ipaddr =  if matches!(getip.rtype, GetIPType::Json) {
        let params = getip
            .params
            .as_ref()
            .ok_or_else(|| unreachable!())?;

            let json: serde_json::Value = resp.json().await?;

            let mut json = &json;

            for param in params.split(".") {
                json = json
                    .get(param)
                    .ok_or_else(|| anyhow!("missing json key: {}", param))?;
            }

            json
                .as_str()
                .ok_or_else(|| anyhow!("target value is not string"))?
                .to_owned()
    } else {
        resp.text().await?
    };
    match iptype {
        IPType::IPV4 => {
            let ip = ipaddr.trim().parse::<Ipv4Addr>().context("parse to ipv4addr")?;
            if GIP
                .read()
                .map_err(|err| anyhow!("read ipv4 {}", err))?
                .ipv4
                .eq(&ip)
            {
                // no change
            } else {
                GIP.write()
                    .map_err(|err| anyhow!("update ipv4 {}", err))?
                    .ipv4 = ip;
            };
        }
        IPType::IPV6 => {
            let ip = ipaddr.trim().parse::<Ipv6Addr>().context("parse to ipv6addr")?;
            if GIP
                .read()
                .map_err(|err| anyhow!("update ipv6 {}", err))?
                .ipv6
                .eq(&ip)
            {
                // no change
            } else {
                GIP.write()
                    .map_err(|err| anyhow!("update ipv6 {}", err))?
                    .ipv6 = ip;
            };
        }
    };

    Ok(ipaddr)
}

async fn update_record() {
    if let Some(dnss) = &get_config().dns { 
        for dns in dnss.values() {
            for record in &dns.records {
                task_scheduler(&dns.provider, record).await;
            }
        }
    };
}
