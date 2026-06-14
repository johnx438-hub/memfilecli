# 🚀 MemFileCli v2.0 Project Spec: Agent-First Wiki Upgrade

> **日期**: 2026-06-15  
> **作者**: Chikusa (小千) & Ge (哥)  
> **定位**: 从"本地记忆检索工具"升级为"Agent友好型知识底座 (LLM-Wiki)"  
> **核心目标**: 在保持现有高性能语义搜索的基础上，增加结构化元数据（UUID/Summary/Links），为 Agent 提供低噪声、可漫游的上下文接口。

---

## 🏗️ 1. 架构概览

```text
┌─────────────────────────────────────────────┐
│           Rust CLI (主程序)                  │
│  init / config / index / search / stats     │
│  verify / list-files                        │
└──────────┬──────────────────────────────────┘
           │ stdin JSON Protocol (不可变)
           ▼
┌─────────────────────────────────────────────┐
│         Python Bridge (ChromaDB)             │
│  write_chromadb.py ← 写入向量 + Metadata     │
│  query_chromadb.py ← 语义检索 + 聚类输出      │
└─────────────────────────────────────────────┘
```

---

## 📅 2. 阶段规划 (Phases)

### Phase 1: 数据层升级 ✅ COMPLETED
**目标**: 给 Chunk 加"身份"(UUID)、"摘要"(Summary)和"关系"(Links)，不破坏现有工作流。

| 子任务 | 改动范围 | 验收标准 | 状态 |
|--------|----------|----------|------|
| **1.1 UUID ID系统** | `write_chromadb.py`: chunk_id生成逻辑 + metadata增加`uuid`字段<br>Rust端: Chunker新增UUID生成函数 | Agent可通过稳定ID引用Chunk，文件改名不影响引用。 | ✅ 完成 |
| **1.2 Summary字段** | Rust端: 新增`extract_summary()`规则版（取第一段/标题下第一句）<br>Python端: metadata增加`summary`字段 | Index速度不下降，Metadata中包含摘要文本。 | ✅ 完成 |
| **1.3 `[[Link]]`解析** | Rust端: 新增`extract_links()`函数，正则匹配`\[\[([^\]]+)\]\]`<br>Python端: metadata增加`links`字段（JSON字符串） | Metadata中正确存储原始链接文本列表。 | ✅ 完成 |

### Phase 2: Agent接口层 ✅ COMPLETED
**目标**: 提供结构化查询命令，让 Agent 能像查数据库一样获取知识。

| 子任务 | 改动范围 | 验收标准 | 状态 |
|--------|----------|----------|------|
| **2.1 `get`命令** | Rust端: 新增`Commands::Get` + `cmd_get()`<br>Python端: `get_chromadb.py` (通过UUID查询chunk) | 返回完整上下文包（Markdown Block / JSON） | ✅ 完成 |
| **2.2 `neighbors`命令** | Rust端: 新增`Commands::Neighbors` + `cmd_neighbors()`<br>Python端: `neighbors_chromadb.py` (查找反向链接) | 列出邻近节点摘要，支持通配符匹配 | ✅ 完成 |

### Phase 3: TUI交互层
**目标**: 叠加 ratatui 界面，实现可视化知识漫游。
- **功能**: 文件树 + 编辑器 + 预览 + 图谱跳转。

---

## 🧠 3. 关键技术决策 (Consensus)

### Q1: Summary 提取策略？
- **决策**: **双轨制**。Phase 1 使用**纯规则**（取第一段，截断到100字），保证 Index 速度零延迟。
- **扩展**: `config.json` 预留 `summary.method` (`"rule"` | `"llm"`)，Phase 2 可切换为 LLM 生成摘要。

### Q2: `[[Link]]` 是否自动解析成 UUID？
- **决策**: **Phase 1 不自动解析**。只存原始文本链接（如 `["Rust所有权基础"]`）。
- **理由**: 避免引入复杂的"文件名→UUID映射表"维护逻辑，降低 Bug 风险和索引延迟。Agent 可通过搜索找到目标 UUID。

### Q3: Agent 输出格式？
- **决策**: **Markdown Block + JSON 参数**。
    - 默认: Markdown Block（人类可读，GUI 渲染友好）。
    - `--format json`: 纯 JSON（Agent 程序处理友好）。

---

## 🛡️ 4. 共识边界 (Consensus Boundaries)

### ✅ 可以改的（扩展区）
- **write_chromadb.py**: UUID生成逻辑 + metadata增加新字段 (`uuid`, `summary`, `links`)。
- **Rust Chunker**: 新增辅助函数 (`extract_summary`, `extract_links`)，修改 `enhance` 返回值结构。
- **config.json**: 新增配置项（如 summary 方法）。

### ❌ 绝对不能碰的（保护边界）
| 模块 | 保护内容 | 原因 |
|------|----------|------|
| **Rust CLI命令** | `init/config/index/search/stats/verify/list-files` 接口定义 | 现有工作流不能断，必须向后兼容。 |
| **ChromaDB metadata** | `parent_file`, `chunk_order`, `total_chunks` 字段名 | Python查询端依赖这些做聚类输出。 |
| **stdin JSON协议** | Rust→Python的 payload 结构 | 桥接通信核心，改动会导致崩溃。 |
| **增量索引逻辑** | mtime检测 + 旧chunk删除机制 (`where={"parent_file": ...}`) | 否则每次 index 都要全量重建，性能崩塌。 |

---

## 🧪 5. Phase 1 验收测试清单

```bash
# 1. 回归测试（必须通过）
memfilecli search "rust" --limit 3      # ✅ 搜索是否正常？聚类输出是否保留？
memfilecli index --rebuild              # ✅ 增量索引是否工作？旧chunks是否正确删除？
memfilecli stats                        # ✅ 统计信息是否正确？

# 2. 新字段验证（直接查 ChromaDB）
python3 -c "
import chromadb, os
client = chromadb.PersistentClient(path=os.path.expanduser('~/.memfilecli_db'))
col = client.get_collection('memfiles')
data = col.get(limit=1, include=['metadatas'])
meta = data['metadatas'][0]
assert 'uuid' in meta, 'Missing UUID'
assert 'summary' in meta, 'Missing Summary'
assert 'links' in meta, 'Missing Links'
print('✅ Metadata structure verified')
"

# 3. 精度对比（目标：≥70%）
# 记录升级前后的 Top1 匹配度，确保不下降。
```

---

## 📝 6. 附录: 数据结构示例

### ChromaDB Metadata (Phase 1)
```json
{
  "parent_file": "memory_20260429_xxx.md",
  "filename": "xxx.md",
  "date": "20260429",
  "type": "memory",
  "chunk_index": 0,
  "total_chunks": 3,
  "chunk_order": 1,
  // --- New Fields ---
  "uuid": "a1b2c3d4e5f67890", 
  "summary": "Rust使用async/await语法糖，底层基于Future trait...",
  "links": ["Rust所有权基础", "Go Goroutine"]
}
```

### Agent Output (`--format json`)
```json
{
  "id": "a1b2c3d4e5f67890",
  "title": "Rust异步模型",
  "summary": "...",
  "content": "...",
  "neighbors": [
    {"relation": "seealso", "target": "Rust所有权基础"}
  ]
}
```
