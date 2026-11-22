[![Continuous Integration](https://github.com/nicklaswallgren/bankid-rs/workflows/test/badge.svg)](https://github.com/nicklaswallgren/bankid-rs/actions)
[![License](https://img.shields.io/github/license/nicklaswallgren/bankid-rs)](https://github.com/nicklaswallgren/bankid-rs/blob/master/LICENSE)
[![Crates.io](https://img.shields.io/crates/v/bankid.svg)](https://crates.io/crates/bankid)
[![Docs](https://docs.rs/bankid/badge.svg)](https://docs.rs/crate/bankid/)

# BankID-rs

BankID-rs is a BankID SDK for the Relying Party API v6.0 `/auth`, `/sign`, `/collect` and `/cancel` endpoints.

The client is built from a `Config` holding the RP API base URL, the CA certificates that issue the server's TLS
certificate, and the relying party's client identity. `Config::prod` and `Config::test` hold BankID's published
endpoints and CAs; `Config::new` points the client anywhere else, such as a local stand-in for the API.

To learn how to use BankID-rs, please refer to the [documentation](https://docs.rs/crate/bankid-rs/). There are some [examples that may be useful](./examples) as well.

## Changelog

Please see the [changelog](./CHANGELOG.md) for a release history and indications on how to upgrade from one version to another.

## Contributing

If you find any problems or have suggestions about this crate, please submit an issue. Moreover, any pull request, code review and feedback are welcome.

### Code Guide

We use GitHub Actions to make sure the codebase is consistent (`cargo +nightly fmt`) and continuously tested (`cargo test`). We try to keep comments at a maximum of 120 characters of length and code at 120.

## Building

```sh
$ cargo build
$ cargo test
```

## License

[MIT](./LICENSE)
