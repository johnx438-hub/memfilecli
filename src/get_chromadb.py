import chromadb
import sys
import json

# Read JSON payload from stdin
payload = json.loads(sys.stdin.read())
db_path = payload["db_path"]
collection_name = payload.get("collection_name", "memfiles")
target_uuid = payload["uuid"]

client = chromadb.PersistentClient(path=db_path)
try:
    collection = client.get_collection(collection_name)
except Exception as e:
    print(f"ERROR|Collection not found|{e}", file=sys.stderr)
    sys.exit(1)

# Query for the specific UUID (support prefix matching)
all_data = collection.get(include=["documents", "metadatas"])

# Find matching chunk by full UUID or prefix
matched_id = None
for item_id in all_data["ids"]:
    if item_id == target_uuid or item_id.startswith(target_uuid):
        matched_id = item_id
        break

if not matched_id:
    print("", end="")  # Empty output means chunk not found
    sys.exit(0)

results = collection.get(ids=[matched_id], include=["documents", "metadatas"])

# Extract data
doc = results["documents"][0]
meta = results["metadatas"][0]

# Parse links from JSON string
raw_links = meta.get("links", "[]")
try:
    links = json.loads(raw_links) if isinstance(raw_links, str) else raw_links
except json.JSONDecodeError:
    links = []

# Build result object (use matched_id for full UUID in output)
result = {
    "id": matched_id,  # Full UUID, not the prefix
    "filename": meta.get("filename", "unknown"),
    "date": meta.get("date", "unknown"),
    "summary": meta.get("summary", ""),
    "content": doc,
    "links": links if isinstance(links, list) else [],
    "chunk_order": meta.get("chunk_order", 0),
    "total_chunks": meta.get("total_chunks", 1)
}

# Output as JSON
print(json.dumps(result, ensure_ascii=False))
