use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub embedding: EmbeddingConfig,
    pub chunking: ChunkingConfig,
    pub chromadb: ChromaDBConfig,
    pub search: SearchConfig,
    pub scripts: ScriptsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    pub memory_dirs: Vec<String>,
    #[serde(default = "default_db_path")]
    pub db_path: String,
}
fn default_db_path() -> String {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    format!("{}/.memfilecli_db", home.display())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_backend")] pub backend: String,
    pub ollama: OllamaConfig,
    #[serde(default)] pub openai: OpenAIConfig,
}
fn default_backend() -> String { "ollama".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_url")] pub api_url: String,
    #[serde(default = "default_ollama_model")] pub model: String,
}
fn default_ollama_url() -> String { "http://localhost:11434/api/embeddings".to_string() }
fn default_ollama_model() -> String { "qwen3-embedding:8b".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    #[serde(default = "default_openai_url")] pub api_url: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub api_key: Option<String>,
    #[serde(default = "default_openai_model")] pub model: String,
}
fn default_openai_url() -> String { "https://api.openai.com/v1/embeddings".to_string() }
fn default_openai_model() -> String { "text-embedding-3-small".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChromaDBConfig {
    #[serde(default = "default_collection_name")] pub collection_name: String,
    #[serde(default = "default_distance_metric")] pub distance_metric: String,
}
fn default_collection_name() -> String { "memfiles".to_string() }
fn default_distance_metric() -> String { "cosine".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchConfig {
    #[serde(default = "default_search_limit")] pub default_limit: usize,
    #[serde(default = "default_search_threshold")] pub default_threshold: u8,
}
fn default_search_limit() -> usize { 5 }
fn default_search_threshold() -> u8 { 30 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkingConfig {
    #[serde(default = "default_use_markdown")] pub use_markdown_chunking: bool,
    #[serde(default = "default_max_chunk")] pub max_chunk_size: usize,
    #[serde(default = "default_min_chunk")] pub min_chunk_size: usize,
}
fn default_use_markdown() -> bool { true }
fn default_max_chunk() -> usize { 500 }
fn default_min_chunk() -> usize { 50 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScriptsConfig {
    #[serde(default = "default_write_script")]
    pub write_script: String,
    #[serde(default = "default_query_script")]
    pub query_script: String,
}
fn default_write_script() -> String { format!("{}/src/write_chromadb.py", env!("CARGO_MANIFEST_DIR")) }
fn default_query_script() -> String { format!("{}/src/query_chromadb.py", env!("CARGO_MANIFEST_DIR")) }

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Config {
            general: GeneralConfig {
                memory_dirs: vec![format!("{}/memory_vault", home.display())],
                db_path: default_db_path(),
            },
            embedding: EmbeddingConfig { backend: "ollama".to_string(), ollama: OllamaConfig::default(), openai: OpenAIConfig::default() },
            chunking: ChunkingConfig::default(),
            chromadb: ChromaDBConfig { collection_name: default_collection_name(), distance_metric: default_distance_metric() },
            search: SearchConfig { default_limit: default_search_limit(), default_threshold: default_search_threshold() },
            scripts: ScriptsConfig::default(),
        }
    }
}
impl Default for OllamaConfig { fn default() -> Self { OllamaConfig { api_url: default_ollama_url(), model: default_ollama_model() } } }
impl Default for OpenAIConfig { fn default() -> Self { OpenAIConfig { api_url: default_openai_url(), api_key: None, model: default_openai_model() } } }

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("Failed to read config file")?;
            let mut cfg: Config = serde_json::from_str(&content).context("Failed to parse config file")?;
            if let Some(home) = dirs::home_dir() {
                cfg.general.memory_dirs = cfg.general.memory_dirs.into_iter().map(|d| {
                    if Path::new(&d).is_absolute() { d } else { home.join(&d).to_string_lossy().into_owned() }
                }).collect();
            }
            Ok(cfg)
        } else {
            let cfg = Config::default();
            cfg.save()?;
            eprintln!("{} {}", "📝".bright_green(), "Config file created. Edit it at:".bright_yellow());
            eprintln!("   {}", config_path.display().to_string().bright_cyan());
            Ok(cfg)
        }
    }
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() { fs::create_dir_all(parent)?; }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }
    fn config_path() -> PathBuf {
        dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("memfilecli").join("config.json")
    }
}

pub struct Chunker;
impl Chunker {
    pub fn chunk(text: &str, config: &ChunkingConfig) -> Vec<String> {
        if config.use_markdown_chunking && Regex::new(r"^## ").unwrap().is_match(text) {
            Self::chunk_by_heading(text, config.min_chunk_size)
        } else {
            Self::chunk_by_length(text, config.max_chunk_size, config.min_chunk_size)
        }
    }
    fn chunk_by_heading(text: &str, min_len: usize) -> Vec<String> {
        let re = Regex::new(r"(?=^## )").unwrap();
        let parts: Vec<&str> = re.split(text).collect();
        parts.into_iter().map(|p| p.trim()).filter(|p| !p.is_empty() && p.len() >= min_len).map(String::from).collect()
    }
    fn chunk_by_length(text: &str, max_size: usize, min_len: usize) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let total = chars.len();
        if total <= min_len { return vec![text.to_string()]; }
        let mut chunks = Vec::new();
        for i in (0..total).step_by(max_size) {
            let end = std::cmp::min(i + max_size, total);
            let mut actual_end = end;
            while actual_end > i && chars[actual_end - 1] != '\n' && !chars[actual_end - 1].is_whitespace() { actual_end -= 1; }
            if actual_end == i { actual_end = end; }
            let trimmed: String = chars[i..actual_end].iter().collect();
            let t = trimmed.trim();
            if !t.is_empty() && t.len() >= min_len { chunks.push(t.to_string()); }
        }
        chunks
    }
    pub fn enhance(chunk: &str, filename: &str, date_str: &str) -> String {
        format!("[文件名] {}\n[日期] {}\n\n{}", filename, date_str, chunk)
    }
    pub fn extract_date(filename: &str) -> String {
        Regex::new(r"(\d\d\d\d\d\d\d\d)").unwrap().captures(filename).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()).unwrap_or_else(|| "unknown".to_string())
    }
}

pub struct Embedder { config: Config }
#[derive(Debug, Deserialize)] struct OllamaEmbedResponse { embedding: Vec<f32> }
impl Embedder {
    pub fn new(config: Config) -> Self { Embedder { config } }
    pub fn embed(&self, text: &str) -> Result<Option<Vec<f32>>> {
        match self.config.embedding.backend.as_str() {
            "ollama" => self.embed_ollama(text),
            "openai" => self.embed_openai(text),
            _ => Ok(None),
        }
    }
    fn embed_ollama(&self, text: &str) -> Result<Option<Vec<f32>>> {
        let payload = serde_json::json!({"model": self.config.embedding.ollama.model, "prompt": text});
        let json_str = serde_json::to_string(&payload)?;
        let output = Command::new("curl").args(["-s", "--fail-with-body", "-X", "POST", &self.config.embedding.ollama.api_url, "-H", "Content-Type: application/json", "-d", &json_str]).output()?;
        if !output.status.success() { eprintln!("{} Ollama API error", "⚠️".bright_yellow()); return Ok(None); }
        let body = String::from_utf8(output.stdout)?;
        serde_json::from_str::<OllamaEmbedResponse>(&body).map(|r| Some(r.embedding)).or_else(|_| {
            #[derive(Debug, Deserialize)] struct OllamaPromptResponse { embedding: Vec<f32> }
            serde_json::from_str::<OllamaPromptResponse>(&body).map(|r| Some(r.embedding)).or(Ok(None))
        })
    }
    fn embed_openai(&self, text: &str) -> Result<Option<Vec<f32>>> {
        let env_key = std::env::var("MEMFILECLI_OPENAI_API_KEY").ok();
        let api_key = self.config.embedding.openai.api_key.as_deref().or_else(|| env_key.as_deref()).unwrap_or("");
        #[derive(Debug, Deserialize)] struct OpenAIEmbedResponse { data: Vec<OpenAIEmbedData> }
        #[derive(Debug, Deserialize)] struct OpenAIEmbedData { embedding: Vec<f32> }
        let payload = serde_json::json!({"model": self.config.embedding.openai.model, "input": text});
        let json_str = serde_json::to_string(&payload)?;
        let output = Command::new("curl").args(["-s", "--fail-with-body", "-X", "POST", &self.config.embedding.openai.api_url, "-H", &format!("Authorization: Bearer {}", api_key), "-H", "Content-Type: application/json", "-d", &json_str]).output()?;
        if !output.status.success() { eprintln!("{} OpenAI API error", "⚠️".bright_yellow()); return Ok(None); }
        let body = String::from_utf8(output.stdout)?;
        match serde_json::from_str::<OpenAIEmbedResponse>(&body) { Ok(r) => Ok(r.data.first().map(|d| d.embedding.clone())), Err(_) => Ok(None) }
    }
    pub fn check_ollama(&self) -> Result<bool> {
        let output = Command::new("curl").args(["-s", "--fail-with-body", "http://localhost:11434/api/tags"]).output()?;
        if !output.status.success() { return Ok(false); }
        let body = String::from_utf8(output.stdout)?;
        #[derive(Debug, Deserialize)] struct TagsResponse { models: Vec<ModelInfo> }
        #[derive(Debug, Deserialize)] struct ModelInfo { name: String }
        Ok(serde_json::from_str::<TagsResponse>(&body).ok().map(|r| r.models.iter().any(|m| m.name.contains(&self.config.embedding.ollama.model))).unwrap_or(false))
    }
    pub fn list_ollama_models(&self) -> Result<Vec<String>> {
        let output = Command::new("curl").args(["-s", "--fail-with-body", "http://localhost:11434/api/tags"]).output()?;
        if !output.status.success() { return Ok(vec![]); }
        let body = String::from_utf8(output.stdout)?;
        #[derive(Debug, Deserialize)] struct TagsResponse { models: Vec<OllamaModelInfo> }
        #[derive(Debug, Deserialize)] struct OllamaModelInfo { name: String }
                Ok(serde_json::from_str::<TagsResponse>(&body)?.models.into_iter().map(|m| m.name).collect())
    }
}

pub struct IndexManager { db_path: PathBuf }
#[derive(Debug, Serialize, Deserialize)] struct IndexMetaEntry { mtime: f64, chunks: usize, last_indexed: String }
impl IndexManager {
    pub fn new(db_path: &str) -> Self {
        let path = PathBuf::from(db_path);
        if let Some(parent) = path.parent() { fs::create_dir_all(parent).ok(); }
        IndexManager { db_path: path }
    }
    pub fn load_meta(&self) -> Result<HashMap<String, IndexMetaEntry>> {
        let meta_path = self.meta_file_path();
        if meta_path.exists() { Ok(serde_json::from_str(&fs::read_to_string(&meta_path)?)?) } else { Ok(HashMap::new()) }
    }
    pub fn save_meta(&self, meta: &HashMap<String, IndexMetaEntry>) -> Result<()> {
        fs::write(self.meta_file_path(), serde_json::to_string_pretty(meta)?).map_err(Into::into)
    }
    fn meta_file_path(&self) -> PathBuf { self.db_path.join(".index_meta.json") }
    pub fn collect_files(dirs: &[String]) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for dir in dirs {
            let path = Path::new(dir);
            if !path.exists() || !path.is_dir() { continue; }
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() && p.extension().map_or(false, |ext| ext == "md") { files.push(p.to_path_buf()); }
            }
        }
        files.sort(); files
    }
    pub fn file_mtime(path: &Path) -> Result<f64> { Ok(fs::metadata(path)?.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs_f64()) }
}

#[derive(Parser)] #[command(name = "memfilecli")] #[command(about = "MemFileCLI — 结构化记忆检索引擎 🧠")] struct Cli {
    #[command(subcommand)] command: Commands,
}
#[derive(Subcommand)] enum Commands {
    Init, Config,
    Index { #[arg(long)] all: bool, #[arg(long)] dir: Option<String>, #[arg(long)] force_ollama: bool, #[arg(long)] rebuild: bool },
    Search { query: String, #[arg(short, long)] limit: Option<usize>, #[arg(long)] threshold: Option<u8> },
    Stats, Verify, ListFiles,
}

fn read_line() -> Result<String> { let mut input = String::new(); std::io::stdin().read_line(&mut input)?; Ok(input) }

pub struct IndexArgs { pub all: bool, pub dir: Option<String>, pub force_ollama: bool, pub rebuild: bool }
pub struct SearchArgs { pub query: String, pub limit: Option<usize>, pub threshold: Option<u8> }

fn cmd_search(args: &SearchArgs) -> Result<()> {
    let config = Config::load()?;
    let embedder = Embedder::new(config.clone());
    
    // Use CLI args if provided, otherwise fall back to config defaults
    let limit = args.limit.unwrap_or(config.search.default_limit);
    let threshold = args.threshold.unwrap_or(config.search.default_threshold);
    
    println!("{} Searching for: '{}'", "🎯".bright_green(), args.query.bright_cyan());
    println!();
    
    // Generate query embedding via configured backend (Ollama/OpenAI)
    println!("{} Generating query vector...", "🚀".bright_green());
    let query_embedding = match embedder.embed(&args.query)? {
        Some(embedding) => embedding.into_iter().map(|f| f as f64).collect::<Vec<f64>>(),
        None => { eprintln!("{} Failed to generate embedding", "❌".bright_red()); return Ok(()); }
    };
    
    println!("{} Querying ChromaDB (limit: {}, threshold: {}%)...", "🔍".bright_green(), limit, threshold);
    
    // Send query via stdin JSON payload with threshold filtering
    let payload = serde_json::json!({
        "db_path": config.general.db_path,
        "collection_name": config.chromadb.collection_name,
        "query_text": args.query,
        "query_embedding": query_embedding,
        "limit": limit,
        "threshold": threshold as f64
    });
    
    let mut child = Command::new("python3")
        .arg(&config.scripts.query_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(payload.to_string().as_bytes())?;
    }
    
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{} ChromaDB query failed: {}", "❌".bright_red(), stderr.lines().next().unwrap_or("unknown"));
        return Ok(());
    }
    
    let stdout = String::from_utf8(output.stdout)?;
    if stdout.trim().is_empty() {
        println!("{} No related memories found (below threshold).", "🔍".bright_blue());
        return Ok(());
    }
    
    #[derive(serde::Deserialize)] struct SearchResult { score: f64, filename: String, date: String, doc: String }
    let results: Vec<SearchResult> = stdout.lines()
        .filter_map(|line| serde_json::from_str::<SearchResult>(line).ok())
        .collect();
    
    for (i, result) in results.iter().enumerate() {
        println!("--- [{}] 匹配度: {:.1}% ---", i + 1, result.score);
        println!("📄 文件: {}", result.filename.bright_cyan());
        println!("📅 日期: {}", result.date.bright_yellow());
        let preview: String = result.doc.chars().take(300).collect();
        let preview = if result.doc.chars().count() > 300 { format!("{}...", preview) } else { preview };
        println!("📝 内容: {}", preview);
        println!();
    }
    
    Ok(())
}

fn query_chromadb(db_path: &str, query: &str, limit: usize, script_path: &str) -> Result<Vec<(String, String, String, f64)>> {
    let script = PathBuf::from(script_path);
    let output = Command::new("python3").args([&script.to_string_lossy().to_string(), db_path, query, &limit.to_string()]).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("ChromaDB query failed: {}", stderr.lines().next().unwrap_or("unknown")));
    }
    let stdout = String::from_utf8(output.stdout)?;
    #[derive(serde::Deserialize)] struct SearchResult { score: f64, filename: String, date: String, doc: String }
    let mut results: Vec<(String, String, String, f64)> = stdout.lines()
        .filter_map(|line| serde_json::from_str::<SearchResult>(line).ok())
        .map(|e| (e.doc, e.filename, e.date, e.score))
        .collect();
    results.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    Ok(results)
}

fn cmd_init() -> Result<()> {
    println!("{}", "🔧 MemFileCLI Configuration Wizard".bright_cyan().bold());
    let mut config = Config::default();
    print!("{} Enter memory dirs (comma-separated, or Enter for default): ", "📁".bright_green());
    let input = read_line()?;
    if !input.trim().is_empty() { config.general.memory_dirs = input.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(); }
    println!("\n{} Select embedding backend:", "🧠".bright_green());
    println!("   1) Ollama (local, free)\n   2) OpenAI Compatible API\n   3) ChromaDB built-in");
    print!("   Choice [1]: ");
    let input = read_line()?;
    if input.trim() == "2" { config.embedding.backend = "openai".to_string(); }
    if config.embedding.backend == "ollama" {
        print!("\n{} Checking Ollama for available models... ", "🔍".bright_green());
        let embedder = Embedder::new(config.clone());
        match embedder.list_ollama_models() {
            Ok(models) => {
                if models.is_empty() { println!("{}", "(none found)".bright_yellow()); }
                else {
                    println!("{} models found", models.len().to_string().bright_green());
                    for (i, model) in models.iter().enumerate() { println!("   {}. {}{}", i+1, model.bright_cyan(), if model.contains("embedding") { " ← recommended" } else { "" }); }
                    print!("\n{} Enter model number (or Enter for default): ", "🎯".bright_green());
                    let input = read_line()?;
                    if !input.trim().is_empty() { if let Some(num) = input.trim().parse::<usize>().ok() { if num > 0 && num <= models.len() { config.embedding.ollama.model = models[num-1].clone(); } } }
                }
            } Err(e) => println!("   Error: {}", e),
        }
    }
    config.save()?;
    println!("\n{} Configuration saved!", "✅".bright_green());
    Ok(())
}

fn cmd_config() -> Result<()> {
    let config = Config::load()?;
    println!("{}", "📋 Current Configuration".bright_cyan().bold());
    println!("{} General:", "📁".bright_green());
    for dir in &config.general.memory_dirs { println!("   {}", dir.bright_yellow()); }
    println!("   DB Path: {}", config.general.db_path.bright_yellow());
    println!("\n{} Embedding: Backend={}", "🧠".bright_green(), config.embedding.backend.bright_cyan());
    if config.embedding.backend == "ollama" { println!("   Model: {} URL: {}", config.embedding.ollama.model.bright_yellow(), config.embedding.ollama.api_url); }
    else if config.embedding.backend == "openai" { println!("   Model: {} URL: {}", config.embedding.openai.model.bright_yellow(), config.embedding.openai.api_url); }
    println!("\n{} ChromaDB:", "💾".bright_magenta());
    println!("   Collection: {}", config.chromadb.collection_name.bright_cyan());
    println!("   Distance: {}", config.chromadb.distance_metric.bright_cyan());
    println!("\n{} Search Defaults:", "🔍".bright_blue());
    println!("   Limit: {} Threshold: {}%", config.search.default_limit, config.search.default_threshold);
    Ok(())
}

fn write_chunks_to_chromadb(db_path: &str, script_path: &str, collection_name: &str, filenames: &[String], dates: &[String], docs: &[String], embeddings: Option<Vec<Vec<f64>>>) -> Result<()> {
    let payload = serde_json::json!({
        "db_path": db_path,
        "collection_name": collection_name,
        "chunks": docs,
        "filenames": filenames,
        "dates": dates,
        "embeddings": embeddings
    });
    
    let mut child = Command::new("python3")
        .arg(script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(payload.to_string().as_bytes())?;
    }
    
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("ChromaDB write error: {}", stderr.lines().next().unwrap_or("unknown")));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("{} {}", "📦".bright_blue(), stdout.trim());
    Ok(())
}

fn cmd_index(args: &IndexArgs) -> Result<()> {
    let config = Config::load()?;
    let embedder = Embedder::new(Config { embedding: config.embedding.clone(), general: Default::default(), chunking: Default::default(), chromadb: Default::default(), search: Default::default(), scripts: Default::default() });
    let index_mgr = IndexManager::new(&config.general.db_path);
    let dirs = if let Some(ref dir) = args.dir { vec![dir.clone()] } else { config.general.memory_dirs.clone() };
    let use_ollama = args.force_ollama || embedder.check_ollama()?;
    if use_ollama { println!("{} Detected Ollama, using {} for high-precision indexing...", "🚀".bright_green(), config.embedding.ollama.model.bright_cyan()); }
    else { println!("{} No Ollama detected, using ChromaDB built-in embedding.", "💡".bright_yellow()); }
    let files = IndexManager::collect_files(&dirs);
    println!("{} Scanning {} files...", "🔍".bright_green(), files.len().to_string().bright_cyan());
    
    let mut meta = if args.rebuild { HashMap::new() } else { index_mgr.load_meta()? };
    let indicator = indicatif::ProgressBar::new(files.len() as u64);
    indicator.set_prefix("📊"); indicator.enable_steady_tick(std::time::Duration::from_millis(100));
    let mut added_count = 0; let mut skipped_count = 0; let mut deleted_count = 0;
    let current_files: std::collections::HashSet<String> = files.iter().map(|f| f.to_string_lossy().into_owned()).collect();
    
    for file_path in &files {
        indicator.inc(1);
        let file_str = file_path.to_string_lossy().to_string();
        
        // Check if file needs updating (incremental)
        if !args.rebuild {
            if let Some(entry) = meta.get(&file_str) {
                if let Ok(mtime) = IndexManager::file_mtime(file_path) {
                    if (mtime - entry.mtime).abs() < 1.0 { skipped_count += 1; continue; }
                }
            }
        }
        
        // Read file content
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => { eprintln!("{} {}: {}", "⚠️".bright_yellow(), file_path.display(), e); continue; }
        };
        
        let filename = file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "unknown.md".to_string());
        let date_str = Chunker::extract_date(&filename);
        let chunks = Chunker::chunk(&content, &config.chunking);
        
        // Delete old chunks from ChromaDB if file was previously indexed
        if meta.contains_key(&file_str) { deleted_count += meta[&file_str].chunks; }
        
        // Process each chunk: generate embedding and collect for ChromaDB insertion
        let mut new_filenames = Vec::new();
        let mut new_dates = Vec::new();
        let mut new_docs = Vec::new();
        let mut new_embeddings: Option<Vec<Vec<f64>>> = if use_ollama && config.embedding.backend == "ollama" { Some(Vec::new()) } else { None };
        
        for chunk in &chunks {
            let enhanced = Chunker::enhance(chunk, &filename, &date_str);
            
            if let Some(ref mut embeddings) = new_embeddings {
                match embedder.embed(&enhanced) {
                    Ok(Some(embedding)) => { 
                        embeddings.push(embedding.into_iter().map(|f| f as f64).collect::<Vec<_>>()); 
                        added_count += 1; 
                    }
                    Ok(None) => skipped_count += 1,
                    Err(e) => eprintln!("{} Embedding failed for {}: {}", "⚠️".bright_yellow(), filename, e),
                }
            } else {
                // ChromaDB built-in embedding (no external API needed)
                added_count += 1;
            }
            
            new_filenames.push(filename.clone());
            new_dates.push(date_str.clone());
            new_docs.push(enhanced);
        }
        
        // Write chunks to ChromaDB via Python script with embeddings
        if !new_docs.is_empty() {
            write_chunks_to_chromadb(&config.general.db_path, &config.scripts.write_script, 
                &config.chromadb.collection_name,
                &new_filenames, &new_dates, &new_docs, new_embeddings)?;
        }
        
        // Update metadata
        meta.insert(file_str, IndexMetaEntry { mtime: IndexManager::file_mtime(file_path).unwrap_or(0.0), chunks: new_docs.len(), last_indexed: chrono::Local::now().to_rfc3339() });
    }
    
    indicator.finish_with_message("✅");
    
    // Clean up deleted files from metadata and ChromaDB
    let stale_keys: Vec<String> = meta.keys().filter(|k| !current_files.contains(*k)).cloned().collect();
    for file_str in &stale_keys { deleted_count += meta[file_str].chunks; meta.remove(file_str); }
    
    index_mgr.save_meta(&meta)?;
    
    println!("\n{} Index Summary", "📊".bright_cyan());
    println!("   {} Added: {}", "✅".bright_green(), added_count.to_string().bright_cyan());
    println!("   ⏭️  Skipped: {}", skipped_count.to_string().bright_yellow());
    println!("   🗑️  Cleaned: {}", deleted_count.to_string().bright_red());
    
    Ok(())
}

fn cmd_stats() -> Result<()> {
    let config = Config::load()?;
    let index_mgr = IndexManager::new(&config.general.db_path);
    match index_mgr.load_meta() {
        Ok(meta) => {
            if meta.is_empty() { println!("{} Index is empty. Run 'memfilecli index --all' first.", "📭".bright_yellow()); return Ok(()); }
            let total_chunks: usize = meta.values().map(|m| m.chunks).sum();
            println!("{}", "📊 Memory Statistics".bright_cyan().bold());
            println!("   📁 Total indexed files: {}", meta.len().to_string().bright_cyan());
            println!("   📄 Total chunks: {}", total_chunks.to_string().bright_cyan());
            let mut dir_counts: HashMap<String, usize> = HashMap::new();
            for (file_path, _) in &meta { if let Some(dir) = Path::new(file_path).parent() { *dir_counts.entry(dir.display().to_string()).or_insert(0) += 1; } }
            println!("\n{} By directory:", "📂".bright_green());
            for (dir, count) in &dir_counts { println!("   {}: {} files", dir.bright_yellow(), count.to_string().bright_cyan()); }
        } Err(_) => { println!("{} No index found.", "📭".bright_yellow()); }
    }
    Ok(())
}

fn cmd_verify() -> Result<()> {
    let config = Config::load()?;
    let embedder = Embedder::new(config.clone());
    println!("{}", "🔍 Verification Check".bright_cyan().bold());
    match config.embedding.backend.as_str() {
        "ollama" => {
            print!("{} Ollama connection... ", "🧠".bright_green());
            match embedder.check_ollama() { Ok(true) => println!("{}", "✓ Connected".bright_green()), _ => println!("{}", "✗ Not available".bright_red()) }
            print!("{} Available models... ", "📋".bright_green());
            match embedder.list_ollama_models() { Ok(models) => if models.is_empty() { println!("{}", "(none)".bright_yellow()); } else { println!("{} found", models.len().to_string().bright_green()); for m in &models { println!("   - {}", m); } } Err(e) => println!("Error: {}", e) }
        } _ => println!("{} ChromaDB built-in", "💡".bright_blue()),
    }
    Ok(())
}

fn cmd_list_files() -> Result<()> {
    let config = Config::load()?;
    let index_mgr = IndexManager::new(&config.general.db_path);
    match index_mgr.load_meta() {
        Ok(meta) => {
            if meta.is_empty() { println!("{} No indexed files.", "📭".bright_yellow()); return Ok(()); }
            println!("{}", "📄 Indexed Files".bright_cyan().bold());
            for (file_path, entry) in &meta {
                let filename = Path::new(file_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "unknown.md".to_string());
                println!("   {} [{}]  ({} chunks)", filename.bright_cyan(), Chunker::extract_date(&filename), entry.chunks.to_string().bright_yellow());
            }
        } Err(_) => { println!("{} No index found.", "📭".bright_yellow()); }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => cmd_init(),
        Commands::Config => cmd_config(),
        Commands::Index { all: _, dir, force_ollama, rebuild } => cmd_index(&IndexArgs { all: true, dir, force_ollama, rebuild }),
        Commands::Search { query, limit, threshold } => cmd_search(&SearchArgs { query, limit, threshold }),
        Commands::Stats => cmd_stats(),
        Commands::Verify => cmd_verify(),
        Commands::ListFiles => cmd_list_files(),
    }
}
