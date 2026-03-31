# Hi Theya

Theya: Offline AI coding assistant

## Usage

Install `theya` by command:

```bash
cargo install --path .
```

By default, it use ollama server running at `http://localhost:11434` with
`qwen3-coder:30b` model. You may install the model through commands:

```bash
systemctl start ollama.service
ollama pull qwen3-coder:30b
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

Place `$HOME/.config/theya/config` with content:

```toml
[main]
# Global URI for olllama access
uri = "http://ws:11434"

[quick-chat]
model = "qwen3-coder:30b-a3b-q4_K_M"

[slow-chat]
model = "qwen3-coder:30b-a3b-q8_0"

[patch-review]
model = "qwen3.5:35b"

[code]
# You may override global URI here, so `theya code` use different ollama server
uri = "http://dev:11434"
model = "qwen3-coder:30b-a3b-q8_0"

[projects.nmstate]
# Theya use this value to compare with `git remote get-url origin`
git = "https://github.com/cathay4t/nmstate"
# Command to format the code
format = "cd rust; cargo fmt"
# Command to compile the code
compile = "cd rust; cargo build"
# Command to run link check
lint = "cd rust; cargo clippy"
# Command to run unit test
unit_test = "cd rust; cargo test"
```
