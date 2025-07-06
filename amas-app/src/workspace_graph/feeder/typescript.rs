use crate::file::File;
use crate::workspace_graph::WorkspaceGraph;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

struct ImportVisitor {
    imports: Vec<String>,
    current_file_dir: PathBuf,
}

impl ImportVisitor {
    fn new(current_file_path: &Path) -> Self {
        Self {
            imports: Vec::new(),
            current_file_dir: current_file_path
                .parent()
                .unwrap_or(Path::new(""))
                .to_path_buf(),
        }
    }

    fn resolve_import_path(&self, import_path: &str) -> Option<PathBuf> {
        // Handle relative imports
        if import_path.starts_with('.') {
            let resolved = self.current_file_dir.join(import_path);

            // Canonicalize the path to resolve .. and . components (similar to realpath)
            let canonical_base = match resolved.canonicalize() {
                Ok(path) => path,
                Err(_) => {
                    // If canonicalize fails, try to manually resolve the path
                    self.manual_resolve_path(&resolved)
                }
            };

            // Try different extensions
            for ext in &[".ts", ".tsx", ".js", ".jsx"] {
                let with_ext = canonical_base.with_extension(&ext[1..]);
                if with_ext.exists() {
                    return Some(with_ext);
                }
            }

            // Try index files
            for ext in &[".ts", ".tsx", ".js", ".jsx"] {
                let index_file = canonical_base.join(format!("index{}", ext));
                if index_file.exists() {
                    return Some(index_file);
                }
            }
        }

        // For absolute imports, you might want to resolve them based on your project structure
        // This is a simplified version that doesn't handle node_modules or path mapping
        None
    }

    fn manual_resolve_path(&self, path: &Path) -> PathBuf {
        // Manual path resolution to handle .. and . components
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    // Go up one directory
                    components.pop();
                }
                std::path::Component::CurDir => {
                    // Current directory, skip
                    continue;
                }
                other => {
                    components.push(other);
                }
            }
        }

        // Reconstruct the path
        let mut result = PathBuf::new();
        for component in components {
            result.push(component);
        }

        result
    }

    fn add_import(&mut self, import_path: &str) {
        if let Some(resolved_path) = self.resolve_import_path(import_path) {
            // Canonicalize the final resolved path to ensure consistency
            let canonical_path = resolved_path
                .canonicalize()
                .unwrap_or_else(|_| resolved_path);
            self.imports
                .push(canonical_path.to_string_lossy().to_string());
        }
    }
}

impl<'a> Visit<'a> for ImportVisitor {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let import_path = decl.source.value.as_str();
        self.add_import(import_path);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        let import_path = decl.source.value.as_str();
        self.add_import(import_path);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &decl.source {
            let import_path = source.value.as_str();
            self.add_import(import_path);
        }
    }

    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        // Handle dynamic imports: import("./module")
        if let Expression::ImportExpression(_) = &expr.callee {
            if let Some(first_arg) = expr.arguments.first() {
                if let Argument::StringLiteral(str_lit) = first_arg {
                    let import_path = str_lit.value.as_str();
                    self.add_import(import_path);
                }
            }
        }

        // Handle require calls: require("./module")
        if let Expression::Identifier(ident) = &expr.callee {
            if ident.name == "require" {
                if let Some(first_arg) = expr.arguments.first() {
                    if let Argument::StringLiteral(str_lit) = first_arg {
                        let import_path = str_lit.value.as_str();
                        self.add_import(import_path);
                    }
                }
            }
        }

        // Continue visiting child nodes
        self.visit_expression(&expr.callee);
        for arg in &expr.arguments {
            self.visit_argument(arg);
        }
    }

    fn visit_member_expression(&mut self, expr: &MemberExpression<'a>) {
        // Handle require calls through member expressions: require("./module").default
        if let MemberExpression::StaticMemberExpression(static_expr) = expr {
            if let Expression::CallExpression(call_expr) = &static_expr.object {
                if let Expression::Identifier(ident) = &call_expr.callee {
                    if ident.name == "require" {
                        if let Some(first_arg) = call_expr.arguments.first() {
                            if let Argument::StringLiteral(str_lit) = first_arg {
                                let import_path = str_lit.value.as_str();
                                self.add_import(import_path);
                            }
                        }
                    }
                }
            }
        }

        // Continue visiting child nodes
        match expr {
            MemberExpression::ComputedMemberExpression(computed) => {
                self.visit_expression(&computed.object);
                self.visit_expression(&computed.expression);
            }
            MemberExpression::StaticMemberExpression(static_expr) => {
                self.visit_expression(&static_expr.object);
            }
            MemberExpression::PrivateFieldExpression(private) => {
                self.visit_expression(&private.object);
            }
        }
    }
}

fn parse_typescript_file(
    file_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let source_code = fs::read_to_string(file_path)?;
    let allocator = Allocator::default();

    // Determine source type based on file extension
    let source_type = match file_path.extension().and_then(|s| s.to_str()) {
        Some("tsx") => SourceType::tsx(),
        Some("ts") => SourceType::ts(),
        Some("jsx") => SourceType::jsx(),
        Some("js") => SourceType::unambiguous(),
        Some("mjs") => SourceType::mjs(),
        Some("cjs") => SourceType::cjs(),
        _ => SourceType::default(),
    };

    let ParserReturn {
        program, errors, ..
    } = Parser::new(&allocator, &source_code, source_type).parse();

    if !errors.is_empty() {
        // Log errors but continue processing
        for error in &errors {
            eprintln!("Parse error in {}: {}", file_path.display(), error);
        }
    }

    let mut visitor = ImportVisitor::new(file_path);
    visitor.visit_program(&program);

    Ok(visitor.imports)
}

fn find_typescript_files(project_path: &str) -> Vec<PathBuf> {
    WalkDir::new(project_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| {
                        matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs")
                    })
                    .unwrap_or(false)
        })
        .filter(|entry| {
            // Skip node_modules and other common directories to ignore
            !entry.path().components().any(|component| {
                matches!(
                    component.as_os_str().to_str(),
                    Some("node_modules")
                        | Some(".git")
                        | Some("dist")
                        | Some("build")
                        | Some("coverage")
                )
            })
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

pub fn feed_workspace_graph_with_ts_project(
    graph: &mut WorkspaceGraph,
    project_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = Path::new(project_path);

    // Find all TypeScript/JavaScript files
    let typescript_files = find_typescript_files(project_path.to_str().unwrap());

    // Map to store file paths to node indices
    let mut file_to_node: HashMap<String, NodeIndex> = HashMap::new();

    // First pass: Add all files as nodes
    for file_path in &typescript_files {
        let canonical_path = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let file_path_str = canonical_path.to_string_lossy().to_string();

        // Assuming your File struct has a way to be created from a path
        // You'll need to adjust this based on your File implementation
        let file = File::new(file_path_str.clone()); // Adjust constructor as needed

        let node_index = graph.add_file(file);
        file_to_node.insert(file_path_str, node_index);
    }

    // Second pass: Parse imports and add edges
    for file_path in &typescript_files {
        let canonical_path = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let file_path_str = canonical_path.to_string_lossy().to_string();

        match parse_typescript_file(file_path) {
            Ok(imports) => {
                if let Some(&current_node) = file_to_node.get(&file_path_str) {
                    for import_path in imports {
                        // file_to_node.get(&import_path)
                        if let Some(&imported_node) = file_to_node.get(&import_path)
                        {
                            graph.add_import(current_node, imported_node);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to parse {}: {}", file_path_str, e);
                // Continue with other files even if one fails
            }
        }
    }

    Ok(())
}
