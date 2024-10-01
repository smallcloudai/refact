# Refact Agent Integrations

Humans can analyze bugs on Jira/Github and debug programs, sure, if a robot can do it by itself, or help you do it,
that's fantastic!


## Architecture

Each integration is a piece of Rust code that communicates with an external program or service. There are three
things you need to know about: the configuration file, a tool for the model to call, and sessions to preserve
state between calls.


## The Config File

It's located at `~/.cache/refact/integrations.yaml`.

Even if an integration is compiled into the `refact-lsp` binary, there are 2 things needed to turn on an integration:

* The --experimental flag if the integration is still experimental
* A branch in `integrations.yaml` that the integration will check


## Tool

Look for "trait Tool" in the source code to find the abstract interface to write a new tool. Look at how
ToolGithub (our oldest integration) implements it.

A new tool also needs a description in `tools_description.rs`, and finally you need to create the tool
using `new_if_configured`.

```yaml
  - name: "github"
    agentic: true
    experimental: true
    description: "Access to gh command line command, to fetch issues, review PRs."
    parameters:
      - name: "project_dir"
        type: "string"
        description: "Look at system prompt for location of version control (.git folder) of the active file."
      - name: "command"
        type: "string"
        description: 'Examples:\ngh issue create --body "hello world" --title "Testing gh integration"\ngh issue list --author @me --json number,title,updatedAt,url\n'
    parameters_required:
      - "project_dir"
      - "command"
```

Here in the Github tool description, you see what a model needs to write to launch a command. The `project_dir` means that any
action needs to be about a repository that is already cloned. The `command` parameter mimics the command line `gh` command
any model already knows about, so it's easy for it to figure out what it has to write.


## Session

TBD


## Command Confirmation

The confirmation mechanism is common for all commands and integrations. It's two simple lists:

```yaml
commands_need_confirmation:
  - "gh * delete*"
commands_deny:
  - "gh auth token*"
```

The system will match those patterns against `TBD()` string in a tool.

