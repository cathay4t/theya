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

To use different model and/or remote ollama server:

```bash
env THEYA_URI="http://remote-server.example.com:11434" \
    THEYA_MODULE="qwen3-coder:480b" \
    theya
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

## Usage: Slow Chat

```bash
theya chat --slow
# Type your question in the editor, save and quit.
# The lengthy complex answer will shows ollama replys
```
