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

### Configuration

Please check [config.example](config.example) for detail.
