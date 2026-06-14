import chromadb
import sys
import json
from collections import Counter

# Read JSON payload from stdin
payload = json.loads(sys.stdin.read())
db_path = payload["db_path"]
collection_name = payload.get("collection_name", "memfiles")  # Configurable collection name
chunks = payload["chunks"]
filenames = payload["filenames"]
dates = payload["dates"]
embeddings = payload.get("embeddings", None)  # Pre-computed embeddings from Rust (Ollama/OpenAI)
uuids = payload.get("uuids", None)  # Phase 1.1: UUID IDs for stable chunk references
summaries = payload.get("summaries", None)  # Phase 1.2: Summaries for Agent-first context
links = payload.get("links", None)  # Phase 1.3: WikiLinks as raw text arrays

client = chromadb.PersistentClient(path=db_path)

# Create collection if it doesn't exist
try:
    collection = client.get_collection(collection_name)
except Exception:
    # New collection - will use provided embeddings on first write
    collection = client.create_collection(
        name=collection_name,
        metadata={"hnsw:space": "cosine"}  # Default to cosine distance
    )

# Step 1: Delete old chunks for these files (incremental update)
# Phase 1.1: Use metadata filtering instead of ID prefix matching
for filename in set(filenames):
    try:
        collection.delete(where={"parent_file": filename})
    except Exception as e:
        print(f"WARNING|Delete failed for {filename}: {e}", file=sys.stderr)

# Step 2: Calculate total chunks per file (for clustering metadata)
filename_counts = Counter(filenames)

# Step 3: Add new chunks with enhanced metadata for clustering
new_ids = []
docs = []
metas = []
chunk_order_counter = {}  # Track order within each file

for i, (filename, date_str, doc) in enumerate(zip(filenames, dates, chunks)):
    # Phase 1.1: Use UUID as stable ID instead of filename-based ID
    chunk_id = uuids[i] if uuids and i < len(uuids) else f"{filename}_chunk_{i}"
    new_ids.append(chunk_id)
    docs.append(doc)
    
    # Track chunk order (1-based) for this file
    if filename not in chunk_order_counter:
        chunk_order_counter[filename] = 0
    chunk_order_counter[filename] += 1
    
    metas.append({
        "source": filename,
        "filename": filename,
        "date": date_str,
        "type": "memory",
        "chunk_index": i,
        # New clustering metadata
        "parent_file": filename,
        "total_chunks": filename_counts[filename],
        "chunk_order": chunk_order_counter[filename],
        # Phase 1.1: Store UUID in metadata for reference
        "uuid": chunk_id if uuids and i < len(uuids) else None,
        # Phase 1.2: Store summary for Agent-first context
        "summary": summaries[i] if summaries and i < len(summaries) else "",
        # Phase 1.3: Store WikiLinks as JSON string (ChromaDB doesn't support nested arrays in metadata)
        "links": json.dumps(links[i], ensure_ascii=False) if links and i < len(links) else "[]"
    })

if new_ids:
    add_kwargs = {
        "ids": new_ids,
        "documents": docs,
        "metadatas": metas,
    }
    if embeddings is not None:
        # Use pre-computed embeddings from Rust (Ollama/OpenAI)
        add_kwargs["embeddings"] = embeddings
    
    collection.add(**add_kwargs)

# Count deleted chunks by checking metadata before/after (simplified for Phase 1.1)
print(f"OK|Added {len(chunks)} chunks")
