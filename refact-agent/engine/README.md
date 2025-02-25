# Refact Agent

This is a small executable written in Rust, a part of the Refact Agent project. Its main job is to live
inside your IDE quietly and keep AST and VecDB indexes up to date. It is well-written: it will not break if
you edit your files quickly or switch branches, it caches vectorization model responses so you
don't have to wait for VecDB to complete indexing, AST supports connection graph between definitions
and usages in many popular programming languages.

Yes, it looks like an LSP server to IDE, hence the name. It can also work within a python program,
check out the [Text UI](#cli) below, you can talk about your project in the command line!

---


# Table of Contents

- [Installation](#installation)
- [Things to Try](#things-to-try)
- [Telemetry](#telemetry)
- [Caps File](#caps-file)
- [AST](#ast)
- [CLI](#cli)
- [Progress and Future Plans](#progress-and-future-plans)
- [Archiecture](#archiecture)
- [Contributing](#contributing)
- [Follow Us and FAQ](#follow-us-and-faq)
- [License](#license)


# Key Features

* Integrates with the IDE you are already using, like VSCode or JetBrains
* Offers assistant functionality: code completion and chat
* Keeps track of your source files, keeps AST and vector database up to date
* Integrates browser, databases, debuggers for the model to use
* Ask it anything! It will use the tools available to make changes to your project


## Installation

Installable by the end user:

 * VS Code https://github.com/smallcloudai/refact-vscode/

 * JetBrains IDEs https://github.com/smallcloudai/refact-intellij

 * VS Classic https://github.com/smallcloudai/refact-vs-classic/

 * Sublime Text https://github.com/smallcloudai/refact-sublime/

 * Neovim https://github.com/smallcloudai/refact-neovim

 * Refact Self-Hosting Server https://github.com/smallcloudai/refact/


### Other Important Repos

* [Documentation](https://github.com/smallcloudai/web_docs_refact_ai)
* [Chat UI](https://github.com/smallcloudai/refact-chat-js)


## Progress

- [x] Code completion with RAG
- [x] Chat with tool usage
- [x] definition() references() tools
- [x] vecdb search() with scope (semantic search)
- [x] regex_search() with scope (pattern matching)
- [x] @file @tree @web @definition @references @search mentions in chat
- [x] locate() uses test-time compute to find good project cross-section
- [x] Latest gpt-4o gpt-4o-mini
- [x] Claude-3-5-sonnet
- [x] Llama-3.1 (passthrough)
- [ ] Llama-3.2 (passthrough)
- [ ] Llama-3.2 (scratchpad)
- [x] [Bring-your-own-key](https://docs.refact.ai/byok/)
- [ ] Memory (--experimental)
- [ ] Docker integration (--experimental)
- [ ] git integration (--experimental)
- [x] pdb python debugger integration (--experimental)
- [ ] More debuggers
- [x] Github integration (--experimental)
- [ ] Gitlab integration
- [ ] Jira integration


### Compiling and Running

It will automatically pick up OPENAI_API_KEY, or maybe you have Refact cloud key or Refact Self-Hosting Server:

```
cargo build
target/debug/refact-lsp --http-port 8001 --logs-stderr
target/debug/refact-lsp --address-url Refact --api-key $REFACT_API_KEY --http-port 8001 --logs-stderr
target/debug/refact-lsp --address-url http://my-refact-self-hosting/ --api-key $REFACT_API_KEY --http-port 8001 --logs-stderr
```

Try `--help` for more options.



## Things to Try

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

Chat, the not-very-standard version, it has deterministic_messages in response for all your @-mentions. The more standard version
is at /v1/chat/completions.

```bash
curl http://127.0.0.1:8001/v1/chat -k \
  -H 'Content-Type: application/json' \
  -d '{
  "messages": [
    {"role": "user", "content": "Who is Bill Clinton? What is his favorite programming language?"}
  ],
  "stream": false,
  "temperature": 0.1,
  "max_tokens": 20
}'
```



## Telemetry

The flag `--basic-telemetry` means send counters and error messages. It is "compressed"
into `.cache/refact/telemetry/compressed` folder, then from time to time it's sent and moved
to `.cache/refact/telemetry/sent` folder.

To be clear: without these flags, no telemetry is sent. At no point it sends your code.

"Compressed" means similar records are joined together, increasing the counter. "Sent" means the rust binary
communicates with a HTTP endpoint specified in caps (see Caps section below) and sends .json file exactly how
you see it in `.cache/refact/telemetry`. The files are human-readable.

When using Refact self-hosted server, telemetry goes to the self-hosted server, not to the cloud.



## Caps File

The capabilities file stores the same things as [bring-your-own-key.yaml](bring_your_own_key), the file describes how to access AI models.
The `--address-url` parameter controls where to get this file, it defaults to `~/.config/refact/bring-your-own-key.yaml`.
If it's a URL, the executable fetches `$URL/refact-caps` to know what to do. This is especially useful to connect to Refact Self-Hosting Server,
because the configuration does not need to be copy-pasted among engineers who use the server.


## AST

Supported languages:

- [x] Java
- [x] JavaScript
- [x] TypeScript
- [x] Python
- [x] Rust
- [ ] C#

You can still use Refact for other languages, just the AST capabilities will be missing.



## CLI

You can compile and use Refact Agent from command line with this repo alone, and it's a not an afterthought, it works great!

```
cargo build --release
cp target/release/refact-lsp python_binding_and_cmdline/refact/bin/
pip install -e python_binding_and_cmdline/
```


___

## Contributing

- Contributing [CONTRIBUTING.md](CONTRIBUTING.md)
- [GitHub issues](https://github.com/smallcloudai/refact/issues) for bugs and errors
- [Community forum](https://github.com/smallcloudai/refact/discussions) for community support and discussions
If you wish to contribute to this project, feel free to explore our [current issues](https://github.com/smallcloudai/refact/issues) or open new issues related to (bugs/features) using our [CONTRIBUTING.md](CONTRIBUTING.md).


## Follow Us and FAQ

- [Contributing](CONTRIBUTING.md)
- [Refact Docs](https://docs.refact.ai/)
- [GitHub Issues](https://github.com/smallcloudai/refact/issues) for bugs and errors
- [Community Forum](https://github.com/smallcloudai/refact/discussions) for community support and discussions
- [Discord](https://www.smallcloud.ai/discord) for chatting with community members
- [Twitter](https://twitter.com/refact_ai) for product news and updates


## License

Refact is free to use for individuals and small teams under the BSD-3-Clause license. If you wish to use Refact for Enterprise, please [contact us](https://refact.ai/contact/).








