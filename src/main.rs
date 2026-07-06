use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortAttr {
    Name,
    Modified,
    Created,
    Size,
}

fn parse_sort_attrs(input: &str) -> Result<Vec<SortAttr>, String> {
    let mut attrs = Vec::new();
    for part in input.split(',') {
        match part.trim().to_lowercase().as_str() {
            "name" => attrs.push(SortAttr::Name),
            "modified" => attrs.push(SortAttr::Modified),
            "created" => attrs.push(SortAttr::Created),
            "size" => attrs.push(SortAttr::Size),
            other => return Err(format!("Invalid sort attribute '{}'", other)),
        }
    }
    if attrs.is_empty() {
        return Err("Sort attribute list cannot be empty".to_string());
    }
    Ok(attrs)
}

#[derive(Clone)]
struct AppState {
    base_dir: PathBuf,
    default_sort: std::sync::Arc<Vec<SortAttr>>,
    sort_descending: bool,
}

#[derive(Deserialize)]
struct ListQuery {
    path: Option<String>,
}

#[derive(Deserialize)]
struct DownloadQuery {
    path: String,
}

#[derive(Serialize)]
struct Breadcrumb {
    name: String,
    path: String,
}

#[derive(Serialize)]
struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
    modified: Option<i64>,
    created: Option<i64>,
    path: String,
}


#[derive(Serialize)]
struct DirResponse {
    absolute_base_path: String,
    breadcrumbs: Vec<Breadcrumb>,
    entries: Vec<FileEntry>,
}

enum AppError {
    BadRequest(String),
    NotFound(String),
    Forbidden(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        eprintln!("[ERROR] Status: {}, Msg: {}", status, error_message);

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

/// Securely resolve a requested relative path against the base directory.
/// Ensures no directory traversal escapes base_dir.
fn secure_resolve_path(base_dir: &Path, requested_path: &str) -> Result<PathBuf, AppError> {
    if requested_path.is_empty() {
        return Ok(base_dir.to_path_buf());
    }

    // Decode URL percent encoding
    let decoded = percent_encoding::percent_decode_str(requested_path)
        .decode_utf8()
        .map_err(|_| AppError::BadRequest("Invalid UTF-8 in path".to_string()))?;

    // Standardize directory separation and join base_dir
    let target = base_dir.join(decoded.as_ref());

    // Canonicalize target path to resolve all standard symlinks, relative segments (..)
    let canonical = target.canonicalize().map_err(|e| {
        match e.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound("File or directory not found".to_string()),
            _ => AppError::Internal(format!("IO error: {}", e)),
        }
    })?;

    // Verify target resides within base_dir
    if !canonical.starts_with(base_dir) {
        return Err(AppError::Forbidden("Access denied: path traversal attempt detected".to_string()));
    }

    Ok(canonical)
}

async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

async fn list_files(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<DirResponse>, AppError> {
    let req_path = params.path.unwrap_or_default();
    println!("[API] Listing files for relative path: {:?}", req_path);

    let resolved = secure_resolve_path(&state.base_dir, &req_path)?;

    if !resolved.is_dir() {
        return Err(AppError::BadRequest("Target is not a directory".to_string()));
    }

    let mut entries = Vec::new();
    let mut read_dir = tokio::fs::read_dir(&resolved).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;
        let name = entry.file_name().to_string_lossy().into_owned();

        // Calculate relative path from base_dir for UI navigation
        let rel_path = path
            .strip_prefix(&state.base_dir)
            .map_err(|_| AppError::Internal("Failed to construct relative path".to_string()))?
            .to_string_lossy()
            .into_owned();

        let is_dir = metadata.is_dir();
        let size = metadata.len();
        
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);

        let created = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);

        entries.push(FileEntry {
            name,
            is_dir,
            size,
            modified,
            created,
            path: rel_path,
        });
    }

    // Sort entries: folders first, then respect default_sort, fallback to case-insensitive name
    let default_sort = &state.default_sort;
    let sort_descending = state.sort_descending;
    entries.sort_by(|a, b| {
        // Folders always go first
        if a.is_dir != b.is_dir {
            return if a.is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }

        let mut ordering = std::cmp::Ordering::Equal;
        for attr in default_sort.iter() {
            let attr_ordering = match attr {
                SortAttr::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortAttr::Modified => a.modified.cmp(&b.modified),
                SortAttr::Created => a.created.cmp(&b.created),
                SortAttr::Size => a.size.cmp(&b.size),
            };
            if attr_ordering != std::cmp::Ordering::Equal {
                ordering = attr_ordering;
                break;
            }
        }

        if ordering == std::cmp::Ordering::Equal {
            ordering = a.name.to_lowercase().cmp(&b.name.to_lowercase());
        }

        if sort_descending {
            ordering.reverse()
        } else {
            ordering
        }
    });

    // Generate breadcrumbs relative to base_dir
    let rel_target = resolved
        .strip_prefix(&state.base_dir)
        .map_err(|_| AppError::Internal("Failed to construct relative path".to_string()))?;

    let mut breadcrumbs = Vec::new();
    let mut current_breadcrumb_path = PathBuf::new();

    for component in rel_target.components() {
        let name = component.as_os_str().to_string_lossy().into_owned();
        current_breadcrumb_path.push(component);
        breadcrumbs.push(Breadcrumb {
            name,
            path: current_breadcrumb_path.to_string_lossy().into_owned(),
        });
    }

    Ok(Json(DirResponse {
        absolute_base_path: state.base_dir.to_string_lossy().into_owned(),
        breadcrumbs,
        entries,
    }))
}

async fn download_file(
    State(state): State<AppState>,
    Query(params): Query<DownloadQuery>,
) -> Result<Response, AppError> {
    println!("[API] Downloading file: {:?}", params.path);

    let resolved = secure_resolve_path(&state.base_dir, &params.path)?;

    if resolved.is_dir() {
        return Err(AppError::BadRequest("Cannot download directories".to_string()));
    }

    let file_name = resolved
        .file_name()
        .ok_or_else(|| AppError::Internal("Failed to read file name".to_string()))?
        .to_string_lossy()
        .into_owned();

    let file = tokio::fs::File::open(&resolved).await.map_err(|e| {
        match e.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound("File not found".to_string()),
            _ => AppError::Internal(e.to_string()),
        }
    })?;

    let metadata = file.metadata().await?;
    let file_size = metadata.len();

    let mime = mime_guess::from_path(&resolved).first_or_octet_stream();

    // Stream file contents to browser
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    // Build the download response
    let content_disposition = format!("attachment; filename=\"{}\"", file_name);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CONTENT_DISPOSITION, content_disposition)
        .header(header::CONTENT_LENGTH, file_size)
        .body(body)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse arguments
    let mut custom_port = None;
    let mut sort_attrs = vec![SortAttr::Name];
    let mut sort_descending = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("Web File Hub Server");
                println!();
                println!("Usage: file-server [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -p, --port <PORT>  Specify the port to bind to (default: 8080 with auto-fallback)");
                println!("  -s, --sort <ATTRS> Specify default sort order as comma-separated attributes");
                println!("                     Valid attributes: name, modified, created, size (default: name)");
                println!("  -o, --order <DIR>  Specify sort direction: asc, desc (default: asc)");
                println!("  -h, --help         Print this help message");
                std::process::exit(0);
            }
            "-p" | "--port" => {
                if let Some(val) = args.next() {
                    match val.parse::<u16>() {
                        Ok(p) => custom_port = Some(p),
                        Err(_) => {
                            eprintln!("Error: Port must be a valid number between 1 and 65535.");
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("Error: -p/--port option requires a port number.");
                    std::process::exit(1);
                }
            }
            "-s" | "--sort" => {
                if let Some(val) = args.next() {
                    match parse_sort_attrs(&val) {
                        Ok(attrs) => sort_attrs = attrs,
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("Error: -s/--sort option requires an argument (e.g. name,modified,size).");
                    std::process::exit(1);
                }
            }
            "-o" | "--order" => {
                if let Some(val) = args.next() {
                    match val.trim().to_lowercase().as_str() {
                        "asc" => sort_descending = false,
                        "desc" => sort_descending = true,
                        other => {
                            eprintln!("Error: Invalid sort order '{}'. Valid values are asc, desc.", other);
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("Error: -o/--order option requires an order (asc or desc).");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Error: Unknown argument '{}'", arg);
                eprintln!("Run with -h or --help for usage details.");
                std::process::exit(1);
            }
        }
    }

    // Determine directory binary is executed in
    let base_dir = std::env::current_dir()?
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize current working directory: {}", e))?;

    println!("--------------------------------------------------");
    println!("🚀 Starting Web File Hub Server...");
    println!("📁 Directory: {}", base_dir.display());
    println!("🔀 Default Sort Order: {:?} ({})", sort_attrs, if sort_descending { "descending" } else { "ascending" });
    println!("--------------------------------------------------");

    let state = AppState {
        base_dir,
        default_sort: std::sync::Arc::new(sort_attrs),
        sort_descending,
    };

    // Setup router
    let app = Router::new()
        .route("/", get(index))
        .route("/api/files", get(list_files))
        .route("/api/download", get(download_file))
        .with_state(state);

    // Bind to custom port or search for a free port starting at 8080
    let listener = if let Some(port) = custom_port {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
            format!("Failed to bind to requested port {}: {}", port, e)
        })?
    } else {
        let mut port = 8080;
        loop {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
            match tokio::net::TcpListener::bind(&addr).await {
                Ok(listener) => break listener,
                Err(e) => {
                    if port >= 8100 {
                        return Err(format!("Could not find an available port to bind. Last error: {}", e).into());
                    }
                    println!("Port {} is already in use, trying next...", port);
                    port += 1;
                }
            }
        }
    };

    let local_addr = listener.local_addr()?;
    println!("🌐 Server running at: http://{}", local_addr);
    println!("💡 Connect using your browser to view and download files.");
    println!("🛑 Press Ctrl+C to terminate the server.");
    println!("--------------------------------------------------");

    axum::serve(listener, app).await?;

    Ok(())
}
