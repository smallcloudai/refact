trait AstMockupService {
    /// Searches for symbols by name or path
    fn search_by_name_or_path_base(
        &self,
        declaration_symbols_index: &HashMap<String, Vec<AstSymbolInstanceRc>>,
        usage_symbols_index: &HashMap<String, Vec<AstSymbolInstanceRc>>,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
        try_fuzzy_if_not_found: bool,
        sort_results: bool,
    ) -> Result<Vec<AstSymbolInstanceRc>, String>;

    /// Searches for symbols by name
    fn search_by_name(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
        try_fuzzy_if_not_found: bool,
        sort_results: bool,
    ) -> Result<Vec<AstSymbolInstanceRc>, String>;

    /// Searches for symbols by full path
    fn search_by_fullpath(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
        try_fuzzy_if_not_found: bool,
        sort_results: bool,
    ) -> Result<Vec<AstSymbolInstanceRc>, String>;

    /// Searches for symbols by content
    fn search_by_content(
        &self,
        query: &str,
        request_symbol_type: RequestSymbolType,
        exception_doc: Option<Document>,
        language: Option<LanguageId>,
    ) -> Result<Vec<AstSymbolInstanceRc>, String>;

    /// Searches for related declarations by GUID
    fn search_related_declarations(&self, guid: &Uuid) -> Result<Vec<AstSymbolInstanceRc>, String>;

    /// Searches for usages with a specific declaration
    fn search_usages_with_this_declaration(&self, declaration_guid: &Uuid, exception_doc: Option<Document>) -> Result<Vec<AstSymbolInstanceRc>, String>;

    /// Gets the full path of a symbol
    fn get_symbol_full_path(&self, symbol: &AstSymbolInstanceRc) -> String;

    /// Gets type-related symbols by declaration GUID
    fn get_type_related_symbols(&self, declaration_guid: &Uuid) -> Result<Vec<AstSymbolInstanceRc>, String>;

    /// Gets declarations by parent symbol
    fn get_declarations_by_parent(
        &self,
        symbol: &AstSymbolInstanceRc,
        base_usefulness: f32,
        symbols_by_guid: &HashMap<Uuid, AstSymbolInstanceRc>,
    ) -> (Vec<AstSymbolInstanceRc>, HashMap<Uuid, f32>);

    /// Gets symbols near the cursor and categorizes them into buckets
    fn symbols_near_cursor_to_buckets(
        &self,
        doc: &Document,
        code: &str,
        cursor: Point,
        top_n_near_cursor: usize,
        top_n_usage_for_each_decl: usize,
        fuzzy_search_limit: usize,
    ) -> (
        Vec<AstSymbolInstanceRc>,
        Vec<AstSymbolInstanceRc>,
        Vec<AstSymbolInstanceRc>,
        Vec<AstSymbolInstanceRc>,
        Vec<AstSymbolInstanceRc>,
        HashMap<Uuid, f32>
    );

    /// Gets declaration symbols from imports by file path
    fn decl_symbols_from_imports_by_file_path(&self, doc: &Document, imports_depth: usize) -> Vec<AstSymbolInstanceRc>;

    /// Gets paths from imports by file path
    fn paths_from_imports_by_file_path(&self, doc: &Document, imports_depth: usize) -> Vec<PathBuf>;

    /// Gets declaration symbols from imports
    fn decl_symbols_from_imports(&self, parsed_symbols: &Vec<AstSymbolInstanceRc>, imports_depth: usize) -> Vec<AstSymbolInstanceRc>;

    /// Gets paths from imports
    fn paths_from_imports(&self, parsed_symbols: &Vec<AstSymbolInstanceRc>, imports_depth: usize) -> Vec<PathBuf>;

    /// Gets file markup for a document
    fn file_markup(&self, doc: &Document) -> Result<FileASTMarkup, String>;

    /// Gets symbols by file path
    fn get_by_file_path(&self, request_symbol_type: RequestSymbolType, doc: &Document) -> Result<Vec<SymbolInformation>, String>;

    /// Gets symbols names by type
    fn get_symbols_names(&self, request_symbol_type: RequestSymbolType) -> Vec<String>;

    /// Gets symbols paths by type
    fn get_symbols_paths(&self, request_symbol_type: RequestSymbolType) -> Vec<String>;

    /// Gets symbols by GUID
    fn symbols_by_guid(&self) -> &HashMap<Uuid, AstSymbolInstanceRc>;

    /// Checks if the index needs an update
    fn needs_update(&self) -> bool;

    /// Sets the index as updated
    fn set_updated(&mut self);

    /// Reindexes the symbols
    fn reindex(&mut self);

    /// Gets the total number of files in the index
    fn total_files(&self) -> usize;

    /// Gets the total number of symbols in the index
    fn total_symbols(&self) -> usize;

    /// Checks if the index is overflowed
    fn is_overflowed(&self) -> bool;

    /// Resolves declaration symbols
    fn resolve_declaration_symbols(&self, symbols: &mut Vec<AstSymbolInstanceRc>) -> IndexingStats;

    /// Merges usages to declarations
    fn merge_usages_to_declarations(&self, symbols: &mut Vec<AstSymbolInstanceRc>) -> IndexingStats;

    /// Resolves imports
    fn resolve_imports(
        &self,
        symbols: &mut Vec<AstSymbolInstanceRc>,
        import_components_succ_solution_index: &HashMap<String, ImportDeclaration>,
    ) -> (IndexingStats, HashMap<String, ImportDeclaration>);

    /// Creates extra indexes for symbols
    fn create_extra_indexes(&mut self, symbols: &Vec<AstSymbolInstanceRc>);

    /// Parses a single file and returns symbols
    fn parse_single_file(&self, doc: &Document, code: &str) -> Vec<AstSymbolInstanceRc>;
}

