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

# Step 1: Get the target chunk to extract its filename/links
target_data = collection.get(ids=[target_uuid], include=["documents", "metadatas"])
if not target_data["ids"]:
    print("[]")  # Empty result if UUID not found
    sys.exit(0)

target_meta = target_data["metadatas"][0]
target_filename = target_meta.get("filename", "")

# Parse links from target chunk (these are filenames, not UUIDs in Phase 1.3)
raw_links = target_meta.get("links", "[]")
try:
    linked_filenames = json.loads(raw_links) if isinstance(raw_links, str) else raw_links
except json.JSONDecodeError:
    linked_filenames = []

if not linked_filenames or not isinstance(linked_filenames, list):
    print("[]")
    sys.exit(0)

# Step 2: Find chunks that match the linked filenames
all_data = collection.get(include=["documents", "metadatas"])

neighbors = []
seen_files = set()  # Deduplicate by filename

for i, meta in enumerate(all_data["metadatas"]):
    chunk_filename = meta.get("filename", "")
    
    # Check if this chunk's filename is in the target's links
    for linked_file in linked_filenames:
        # Support wildcard matching (e.g., "小千的日记_*")
        if linked_file.endswith("*"):
            pattern = linked_file[:-1]
            if chunk_filename.startswith(pattern) and chunk_filename not in seen_files:
                neighbors.append({
                    "id": all_data["ids"][i],
                    "filename": chunk_filename,
                    "summary": meta.get("summary", ""),
                    "relation": "seealso"
                })
                seen_files.add(chunk_filename)
        elif chunk_filename == linked_file and chunk_filename not in seen_files:
            neighbors.append({
                "id": all_data["ids"][i],
                "filename": chunk_filename,
                "summary": meta.get("summary", ""),
                "relation": "seealso"
            })
            seen_files.add(chunk_filename)

# Output as JSON array
print(json.dumps(neighbors, ensure_ascii=False))
