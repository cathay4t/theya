# Hi Theya

**Work in Progress**

My local AI coding assistant


## Usage

```bash
cargo install --path .
cd your_git_repo_need_patch_review
theya
```

By default, it use ollama server running at `http://localhost:11434` with
`qwen3-coder:30b` module. You may change it via:

```bash
env THEYA_URI="http://remote-server.example.com:11434" \
    THEYA_MODULE="qwen3-coder:480b" \
    theya
```
