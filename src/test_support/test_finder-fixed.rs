use std::fs::File;
use std::io::Read;
use std::path::Path;
use syn::{
    Attribute, Expr, ItemFn, ItemMod, Lit,
    visit::{self, Visit},
};

struct TestFinder {
    current_module_path: Vec<String>,
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
        let mod_name = node.ident.to_string();
        self.current_module_path.push(mod_name);

        let mod_has_cfg = has_bench_support_cfg(&node.attrs);
        let previous_gated_context = self.in_gated_context;

        if mod_has_cfg {
            self.in_gated_context = true;
        }

        visit::visit_item_mod(self, node);

        self.in_gated_context = previous_gated_context;
        self.current_module_path.pop();
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let is_test = node.attrs.iter().any(|attr| attr.path().is_ident("test"));

        if is_test {
            let fn_has_cfg = has_bench_support_cfg(&node.attrs);

            if self.in_gated_context || fn_has_cfg {
                let fn_name = node.sig.ident.to_string();
                if self.current_module_path.is_empty() {
                    println!("{}", fn_name);
                } else {
                    println!("{}::{}", self.current_module_path.join("::"), fn_name);
                }
            }
        }

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

fn print_test_functions<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let path_ref = path.as_ref();
    let mut file = File::open(path_ref)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let syntax_tree = syn::parse_file(&content)?;
    let file_is_gated = has_bench_support_cfg(&syntax_tree.attrs);

    // FIX: Seed the finder with the calculated file-level module path layout
    let initial_module_path = determine_file_module_path(path_ref);

    let mut finder = TestFinder {
        current_module_path: initial_module_path,
        in_gated_context: file_is_gated,
    };

    finder.visit_file(&syntax_tree);

    Ok(())
}
