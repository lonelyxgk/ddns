# DDNS

[中文](README.md) | English

## Description

### Platform Support

- **OS**: Windows / Linux
- **Arch**: x86 / ARM / RISC-V

### Features

- **DNS Provider**: `Cloudflare` `Porkbun` **`todo!`**
- **IP Get Method**: 支持通过`api`获取 `text` 和 `json` 解析
- **Record Type**: `A` / `AAAA` / `CNAME` / `SRV`
- **WebHook**: `Header` `Json` Request Method `HEAD` / `GET` / `POST` / `PUT` / `PATCH` / `DELETE`

## Use

- Download and unzip from [Releases](https://github.com/lonelyxgk/ddns/releases/latest)
- Run `ddns generate` to create the configuration file
- Edit `config.toml`
- Start with `ddns -c config.toml` (if the configuration file is in the same directory, you can start it directly)

## Usages

All usages are in [config.toml](examples/config.toml)
