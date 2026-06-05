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
all_data = collection.get(include=[])
ids_to_delete = []
for item_id in all_data['ids']:
    for filename in filenames:
        if item_id.startswith(f"{filename}_chunk_"):
            ids_to_delete.append(item_id)
            break

if ids_to_delete:
    collection.delete(ids=ids_to_delete)

# Step 2: Calculate total chunks per file (for clustering metadata)
filename_counts = Counter(filenames)

# Step 3: Add new chunks with enhanced metadata for clustering
new_ids = []
docs = []
metas = []
chunk_order_counter = {}  # Track order within each file

for i, (filename, date_str, doc) in enumerate(zip(filenames, dates, chunks)):
    chunk_id = f"{filename}_chunk_{i}"
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
        "chunk_order": chunk_order_counter[filename]
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

print(f"OK|Added {len(chunks)} chunks (deleted {len(ids_to_delete)} old ones)")
