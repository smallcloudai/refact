
# Refact Agent Rust Executable

This is a small executable written in Rust, a part of Refact Agent project. Its main job is to live
inside your IDE quietly and keep AST and VecDB indexes up to date. It's well-written, it will not break if
you edit your files really fast or switch branches, it caches vectorization model responses so you
don't have to wait for VecDB to complete indexing, AST supports full graph connection between definitions
and usage in many popular programming languages, etc.


## Progress

- [x] Code completion with RAG
- [x] Chat using
- [x] definition() / references() tools
- [x] vecdb search() with scope
- [x] @file @tree @definition @references @web @search mentions in chat
- [x] locate() spends test-time compute to find good project cross-section
- [x] gpt-4o gpt-4o-mini
- [x] claude-3-5-sonnet
- [x] llama-3.1 (passthrough)
- [ ] llama-3.2 (passthrough)
- [ ] llama-3.2 (scratchpad)
- [x] [bring-your-own-key](https://docs.refact.ai/byok/)
- [ ] Memory (--experimental)
- [ ] Docker integration (--experimental)
- [ ] git integration (--experimental)
- [x] pdb python debugger integration (--experimental)
- [ ] More debuggers
- [x] github integration (--experimental)
- [ ] gitlab integration
- [ ] Jira integration


## Refact Agent

For end user:

* [VS Code](https://github.com/smallcloudai/refact-vscode/)
* [JetBrains IDEs](https://github.com/smallcloudai/refact-intellij)
* [VS Classic](https://github.com/smallcloudai/refact-vs-classic/)
* [Sublime Text](https://github.com/smallcloudai/refact-sublime/)
* [Neovim](https://github.com/smallcloudai/refact-neovim)

Refact Self-Hosting Server:

* [Refact](https://github.com/smallcloudai/refact/)

Other important repos:

* [Documentation](https://github.com/smallcloudai/web_docs_refact_ai)
* [HTML/JS chat UI](https://github.com/smallcloudai/refact-chat-js)


## Compiling and Running

Depending on which API key you have handy, or maybe you have Refact cloud or self-hosting key:

```
cargo build && target/debug/refact-lsp --http-port 8001 --logs-stderr
cargo build && target/debug/refact-lsp --address-url Refact --api-key $REFACT_API_KEY --http-port 8001 --logs-stderr
cargo build && target/debug/refact-lsp --address-url http://my-refact-self-hosting/ --api-key $REFACT_API_KEY --http-port 8001 --logs-stderr
```

Try `--help` for more options.


## Things to try

Code completion:

```bash
curl http://127.0.0.1:8001/v1/code-completion -k \
  -H 'Content-Type: application/json' \
  -d '{
  "inputs": {
    "sources": {"hello.py": "def hello_world():"},
    "cursor": {
      "file": "hello.py",
      "line": 0,
      "character": 18
    },
    "multiline": true
  },
  "stream": false,
  "parameters": {
    "temperature": 0.1,
    "max_new_tokens": 20
  }
}'
```

RAG status:

```bash
curl http://127.0.0.1:8001/v1/rag-status
```


## Telemetry

The flag `--basic-telemetry` means counters and error messages. It is "compressed"
into `.cache/refact/telemetry/compressed` folder, then from time to time it's sent and moved
to `.cache/refact/telemetry/sent` folder.

To be clear: without these flags, no telemetry is sent. At no point it sends your code.

"Compressed" means similar records are joined together, increasing the counter. "Sent" means the rust binary
communicates with a HTTP endpoint specified in caps (see Caps section below) and sends .json file exactly how
you see it in `.cache/refact/telemetry`. The files are human-readable.

When using Refact self-hosted server, telemetry goes to the self-hosted server, not to the cloud.


## Caps File

The `--address-url` parameter controls the behavior of this program by a lot. The address is first used
to construct `$URL/coding_assistant_caps.json` address to fetch the caps file. Furthermore, there are
compiled-in caps you can use by magic addresses "Refact" or make your personal configuration for to use other services.
Take a look examples in [bring_your_own_key](bring_your_own_key)

The caps file describes which models are running, default models for completion and chat,
where to send the telemetry, how to download a
tokenizer, where is the endpoint to access actual language models. To read more, check out
compiled-in caps in [caps.rs](src/caps.rs).


## AST

Supported languages:

- [x] Java
- [x] JavaScript
- [x] TypeScript
- [x] Python
- [x] Rust
- [ ] C#

You can still use Refact for other languages, just the AST capabilities will be missing.


