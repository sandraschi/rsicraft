# RSICraft manual

The manual that ships in the binary has ten tabs. The desktop Help page, `rsicraft help-tabs`, and `rsi_help` all read the same text (`crates/rsicraft-core/src/help.rs`). This file is the index. The HTML page at [`docs/help/index.html`](docs/help/index.html) is the same index in a browser.

```powershell
rsicraft help-tabs
rsicraft help-tabs --tab quickstart
rsicraft help-tabs --search "num_ctx"
```

| Tab id | Title |
| :--- | :--- |
| `start` | 1. Start here |
| `rsi` | 2. What is RSI? |
| `based_on` | 3. What this is based on |
| `quickstart` | 4. Quickstart |
| `providers` | 5. Connecting a model |
| `tasks` | 6. Writing a task |
| `concepts` | 7. How the search works |
| `api` | 8. Tools and commands |
| `troubleshoot` | 9. Troubleshooting |
| `faq` | 10. FAQ |

`tab` accepts those ids or `all`. A search is a case-insensitive substring across headings and bodies. The Troubleshooting tab has one entry for each LLM failure class (`config`, `unreachable`, `timeout`, `model_not_found`, `auth_required`, `context_overflow`, `server_error`, `bad_request`, `bad_response`, `budget_exhausted`, `empty_answer`, `no_code`).

The studio sidebar page "What is RSI?" shows the `rsi` and `based_on` tabs. The Help page shows the full set.

Older tab ids (`overview`, `rsi_landscape`, `tsinghua_openrsi`, `wrappee`, `operators`, `compare`) are not in this build.

The illustrated explainer is still at [`docs/LITTLE_TIMMY_EXPLAINS_RSI.md`](docs/LITTLE_TIMMY_EXPLAINS_RSI.md). It is a story, not the operator manual.
