import chromadb
import sys
import json

# Read JSON payload from stdin
payload = json.loads(sys.stdin.read())
db_path = payload["db_path"]
collection_name = payload.get("collection_name", "memfiles")  # Configurable collection name
query_embedding = payload.get("query_embedding", None)  # Pre-computed vector from Rust (Ollama/OpenAI)
query_text = payload.get("query_text", "")               # Fallback text for ChromaDB to embed
limit = payload.get("limit", 5)
threshold = payload.get("threshold", 30.0)

client = chromadb.PersistentClient(path=db_path)
try:
    collection = client.get_collection(collection_name)
except Exception as e:
    print(f"ERROR|Collection not found|{e}", file=sys.stderr)
    sys.exit(1)

# Build query kwargs - use vector if available, otherwise text
query_kwargs = {
    "n_results": limit,
    "include": ["documents", "metadatas", "distances"]
}

if query_embedding is not None:
    # Use pre-computed embedding from Rust (same model as indexing)
    query_kwargs["query_embeddings"] = [query_embedding]
else:
    # Fallback: let ChromaDB embed the text itself (for backward compatibility)
    query_kwargs["query_texts"] = [query_text]

results = collection.query(**query_kwargs)

# Output results with threshold filtering
for i in range(len(results["ids"][0])):
    dist = results["distances"][0][i]
    score = max(0, 1 - dist) * 100
    
    # Apply threshold filter
    if score < threshold:
        break
    
    doc = results["documents"][0][i]
    meta = results["metadatas"][0][i]
    
    print(json.dumps({
        "score": round(score, 1),
        "filename": meta.get("filename", "unknown"),
        "date": meta.get("date", "unknown"),
        "doc": doc[:500]
    }))
