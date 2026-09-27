# ccxt-core 0.1.5 local patch

- Source: crates.io `ccxt-core` 0.1.5, registry checksum `b8ab1b4698fd9eaa81e3f023453cde7391e873ec7f6134dd6161ff6a57206903` (original `backend/Cargo.lock`).
- Upstream repository: <https://github.com/Praying/ccxt-rust>, package VCS commit `c69f275864b7de9f93500abb97ce90723e15b0cf` (`.cargo_vcs_info.json` in the registry package).
- License: MIT, Copyright (c) 2025 Praying. `LICENSE` was copied from <https://github.com/Praying/ccxt-rust/blob/c69f275864b7de9f93500abb97ce90723e15b0cf/LICENSE>.
- Local changes: `src/http_client/builder.rs` installs a same-origin redirect policy; `src/http_client/redirect.rs` implements its pure decision rule; `src/http_client/mod.rs` includes that module. All other source files remain from the registry package.
- Consumer verification: `backend/tests/redirect_origin_test.rs` exercises the pure rule without network and `backend/tests/redirect_policy_test.rs` exercises the installed client against local HTTP servers.

When updating the package, verify the new package checksum and license, then reapply and review this security policy and both tests.
