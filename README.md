# DDNS

中文 | [English](README_EN.md)

## 介绍

### 平台支持

- **操作系统**: Windows / Linux
- **架构**: x86 / ARM / RISC-V

### 功能特性

- **DNS 提供商**: `Cloudflare` **`todo!`**
- **IP 获取方式**: 支持通过`api`获取 `text` 和 `json` 解析
- **多源支持**: 可配置多个 IP 获取服务商
- **解析**: `A` / `AAAA` / `CNAME` / `SRV`
- **WebHook**: `Header` `Json` 支持 `HEAD` / `GET` / `POST` / `PUT` / `PATCH` / `DELETE`

## 使用

- 从 [Releases](https://github.com/lonelyxgk/ddns/releases/latest) 下载并解压
- 运行`ddns generate`生成配置文件
- 修改`config.toml`
- 启动`ddns -c config.toml`启动(如果配置文件在同一目录可以直接启动)

## 用法

全部用法在[config.toml](examples/config.toml)中
