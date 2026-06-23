# MemFileCLI 🧠

本地轻量文件语义检索工具 — Rust CLI + ChromaDB，支持 Ollama/OpenAI 嵌入后端。

Lightweight, configuration-driven semantic search for local files. Built with Rust CLI + ChromaDB, supports Ollama and OpenAI embedding backends.

---

## ✨ Features / 特性

| Feature | 说明 |
|---------|------|
| **语义搜索** | 基于向量相似度，理解自然语言查询 |
| **双后端嵌入** | Ollama（本地免费）/ OpenAI Compatible API |
| **智能切片** | Markdown 标题切分 + 长度降级 |
| **增量索引** | 检测文件变更，只更新修改过的内容 |
| **Agent-First** | JSON 格式输出，支持 UUID 引用和 WikiLink 关系边 |
| **日期过滤** | `--after` / `--before` 参数按日期范围筛选 |
| **近期浏览** | `recent` 命令快速查看最近索引的 chunks |

---

## 📦 Install / 安装

### Prerequisites / 前置依赖
```bash
# Python 3.8+ + ChromaDB
pip install chromadb

# Rust (for compilation)
cargo --version

# Ollama (optional, for local embeddings)
ollama pull qwen3-embedding:8b
```

### Build from source / 从源码编译
```bash
git clone https://github.com/johnx438-hub/memfilecli.git
cd memfilecli
cargo build --release
cp target/release/memfilecli ~/.local/bin/
```

---

## 🚀 Quick Start / 快速开始

```bash
# 1. Initialize config (interactive wizard)
memfilecli init

# 2. Index memory files
memfilecli index --all

# 3. Semantic search
memfilecli search "你的问题" --limit 5

# 4. Browse recent chunks
memfilecli recent --limit 5

# 5. JSON output (Agent-friendly)
memfilecli search "关键词" --format json
```

---

## 📖 Commands / 命令参考

| Command | Description |
|---------|-------------|
| `init` | 交互式配置向导 |
| `config` | 查看当前配置 |
| `index --all` | 索引所有记忆文件（增量更新） |
| `search "query"` | 语义搜索 |
| `recent` | 按日期列出最近的 chunks |
| `get <uuid>` | 获取指定 chunk 的完整上下文 |
| `neighbors <uuid>` | 查看关联的 chunks (WikiLink) |
| `stats` | 索引统计信息 |
| `verify` | 检查环境依赖 |
| `list-files` | 列出已索引文件 |

### Search Options / 搜索参数
```bash
# Basic search with limit
memfilecli search "关键词" --limit 5

# Set similarity threshold (30-100)
memfilecli search "关键词" --threshold 50

# Date range filtering
memfilecli search "关键词" --after 20260401 --before 20260430

# JSON output for Agent consumption
memfilecli search "关键词" --format json
```

---

## ⚙️ Config / 配置

配置文件位于 `~/.config/memfilecli/config.json`：

```json
{
    "general": {
        "memory_dirs": ["/path/to/your/memory_vault"],
        "db_path": "~/.memfilecli_db"
    },
    "embedding": {
        "backend": "ollama",
        "ollama": {
            "api_url": "http://localhost:11434/api/embeddings",
            "model": "qwen3-embedding:8b"
        }
    },
    "chunking": {
        "use_markdown_chunking": true,
        "max_chunk_size": 500,
        "min_chunk_size": 50
    }
}
```

---

## 🏗️ Architecture / 架构

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Rust CLI   │────▶│ Python Script│────▶│ ChromaDB    │
│ (嵌入 + 切片) │     │ (I/O 操作)   │     │ (向量存储)   │
└─────────────┘     └──────────────┘     └─────────────┘
       ▲                      │                    │
       │                      ▼                    ▼
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  Ollama API  │     │ JSON stdin   │     │ Cosine      │
│ OpenAI API  │     │ stdout       │     │ Similarity  │
└─────────────┘     └──────────────┘     └─────────────┘
```

---

## 💡 Design Philosophy / 设计思路

`memfilecli` 是一个**语义索引工具**。它不直接展示全文，而是返回文件名、时间戳和内容片段。最适合配合结构化的 Markdown 文件（如 Obsidian/Logseq）和 Agent 的自动整理逻辑使用。

> **工作流**: 记录 Markdown → `index` 建立索引 → `search/recent` 检索 → Agent 读取完整内容

---

## 📝 License
MIT License.
