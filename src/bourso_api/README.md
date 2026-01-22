# Bourso API

Unofficial API for [BoursoBank/Boursorama](https://www.boursorama.com) written in Rust.

```txt
src/bourso_api/src
├── lib.rs
├── client/                # everything talking to Bourso (reqwest, cookies, parsing)
│   ├── mod.rs             # BoursoWebClient + shared helpers
│   ├── auth.rs            # init/login/mfa/virtual pad
│   ├── accounts.rs        # fetching account pages, parsing regex
│   ├── trading.rs         # quotes/orders/ticks HTTP logic
│   ├── transfers.rs
│   └── errors.rs
├── models/                # data structures shared across features
│   ├── account.rs
│   ├── trading.rs         # order, quote, enums, DTOs
│   └── value_types.rs     # ClientNumber, AccountId, Password, etc.
├── features/              # self-contained workflows callable from CLI
│   ├── session.rs         # init + login + optional MFA orchestration
│   ├── list_accounts.rs
│   ├── place_order.rs
│   └── transfer_funds.rs
├── constants.rs           # URLs, regex patterns
└── utils/ (optional)      # generic helpers (HTML extraction, formatting)
```
