use crate::model::symbol::Symbol;
use crate::search::tokenizer::CodeTokenizer;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexWriter, ReloadPolicy, doc};

/// BM25 search engine backed by tantivy.
pub struct SearchEngine {
    index: Index,
    // Field handles
    f_symbol_id: Field,
    f_name: Field,
    f_qualified_name: Field,
    f_signature: Field,
    f_doc: Field,
    f_kind: Field,
    f_file: Field,
}

#[derive(Debug)]
pub struct SearchResult {
    pub symbol_id: String,
    pub score: f32,
}

impl SearchEngine {
    /// Create a new search engine with the given index directory.
    pub fn create(index_dir: &Path) -> anyhow::Result<Self> {
        let mut schema_builder = Schema::builder();

        let code_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("code")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        let stored_only = TextOptions::default().set_stored();

        let f_symbol_id = schema_builder.add_text_field("symbol_id", STORED);
        let f_name = schema_builder.add_text_field("name", code_options.clone());
        let f_qualified_name =
            schema_builder.add_text_field("qualified_name", code_options.clone());
        let f_signature = schema_builder.add_text_field("signature", code_options.clone());
        let f_doc = schema_builder.add_text_field("doc", code_options);
        let f_kind = schema_builder.add_text_field("kind", stored_only.clone());
        let f_file = schema_builder.add_text_field("file", stored_only);

        let schema = schema_builder.build();

        std::fs::create_dir_all(index_dir)?;
        let index = Index::create_in_dir(index_dir, schema.clone())?;

        // Register custom tokenizer
        index.tokenizers().register("code", CodeTokenizer);

        Ok(Self {
            index,
            f_symbol_id,
            f_name,
            f_qualified_name,
            f_signature,
            f_doc,
            f_kind,
            f_file,
        })
    }

    /// Open an existing search index.
    pub fn open(index_dir: &Path) -> anyhow::Result<Self> {
        let index = Index::open_in_dir(index_dir)?;

        index.tokenizers().register("code", CodeTokenizer);

        let schema = index.schema();
        let f_symbol_id = schema.get_field("symbol_id")?;
        let f_name = schema.get_field("name")?;
        let f_qualified_name = schema.get_field("qualified_name")?;
        let f_signature = schema.get_field("signature")?;
        let f_doc = schema.get_field("doc")?;
        let f_kind = schema.get_field("kind")?;
        let f_file = schema.get_field("file")?;

        Ok(Self {
            index,
            f_symbol_id,
            f_name,
            f_qualified_name,
            f_signature,
            f_doc,
            f_kind,
            f_file,
        })
    }

    /// Index all symbols.
    pub fn index_symbols(&self, symbols: &[&Symbol]) -> anyhow::Result<()> {
        // Dynamic heap: ~500 bytes per symbol, clamped to [15MB, 100MB]
        // (tantivy requires at least 15MB per thread)
        let heap_size = (symbols.len() * 500).clamp(15_000_000, 100_000_000);
        let mut writer: IndexWriter = self.index.writer(heap_size)?;

        // Clear existing index
        writer.delete_all_documents()?;

        for sym in symbols {
            writer.add_document(doc!(
                self.f_symbol_id => sym.id.0.as_str(),
                self.f_name => sym.name.as_str(),
                self.f_qualified_name => sym.qualified_name.as_str(),
                self.f_signature => sym.signature.as_deref().unwrap_or(""),
                self.f_doc => sym.doc_comment.as_deref().unwrap_or(""),
                self.f_kind => sym.kind.as_str(),
                self.f_file => sym.file_path.to_string_lossy().as_ref(),
            ))?;
        }

        writer.commit()?;
        Ok(())
    }

    /// BM25 search.
    pub fn search(&self, query_str: &str, limit: usize) -> anyhow::Result<Vec<SearchResult>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.f_name,
                self.f_qualified_name,
                self.f_signature,
                self.f_doc,
            ],
        );

        let query = query_parser.parse_query(query_str)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(id) = doc.get_first(self.f_symbol_id)
                && let Some(id_str) = id.as_str()
            {
                results.push(SearchResult {
                    symbol_id: id_str.to_string(),
                    score,
                });
            }
        }

        Ok(results)
    }
}
