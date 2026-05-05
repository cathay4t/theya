<!-- vim-markdown-toc GFM -->

* [Hi Theya](#hi-theya)
    * [Usage](#usage)
        * [Usage: Patch Review](#usage-patch-review)
        * [Usage: Quick chat](#usage-quick-chat)
        * [Usage: Slow Chat](#usage-slow-chat)
        * [Usage: Code](#usage-code)
        * [Usage: Memory](#usage-memory)
            * [Populate the knowledge base](#populate-the-knowledge-base)
            * [List entries](#list-entries)
            * [Search](#search)
            * [Backup and restore](#backup-and-restore)
            * [Recalculate embedding vectors](#recalculate-embedding-vectors)
    * [Configuration](#configuration)

<!-- vim-markdown-toc -->

# Hi Theya

Theya: Offline AI coding assistant

## Usage

Install `theya` by command:

```bash
cargo install --path .
```

Thyea is using OpenAI API compatible server as backend, you may run
ollama/shimmy locally or use any cloud services.

To use ollama locally, you need to modify ollama.service to allow it using
256k or more context windows size. For example, run
`systemctl edit ollama.service` and add content below:

```
[Service]
Environment="OLLAMA_CONTEXT_LENGTH=262144"
```

Place `$HOME/.config/theya/config` with content:

```toml
[main]
# URI for olllama access
uri = "http://localhost:11434"
api_key = "olllama-do-not-need-a-key"

[quick-chat]
model = "qwen3-coder:30b-a3b-q4_K_M"

[slow-chat]
model = "qwen3.6:35b-a3b-q8_0"

[patch-review]
model = "qwen3.6:35b-a3b-q8_0"

[code]
# You may override global URI here
uri = "http://dev:11434"
model = "qwen3.6:35b-a3b-q8_0"

[memory]
# Use local model to prevent data leak
embed_dimensions = 1024
embed_uri = "http://localhost:11434"
embed_model = "qwen3-embedding:8b"
model = "qwen3.6:35b-a3b-q8_0"
uri = "http://localhost:11434"
copilot = true
```

### Usage: Patch Review

Review the last git commit in a git repo.

```bash
cd <your_git_repo_need_patch_review>
theya pr
```

### Usage: Quick chat


```bash
theya chat
# Type your question in the editor, save and quit.
# The quick short answer will shows ollama replys
```

### Usage: Slow Chat

```bash
theya chat --slow
# Type your question in the editor, save and quit.
# The lengthy complex answer will shows ollama replys
```

### Usage: Code

Theya can code on given task in side of a git repo.

```bash
theya code
# Type your coding task in the editor, save and quit.
```

### Usage: Memory

Theya can build a local long-term knowledge base backed by a
[LanceDB](https://lancedb.github.io/lancedb/) vector database stored in
`$HOME/.local/share/theya/knowledge_db`.

An embedding model is required. Configure it under `[memory]` in your config
file (see **Configuration** below).

#### Populate the knowledge base

Extract and index knowledge from your GitHub Copilot chat history:

```bash
theya memory update
# or: theya m update
```

This command automatically extracts important facts and learnings from your
recent Copilot sessions and stores them in the knowledge base. Each entry is
tagged with the project it was extracted from (format: `host_type:owner/repo`,
e.g., `github:cathay4t/theya`), allowing you to organize knowledge by project.

Add a file directly:

```bash
theya memory add <file_path>
```

Compose a free-form note in `$EDITOR`:

```bash
theya memory add --interactive
```

#### List entries

View all stored knowledge entries:

```bash
theya memory list
# or: theya m list
```

Displays all entries in a table format showing:
- **ID**: Unique identifier for the entry
- **Created At**: Timestamp of creation
- **Title**: Brief description of the entry

#### Search

```bash
theya memory search <prompt>
```

#### Backup and restore

Dump the entire database (entries + embedding vectors) to a JSON file:

```bash
theya memory dump <output_file>
```

Restore from a previous dump (prompts whether to wipe existing data first):

```bash
theya memory load <input_file>
```

#### Recalculate embedding vectors

When you change `embed_model` in the config, all stored vectors are
automatically recalculated the next time any `theya memory` subcommand runs.
You can also trigger this manually:

```bash
theya memory recalc
```

## Configuration

Please check [config.example](config.example) for detail.
