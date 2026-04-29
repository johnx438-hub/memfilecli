# MemFileCLI 🧠

**结构化记忆检索引擎** — Rust CLI + ChromaDB，支持 Ollama/OpenAI 嵌入。

轻量级、配置驱动的记忆/笔记语义搜索工具，适合个人知识库、日记、Obsidian vault 等场景。可作为利用Markdown文件做记忆文件夹的记忆目录，返回文件名，时间和片段。（作者本地Agent自己在wsl2 ubuntu用，目前还没测过其他环境。）

## ✨ 特性

- 🔍 **语义搜索**：基于向量相似度，理解自然语言查询
- 🚀 **高性能**：Rust CLI + ChromaDB，毫秒级响应
- 🧩 **双后端嵌入**：Ollama（本地免费）/ OpenAI Compatible API
- 📄 **Markdown格式切片**：Markdown 标题切分 + 长度降级
- 🔄 **增量索引**：自动检测文件变更，只更新修改过的内容
- ⚙️ **全配置化**：JSON 配置文件，支持跨机器部署

## 📦 安装

### 前置依赖

```bash
# Python 3.8+
python3 --version

# ChromaDB（Python）
pip install chromadb

# Rust（编译需要）
cargo --version
```

### 从源码编译

```bash
git clone https://github.com/yourusername/memfilecli.git
cd memfilecli
cargo build --release
cp target/release/memfilecli ~/.local/bin/
```

### 快速开始

```bash
# 1. 初始化配置（交互式向导）
memfilecli init

# 2. 索引记忆文件
memfilecli index --all

# 3. 搜索
memfilecli search "你的问题" --limit 5 --threshold 50
```

## ⚙️ 配置

配置文件位于 `~/.config/memfilecli/config.json`：

```json
{
    "general": {
        "memory_dirs": ["/path/to/your/memory_vault"],
        "db_path": "~/.memfilecli_db"
    },
    "embedding": {
        "backend": "ollama",  // ollama | openai
        "ollama": {
            "api_url": "http://localhost:11434/api/embeddings",
            "model": "qwen3-embedding:8b"
        },
        "openai": {
            "api_url": "https://api.openai.com/v1/embeddings",
            "model": "text-embedding-3-small"
        }
    },
    "chunking": {
        "use_markdown_chunking": true,
        "max_chunk_size": 500,
        "min_chunk_size": 50
    }
}
```

## 📖 命令参考

| 命令 | 说明 |
|------|------|
| `memfilecli init` | 交互式配置向导 |
| `memfilecli config` | 查看当前配置 |
| `memfilecli index --all` | 索引所有记忆文件 |
| `memfilecli search "query"` | 语义搜索 |
| `memfilecli stats` | 查看索引统计 |
| `memfilecli verify` | 检查环境依赖 |
| `memfilecli list-files` | 列出已索引文件 |

### 搜索参数

```bash
# 基本搜索
memfilecli search "今天做了什么" --limit 5

# 设置最低匹配度阈值（30-100）
memfilecli search "memfilecli" --threshold 50

# 指定目录索引
memfilecli index --dir /path/to/directory
```

## 🏗️ 架构

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Rust CLI   │────▶│ Python Script│────▶│ ChromaDB    │
│ (嵌入 + 切片) │     │ (I/O 操作)   │     │ (向量存储)   │
└─────────────┘     └──────────────┘     └─────────────┘
       ▲                      │                    │
       │                      ▼                    ▼
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│ Ollama API  │     │ JSON stdin   │     │ Cosine      │
│ OpenAI API  │     │ stdout       │     │ Similarity  │
└─────────────┘     └──────────────┘     └─────────────┘
```

## 📝 License

MIT License. See [LICENSE](LICENSE) for details.
