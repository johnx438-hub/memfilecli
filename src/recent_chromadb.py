import chromadb
import sys
import json
from collections import defaultdict

# Read JSON payload from stdin
payload = json.loads(sys.stdin.read())
db_path = payload["db_path"]
collection_name = payload.get("collection_name", "memfiles")
limit = payload.get("limit", 5)
output_format = payload.get("format", "text")  # text or json

client = chromadb.PersistentClient(path=db_path)
try:
    collection = client.get_collection(collection_name)
except Exception as e:
    print(f"ERROR|Collection not found|{e}", file=sys.stderr)
    sys.exit(1)

# Get all chunks with metadata (ChromaDB get() doesn't support sorting, so we sort in Python)
results = collection.get(
    include=["documents", "metadatas"]
)

if not results["ids"]:
    if output_format == "json":
        print(json.dumps({"total_results": 0, "files": []}, indent=2, ensure_ascii=False))
    else:
        print("📭 No chunks found in database.")
    sys.exit(0)

# Step 1: Collect all chunks with date info
all_chunks = []
for i in range(len(results["ids"])):
    meta = results["metadatas"][i]
    date_str = meta.get("date", "unknown")
    
    # Skip entries without valid dates
    if date_str == "unknown" or len(date_str) != 8:
        continue
    
    all_chunks.append({
        "id": results["ids"][i],
        "doc": results["documents"][i][:500],
        "filename": meta.get("filename", "unknown"),
        "parent_file": meta.get("parent_file", meta.get("filename", "unknown")),
        "date": date_str,
        "chunk_order": meta.get("chunk_order", 0),
        "total_chunks": meta.get("total_chunks", 1),
        "uuid": meta.get("uuid", "N/A"),
        "summary": meta.get("summary", "")
    })

# Step 2: Sort by date descending (most recent first)
all_chunks.sort(key=lambda x: x["date"], reverse=True)

# Step 3: Take top N chunks
recent_chunks = all_chunks[:limit * 3]  # Get more for clustering, similar to search

# Step 4: Cluster by parent_file (group results by file)
file_groups = defaultdict(list)
for chunk in recent_chunks:
    key = chunk["parent_file"]
    file_groups[key].append(chunk)

# Step 5: Sort each group by chunk_order
for key in file_groups:
    file_groups[key].sort(key=lambda x: x["chunk_order"])

# Step 6: Sort groups by most recent date in each group
sorted_groups = sorted(file_groups.values(),
                       key=lambda g: max(c["date"] for c in g),
                       reverse=True)

# Step 7: Format output - limit to original limit number of files, max 2 chunks per file
output_files = []
output_count = 0
for group in sorted_groups:
    if output_count >= limit:
        break
    
    # Take at most 2 chunks per file (to keep output concise)
    selected_chunks = group[:min(2, len(group))]
    
    # Build file result
    full_path = group[0]["parent_file"]
    rel_path = full_path.replace("/home/archer/Chikusa_MemoRooms/", "")
    date_str = group[0]["date"]
    total_hit = len(selected_chunks)
    total_available = group[0]["total_chunks"]
    
    file_result = {
        "path": rel_path,
        "full_path": full_path,
        "date": date_str,
        "total_chunks": total_available,
        "matched_chunks": total_hit,
        "chunks": []
    }
    
    for chunk in selected_chunks:
        uuid_short = chunk.get('uuid', 'N/A')[:8] if chunk.get('uuid', 'N/A') != 'N/A' else 'N/A'
        
        chunk_data = {
            "chunk_order": chunk['chunk_order'],
            "date": chunk['date'],
            "uuid_short": uuid_short,
            "content": chunk["doc"]
        }
        file_result["chunks"].append(chunk_data)
    
    output_files.append(file_result)
    output_count += len(selected_chunks)

# Output based on format
if output_format == "json":
    # JSON format for Agent consumption
    print(json.dumps({
        "mode": "recent",
        "total_results": len(output_files),
        "files": output_files
    }, indent=2, ensure_ascii=False))
else:
    # Text format (default) for human readability
    print(f"📅 Recent chunks (last {len(output_files)} files)\n")
    
    for file_result in output_files:
        print(f"--- 📄 {file_result['path']} ---")
        print(f"📅 日期：{file_result['date']} | 🧩 命中切片：{file_result['matched_chunks']}/{file_result['total_chunks']}")
        print()
        
        for chunk in file_result["chunks"]:
            print(f"━━━ [切片 {chunk['chunk_order']}/{file_result['total_chunks']}] 日期：{chunk['date']} | ID: {chunk['uuid_short']}... ━━━")
            print(chunk["content"])
            print()
