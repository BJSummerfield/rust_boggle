use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Default, Debug)]
pub struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end_of_word: bool,
    definition: Option<String>,
}
#[derive(Debug)]
pub struct Dictionary {
    root: TrieNode,
}

pub enum SearchResult {
    ValidWord(String), // Holds the definition if it's a valid word
    ValidPrefix,       // Indicates a valid prefix
    NotFound,          // Indicates the key doesn't exist
}

impl Dictionary {
    pub fn new(file_path: &str) -> io::Result<Self> {
        let mut dictionary = Dictionary {
            root: TrieNode::default(),
        };
        dictionary.load_from_file(file_path)?;
        Ok(dictionary)
    }

    pub fn insert(&mut self, word: &str, definition: String) {
        let mut node = &mut self.root;
        for ch in word.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_end_of_word = true;
        node.definition = Some(definition);
    }

    pub fn search(&self, word: &str) -> SearchResult {
        let mut node = &self.root;
        for ch in word.chars() {
            match node.children.get(&ch) {
                Some(n) => node = n,
                None => return SearchResult::NotFound,
            }
        }
        if node.is_end_of_word {
            node.definition
                .as_ref()
                .map(|def| SearchResult::ValidWord(def.clone()))
                .unwrap_or(SearchResult::NotFound)
        } else {
            SearchResult::ValidPrefix
        }
    }

    fn load_from_file(&mut self, file_path: &str) -> io::Result<()> {
        let path = Path::new(file_path);
        let file = File::open(&path)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if let Some((word, definition)) = self.parse_line(&line) {
                self.insert(&word, definition);
            }
        }
        Ok(())
    }

    fn parse_line(&self, line: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() == 2 && parts[0].len() > 2 {
            Some((parts[0].to_lowercase(), parts[1].trim().to_string()))
        } else {
            None
        }
    }
}
