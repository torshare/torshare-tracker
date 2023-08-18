# Torshare BitTorrent Tracker

[![codecov](https://codecov.io/gh/torshare/torshare-tracker/branch/main/graph/badge.svg?token=7K4JSP89Z0)](https://codecov.io/gh/torshare/torshare-tracker)

Torshare Tracker is a high-performance and feature-rich BitTorrent tracker written in Rust. It's a type of server that assists in the communication between clients using the [BitTorrent protocol](https://www.bittorrent.org/beps/bep_0003.html).

## Features
* [X] Supports multiple types of transfer protocols
  - HTTP
  - UDP ([BEP15](https://www.bittorrent.org/beps/bep_0015.html))
* [X] IPv4 and IPv6 support
* [X] Private Torrents
* [X] Reverse Proxy Support

## Implemented BEPs
* [BEP 3](https://www.bittorrent.org/beps/bep_0003.html): The BitTorrent Protocol
* [BEP 7](https://www.bittorrent.org/beps/bep_0007.html): IPv6 Tracker Extension
* [BEP 23](http://bittorrent.org/beps/bep_0023.html): Tracker Returns Compact Peer Lists
* [BEP 31](https://www.bittorrent.org/beps/bep_0031.html): Failure Retry Extension
* [BEP 41](https://www.bittorrent.org/beps/bep_0041.html): UDP Tracker Protocol Extension
* [BEP 48](https://www.bittorrent.org/beps/bep_0048.html): Tracker Protocol Extension: Scrape

## Installation

```sh
git clone https://github.com/torshare/torshare-tracker.git
cargo run
```