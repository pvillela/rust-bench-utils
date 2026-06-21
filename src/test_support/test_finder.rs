use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use syn::{
    Attribute, Expr, ItemFn, ItemMod, Lit,
    visit::{self, Visit},
};
use walkdir::WalkDir;

struct TestFinder {
    current_module_path: Vec<String>,
    // Tracks if any parent module or the file root satisfies the feature flag
    in_gated_context: bool,
}

/// Helper function to check if an attribute list contains `cfg(feature = "_bench")`
fn has_bench_support_cfg(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }

        let mut is_match = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("feature") {
                let value: Expr = meta.value()?.parse()?;
                if let Expr::Lit(expr_lit) = value {
                    if let Lit::Str(lit_str) = expr_lit.lit {
                        if lit_str.value() == "_bench" {
                            is_match = true;
                        }
                    }
                }
            }
            Ok(())
        });
        is_match
    })
}

impl<'ast> Visit<'ast> for TestFinder {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        // Push the current module name to the chain
        let mod_name = node.ident.to_string();
        self.current_module_path.push(mod_name);

        // Check if this specific module has the required feature gate
        let mod_has_cfg = has_bench_support_cfg(&node.attrs);

        // Save the previous context state to restore it later
        let previous_gated_context = self.in_gated_context;

        // Remain true if an enclosing parent module was already gated
        if mod_has_cfg {
            self.in_gated_context = true;
        }

        // Delegate to walk through nested items (child modules, functions, etc.)
        visit::visit_item_mod(self, node);

        // Restore state as we backtrack up out of this module scope
        self.in_gated_context = previous_gated_context;
        self.current_module_path.pop();
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        // Check if any attribute on the function is named "test"
        let is_test = node.attrs.iter().any(|attr| attr.path().is_ident("test"));

        if is_test {
            let fn_has_cfg = has_bench_support_cfg(&node.attrs);

            // Valid if an enclosing module was gated OR this specific function is gated
            if self.in_gated_context || fn_has_cfg {
                let fn_name = node.sig.ident.to_string();
                if self.current_module_path.is_empty() {
                    println!("{}", fn_name);
                } else {
                    println!("{}::{}", self.current_module_path.join("::"), fn_name);
                }
            }
        }

        // Continue walking inside the function just in case there are nested items
        visit::visit_item_fn(self, node);
    }
}

/// Computes the Rust module path components for a given file path based on Cargo conventions.
fn determine_file_module_path(path: &Path) -> Vec<String> {
    let mut components = Vec::new();

    // Find where the source tree starts (src/, tests/, benches/, examples/)
    let mut parts: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Remove the file extension from the last element (e.g., "crypto.rs" -> "crypto")
    if let Some(last) = parts.last_mut() {
        if let Some(idx) = last.rfind(".rs") {
            *last = &last[..idx];
        }
    }

    // Locate standard root folders and extract the relative sub-paths
    if let Some(root_idx) = parts
        .iter()
        .position(|&p| p == "src" || p == "tests" || p == "benches" || p == "examples")
    {
        let sub_parts = &parts[root_idx + 1..];
        for part in sub_parts {
            // "lib", "main", and "mod" are root file identifiers, not module name scopes
            if *part != "lib" && *part != "main" && *part != "mod" {
                components.push(part.to_string());
            }
        }
    } else {
        // Fallback for isolated files outside a Cargo layout: use the file name
        if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
            if file_name != "lib" && file_name != "main" && file_name != "mod" {
                components.push(file_name.to_string());
            }
        }
    }
    components
}

pub fn print_test_functions<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let path_ref = path.as_ref();
    let mut file = File::open(path_ref)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let syntax_tree = syn::parse_file(&content)?;

    // Check the top-level inner attributes (#![cfg(...)]) of the file itself
    let file_is_gated = has_bench_support_cfg(&syntax_tree.attrs);

    // Seed the finder with the calculated file-level module path layout
    let initial_module_path = determine_file_module_path(path_ref);

    let mut finder = TestFinder {
        current_module_path: initial_module_path,
        in_gated_context: file_is_gated,
    };

    finder.visit_file(&syntax_tree);

    Ok(())
}

pub fn process_directory_tests<P: AsRef<Path>>(dir: P) -> Result<(), Box<dyn std::error::Error>> {
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Only process files that have a ".rs" extension
        if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
            // Call the previously defined print_test_functions function
            if let Err(e) = print_test_functions(path) {
                eprintln!("Error processing file {}: {}", path.display(), e);
            }
        }
    }
    Ok(())
}

pub fn process_directory_tests_std<P: AsRef<Path>>(
    dir: P,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir.as_ref();

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child_path = entry.path();

            if child_path.is_dir() {
                // Recursively call the function for sub-directories
                process_directory_tests_std(&child_path)?;
            } else if child_path.is_file()
                && child_path.extension().map_or(false, |ext| ext == "rs")
            {
                // Call the previously defined print_test_functions function
                if let Err(e) = print_test_functions(&child_path) {
                    eprintln!("Error processing file {}: {}", child_path.display(), e);
                }
            }
        }
    }
    Ok(())
}
