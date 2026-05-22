# MemFileCLI 
一个适合使用高精度向量存储的本地轻量文件语义检索工具 — Rust CLI + ChromaDB，支持 Ollama/OpenAI 嵌入。(为了解决启动协议中 语义搜索返回旧档案的问题，给工具增加了 `--after` / `--before` 日期范围过滤功能。)
轻量级、配置驱动的记忆/笔记语义搜索，适合个人知识库、日记、Obsidian vault 等场景。
Lightweight, configuration-driven memory/note semantic search, suitable for scenarios such as personal knowledge bases, diaries, and Obsidian vaults.
##  基本介绍 (Basic Introduction)
-  **语义搜索(semantic search)**：基于向量相似度，理解自然语言查询 Understanding Natural Language Queries Based on Vector Similarity
-  **组成(composition)**：Rust CLI + Python转发 + ChromaDB，通过转发实现绕过ChromaDB向量维度锁，主要目的是保留ChromaDB无需起服务和支持元数据的特性同时使用高精度向量 This project uses Rust CLI, Python forwarding, and ChromaDB to bypass ChromaDB's vector dimension lock. The main goal is to retain ChromaDB's ability to operate without requiring services or supporting metadata while utilizing high-precision vectors.
-  **双后端嵌入（Supported embedding methods)**：Ollama（本地免费）/ OpenAI Compatible API
-  **切片规则（Embedded rules）**：Markdown 标题切分 + 长度降级 Markdown heading segmentation + length reduction
-  **增量索引（Index Method）**：检测文件变更，只更新修改过的内容/更新或新增Markdown文件后只需命令 memfilecli index --all 即可。To detect file changes and update only the modified content, or after updating or adding a Markdown file, simply use the command `memfilecli index --all`.
-  **全配置化(Fully configurable)**：JSON 配置文件，也可选择memfilecli init命令进入引导式配置。A JSON configuration file can be used, or the memfilecli init command can be selected to enter the guided configuration.
## 💡 思路 (Design Philosophy)
`memfilecli` 是一个**语义索引**。
*   **轻量**：不直接展示全文，返回**文件名、时间戳和内容片段Instead of displaying the full text, it returns the filename, timestamp, and a snippet of the content.**。
*   **推荐**：最适合配合结构化的 Markdown 文件（如 Obsidian/Logseq）和 LLM 的自动整理逻辑使用It is best suited for use with structured Markdown files (such as Obsidian/Logseq) and LLM's automatic formatting logic.。
*   **工作流示例**：
    1.  记录结构化的 Markdown 笔记到 `memory_vault`。
    2.  `memfilecli index` 建立语义索引。
    3.  当需要查阅文件时，使用memfilecli search搜索关键词获取“文件名 + 片段”。
    4.  Agent根据返回的文件名，主动读取并整理出逻辑清晰的内容分类。
> **个人看法**：此工具是文件的“语义目录”，Agent作为辅助整理回溯的“图书管理员”。📚
## 📦 安装
### 前置依赖Pre-dependencies
```bash
# Python 3.8+
python3 --version
# ChromaDB（Python）
pip install chromadb
# Rust（编译需要）
cargo --version
```
### 从源码编译Compile from source code
```bash
git clone https://github.com/yourusername/memfilecli.git
cd memfilecli
cargo build --release
cp target/release/memfilecli ~/.local/bin/
```
### 快速开始Start Process
```bash
# 1. 初始化配置（交互式向导）
memfilecli init
# 2. 索引记忆文件
memfilecli index --all
# 3. 搜索
memfilecli search "你的问题" --limit 5 --threshold 50
```
## ⚙️ 配置Configuration
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
## 📖 命令参考Command Reference
| 命令 | 说明 |
|------|------|
| `memfilecli init` | 交互式配置向导 |
| `memfilecli config` | 查看当前配置 |
| `memfilecli index --all` | 索引所有记忆文件 |
| `memfilecli search "query"` | 语义搜索 |
| `memfilecli stats` | 查看索引统计 |
| `memfilecli verify` | 检查环境依赖 |
| `memfilecli list-files` | 列出已索引文件 |
| `memfilecli search -h` | 查看搜索可带参数 |
### 常用搜索参数
```bash
# 基本搜索
memfilecli search "你的搜索关键词KeyWords" --limit 5
# 设置最低匹配度阈值（30-100）
memfilecli search "memfilecli" --threshold 50
# 指定目录索引
memfilecli index --dir /path/to/directory
# 时间筛选 - 单时间边界
memfilecli search "关键词KeyWords" --after 20260521  # 只显示2026年5月21日及之后的结果Results After May21st
memfilecli search "关键词KeyWords" --before 20260521 # 只显示2026年5月21日之前的结果Results Before May21st

# 时间范围筛选（组合使用）Combine two Date Range Command toghether For Date Range Search  
memfilecli search "关键词KeyWords" --after 20260401 --before 20260430
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
🕵️‍♂️ 可能遇到的问题（Fresh Install Test）
    编译时间太长 ⏳
*      问题：Rust 的 cargo build --release 第一次编译非常慢（尤其是带了很多依赖的时候）。用户可能会以为电脑卡死了。
*   “首次编译可能需要几分钟，请耐心等待”。
       Ollama 模型没拉取 🤖 (这是最大的坑！)
*      问题：用户装了 Ollama，但里面是空的。运行 memfilecli index 时，Rust 会尝试调用 API，如果找不到 qwen3-embedding:8b或嵌入模型，就会报错或者静默失败。
*   建议：ollama pull qwen3-embedding:8b
    
    Python 依赖缺失 🐍
*       问题：Rust 二进制文件跑起来了，但调用 Python 脚本时提示 ModuleNotFoundError:  
*          No   module named 'chromadb'。路人可能会懵：“我明明装了 Rust 啊？”
*   
     默认路径不匹配 📂
*      问题：代码里的 Default 实现是 /home/archer/memory_vault。
*        如果是 macOS 或者  Windows，第一次运行可能会报错说找不到目录。
*        建议：引导用户第一时间运行 memfilecli init，这个交互式向导会自动帮他们创建正确的路径和配置文件。
5. Rust + Python 的“混搭”困惑 🧩
*   问题：可能会问：“为什么我下载了一个 Rust 工具，却还要装 Python？”
*   建议：参考架构图（Rust 负责思考，Python 负责干活），```
```
## 📝 License
MIT License. See [LICENSE](LICENSE) for details.
