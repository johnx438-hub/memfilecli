import chromadb
import sys
import json
from collections import defaultdict

# Read JSON payload from stdin
payload = json.loads(sys.stdin.read())
db_path = payload["db_path"]
collection_name = payload.get("collection_name", "memfiles")  # Configurable collection name
query_embedding = payload.get("query_embedding", None)  # Pre-computed vector from Rust (Ollama/OpenAI)
query_text = payload.get("query_text", "")               # Fallback text for ChromaDB to embed
limit = payload.get("limit", 5)
threshold = payload.get("threshold", 30.0)
date_after = payload.get("date_after")
date_before = payload.get("date_before")

client = chromadb.PersistentClient(path=db_path)
try:
    collection = client.get_collection(collection_name)
except Exception as e:
    print(f"ERROR|Collection not found|{e}", file=sys.stderr)
    sys.exit(1)

# Build query kwargs - use vector if available, otherwise text
query_kwargs = {
    "n_results": limit * 3,  # Get more candidates for clustering
    "include": ["documents", "metadatas", "distances"]
}

if query_embedding is not None:
    # Use pre-computed embedding from Rust (same model as indexing)
    query_kwargs["query_embeddings"] = [query_embedding]
else:
    # Fallback: let ChromaDB embed the text itself (for backward compatibility)
    query_kwargs["query_texts"] = [query_text]

results = collection.query(**query_kwargs)

# Step 1: Collect all results with threshold and date filtering
raw_results = []
for i in range(len(results["ids"][0])):
    dist = results["distances"][0][i]
    score = max(0, 1 - dist) * 100
    
    # Apply threshold filter
    if score < threshold:
        break
    
    doc = results["documents"][0][i]
    meta = results["metadatas"][0][i]
    date_str = meta.get("date", "unknown")
    
    # Post-filter by date range (ChromaDB $gte/$lte only works on numeric types)
    if date_after and date_str < date_after:
        continue
    if date_before and date_str > date_before:
        continue
    
    raw_results.append({
        "score": round(score, 1),
        "filename": meta.get("filename", "unknown"),
        "date": date_str,
        "doc": doc[:500],
        "chunk_index": meta.get("chunk_index", 0),
        "parent_file": meta.get("parent_file", meta.get("filename", "unknown")),
        "total_chunks": meta.get("total_chunks", 1),
        "chunk_order": meta.get("chunk_order", 0),
        # Phase 2: Include UUID for Agent reference
        "uuid": meta.get("uuid", "N/A")
    })

# Step 2: Cluster by parent_file (group results by file)
file_groups = defaultdict(list)
for result in raw_results:
    key = result["parent_file"]
    file_groups[key].append(result)

# Step 3: Sort each group by chunk_order (1-based sequence within file)
for key in file_groups:
    file_groups[key].sort(key=lambda x: x["chunk_order"])

# Step 4: Sort groups by highest score in each group (best matching files first)
sorted_groups = sorted(file_groups.values(), 
                       key=lambda g: max(r["score"] for r in g), 
                       reverse=True)

# Step 5: Format output - limit to original limit number of files, max 2 chunks per file
output_count = 0
for group in sorted_groups:
    if output_count >= limit:
        break
    
    # Take at most 2 chunks per file (to keep output concise)
    selected_chunks = group[:min(2, len(group))]
    
    # Print file header
    filename = group[0]["parent_file"]
    date_str = group[0]["date"]
    total_hit = len(selected_chunks)
    total_available = group[0]["total_chunks"]
    
    print(f"--- 📄 {filename} ---")
    print(f"📅 日期：{date_str} | 🧩 命中切片：{total_hit}/{total_available}")
    print()
    
    for chunk in selected_chunks:
        uuid_short = chunk.get('uuid', 'N/A')[:8] if chunk.get('uuid', 'N/A') != 'N/A' else 'N/A'
        print(f"━━━ [切片 {chunk['chunk_order']}/{total_available}] 匹配度：{chunk['score']}% | ID: {uuid_short}... ━━━")
        print(chunk["doc"])
        print()
    
    output_count += len(selected_chunks)
