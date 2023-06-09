mod printer;
pub use printer::Printer;

use ast_grep_core::Language;
use ast_grep_language::Tsx;
use node_resolve::Resolver;
use petgraph::{graph::Graph, stable_graph::NodeIndex};
use std::{
    fs,
    io::BufRead,
    path::{Path, PathBuf},
    process::Command,
};

pub struct Walker {
    root: String,
    files: Vec<String>,
    pub graph: Graph<String, u8>,
}

impl Walker {
    pub fn new(root: String) -> Self {
        let output = Command::new("git")
            .current_dir(Path::new(&root))
            .arg("ls-files")
            .output()
            .expect("git list files fail");
        let files: Vec<String> = output.stdout.lines().map(|str| str.unwrap()).collect();

        Walker {
            root,
            files,
            graph: Graph::new(),
        }
    }

    pub fn collect(&mut self, entry: &String, parent_node: Option<NodeIndex>) {
        if !self.files.contains(entry) {
            return;
        }

        println!("[collecting] {entry}");

        let current_node = self
            .graph
            .node_indices()
            .find(|i| self.graph[*i] == entry.to_owned())
            // if not exist, create a new node
            .unwrap_or_else(|| self.graph.add_node(entry.to_owned()));

        match parent_node {
            Some(parent_node) => {
                if self.graph.contains_edge(parent_node, current_node) {
                    return;
                } else {
                    self.graph.add_edge(parent_node, current_node, 1);
                }
            }
            None => (),
        }

        // by extension
        let ext = Path::new(entry)
            .extension()
            .unwrap_or_default() // no extension with ""
            .to_str()
            .unwrap();

        let js_extensions = ["ts", "tsx", "js", "jsx", "json"];
        if !js_extensions.contains(&ext) {
            return;
        }

        let abs_path = Path::new(&self.root).join(entry.to_owned());
        let code = fs::read_to_string(abs_path.to_owned())
            .expect("Should have been able to read the file");

        let sg = Tsx.ast_grep(code);
        sg.root()
            .find_all("import $_ from \"$PATH\"")
            .for_each(|node| {
                let m = node.get_env().get_match("PATH");
                let specifier = m.unwrap().text().to_string();
                // println!("{specifier}");

                // only relative path
                if !specifier.starts_with(".") {
                    return;
                }

                let resolver = Resolver::new()
                    .with_extensions([".ts", ".tsx", ".js", ".jsx", ".json"])
                    .with_main_fields(["source"]) // TODO:
                    .with_basedir(abs_path.parent().unwrap().to_owned());

                let resolved = resolver.resolve(&specifier);

                match resolved {
                    Err(_) => return,
                    Ok(resolved) => {
                        let entry = Path::new(resolved.to_str().unwrap())
                            .strip_prefix(PathBuf::from(&self.root))
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string();
                        self.collect(&entry, Some(current_node));
                    }
                }
            });
    }

    pub fn collect_all(&mut self) {
        self.files
            .clone()
            .iter()
            .for_each(|file| self.collect(&file.to_owned(), None));
    }
}
