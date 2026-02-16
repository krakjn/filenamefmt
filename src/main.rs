use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "namefmt")]
#[command(about = "Format filenames according to configuration")]
struct Args {
    /// Path or file to process
    path: Option<PathBuf>,
    /// Actually perform renames (default: dry-run mode)
    #[arg(short, long)]
    inplace: bool,
    /// Override config file location
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Prefix YYYY_MM_DD__ to all filenames
    #[arg(long)]
    timestamp: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Config {
    #[serde(default = "default_replace_spaces")]
    replace_spaces: bool,
 
    #[serde(default)]
    behaviors: Vec<Behavior>,

    #[serde(default)]
    detection: DetectionRules,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Behavior {
    pattern: String,
    style: NamingStyle,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
enum NamingStyle {
    #[serde(rename = "camelCase")]
    CamelCase,
    #[serde(rename = "snake_case")]
    SnakeCase,
    #[serde(rename = "kebab-case")]
    KebabCase,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DetectionRules {
    #[serde(default = "default_exe_extensions")]
    exe_extensions: Vec<String>,

    #[serde(default = "default_package_dirs")]
    package_dirs: Vec<String>,
}

impl Default for DetectionRules {
    fn default() -> Self {
        DetectionRules {
            exe_extensions: default_exe_extensions(),
            package_dirs: default_package_dirs(),
        }
    }
}

fn default_replace_spaces() -> bool {
    true
}

fn default_exe_extensions() -> Vec<String> {
    vec!["exe".to_string(), "bin".to_string(), "app".to_string()]
}

fn default_package_dirs() -> Vec<String> {
    vec!["package.json".to_string(), "Cargo.toml".to_string(), "pyproject.toml".to_string()]
}

impl Default for Config {
    fn default() -> Self {
        Config {
            replace_spaces: true,
            behaviors: Vec::new(),
            detection: DetectionRules {
                exe_extensions: default_exe_extensions(),
                package_dirs: default_package_dirs(),
            },
        }
    }
}

fn get_default_config_toml() -> String {
    r#"replace_spaces = true

[detection]
exe_extensions = ["exe", "bin", "app"]
package_dirs = ["package.json", "Cargo.toml", "pyproject.toml"]
"#.to_string()
}

fn get_config_path(custom_path: Option<&PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = custom_path {
        return Ok(path.clone());
    }
    
    let config_dir = dirs::config_dir()
        .ok_or("Could not determine config directory")?;
    Ok(config_dir.join("namefmt").join("namefmt.toml"))
}

fn load_config(config_path: &Path) -> Config {
    if !config_path.exists() {
        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Warning: Failed to create config directory {}: {}", parent.display(), e);
                eprintln!("Using default configuration");
                return Config::default();
            }
        }
        
        // Write default config
        let default_config = get_default_config_toml();
        if let Err(e) = fs::write(config_path, &default_config) {
            eprintln!("Warning: Failed to write default config to {}: {}", config_path.display(), e);
            eprintln!("Using default configuration");
            return Config::default();
        }
    }
    
    match fs::read_to_string(config_path) {
        Ok(content) => {
            match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", config_path.display(), e);
                    eprintln!("Using default configuration");
                    Config::default()
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to read {}: {}", config_path.display(), e);
            eprintln!("Using default configuration");
            Config::default()
        }
    }
}

fn get_timestamp_prefix() -> String {
    let now = chrono::Utc::now();
    format!("{}__", now.format("%Y_%m_%d"))
}

fn format_filename(name: &str, config: &Config, path: &Path, timestamp: bool) -> Option<String> {
    let mut result = name.to_string();
    
    // Check if this is an exe or package (use kebab-case)
    if is_exe_or_package(path, config) {
        result = to_kebab_case(&result);
    } else {
        // Apply pattern-based behaviors
        for behavior in &config.behaviors {
            if matches_pattern(&result, &behavior.pattern) {
                result = apply_style(&result, &behavior.style);
                break;
            }
        }
        
        // Default: replace spaces with underscores
        if config.replace_spaces {
            result = result.replace(' ', "_");
        }
    }
    
    // Apply timestamp prefix last if requested
    if timestamp {
        let prefix = get_timestamp_prefix();
        result = format!("{}{}", prefix, result);
    }
    
    if result != name {
        Some(result)
    } else {
        None
    }
}

fn is_exe_or_package(path: &Path, config: &Config) -> bool {
    // Check if file has exe extension
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if config.detection.exe_extensions.iter().any(|e| e.to_lowercase() == ext_str) {
            return true;
        }
    }
    
    // Check if directory contains package files
    if path.is_dir() {
        for package_file in &config.detection.package_dirs {
            if path.join(package_file).exists() {
                return true;
            }
        }
    } else if let Some(parent) = path.parent() {
        for package_file in &config.detection.package_dirs {
            if parent.join(package_file).exists() {
                return true;
            }
        }
    }
    
    false
}

fn matches_pattern(name: &str, pattern: &str) -> bool {
    // Simple glob-like pattern matching
    // Supports * for any characters
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            name.starts_with(parts[0]) && name.ends_with(parts[1])
        } else if parts.len() == 1 {
            name.contains(parts[0])
        } else {
            false
        }
    } else {
        name.contains(pattern)
    }
}

fn apply_style(name: &str, style: &NamingStyle) -> String {
    match style {
        NamingStyle::CamelCase => to_camel_case(name),
        NamingStyle::SnakeCase => to_snake_case(name),
        NamingStyle::KebabCase => to_kebab_case(name),
    }
}

fn to_camel_case(s: &str) -> String {
    let words: Vec<&str> = s.split(|c: char| c == ' ' || c == '_' || c == '-').collect();
    let mut result = String::new();
    
    for (i, word) in words.iter().enumerate() {
        if word.is_empty() {
            continue;
        }
        if i == 0 {
            result.push_str(&word.to_lowercase());
        } else {
            let mut chars: Vec<char> = word.chars().collect();
            if !chars.is_empty() {
                chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                result.push_str(&chars.iter().collect::<String>());
            }
        }
    }
    
    result
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch.is_uppercase() {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        } else if ch == ' ' || ch == '-' {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch.is_uppercase() {
            if !result.is_empty() && !result.ends_with('-') {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        } else if ch == ' ' || ch == '_' {
            if !result.is_empty() && !result.ends_with('-') {
                result.push('-');
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

fn process_path(path: &Path, config: &Config, inplace: bool, timestamp: bool) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_file() {
        process_file(path, config, inplace, timestamp)?;
    } else if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                process_file(entry.path(), config, inplace, timestamp)?;
            }
        }
    } else {
        return Err(format!("Path does not exist: {}", path.display()).into());
    }
    
    Ok(())
}

fn process_file(file_path: &Path, config: &Config, inplace: bool, timestamp: bool) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(file_name) = file_path.file_name() {
        let name = file_name.to_string_lossy();
        
        if let Some(new_name) = format_filename(&name, config, file_path, timestamp) {
            let new_path = file_path.parent().unwrap().join(&new_name);
            
            if inplace {
                fs::rename(file_path, &new_path)?;
                println!("Renamed: {} -> {}", file_path.display(), new_path.display());
            } else {
                println!("Would rename: {} -> {}", file_path.display(), new_path.display());
            }
        }
    }
    
    Ok(())
}

fn main() {
    let args = Args::parse();
    
    let config_path = match get_config_path(args.config.as_ref()) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    
    let config = load_config(&config_path);
    
    let target_path = args.path.as_ref()
        .map(|p| p.as_path())
        .unwrap_or_else(|| Path::new("."));
    
    match process_path(target_path, &config, args.inplace, args.timestamp) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
