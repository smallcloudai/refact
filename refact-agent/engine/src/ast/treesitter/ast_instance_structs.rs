use async_trait::async_trait;
use dyn_partial_eq::{dyn_partial_eq, DynPartialEq};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cmp::min;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};
use tokio::fs::read_to_string;
use tree_sitter::{Point, Range};
use uuid::Uuid;

use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::structs::{RangeDef, SymbolType};

#[derive(Eq, Hash, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct TypeDef {
    pub name: Option<String>,
    pub inference_info: Option<String>,
    pub inference_info_guid: Option<Uuid>,
    pub is_pod: bool,
    pub namespace: String,
    pub guid: Option<Uuid>,
    pub nested_types: Vec<TypeDef>, // for nested types, presented in templates
}

impl Default for TypeDef {
    fn default() -> Self {
        TypeDef {
            name: None,
            inference_info: None,
            inference_info_guid: None,
            is_pod: false,
            namespace: String::from(""),
            guid: None,
            nested_types: vec![],
        }
    }
}

impl TypeDef {
    pub fn to_string(&self) -> String {
        let mut res = String::from("");
        if let Some(name) = &self.name {
            res.push_str(&name);
        }
        for nested in &self.nested_types {
            res.push_str(&format!("_{}", &nested.to_string()));
        }
        res
    }

    pub fn get_nested_types(&self) -> Vec<TypeDef> {
        let mut types = vec![];
        let mut nested_types = vec![];
        for nested in self.nested_types.iter() {
            types.push(nested.clone());
        }
        for nested in types.iter() {
            nested_types.append(&mut nested.get_nested_types())
        }
        types.append(&mut nested_types);
        types
    }

    pub fn mutate_nested_types<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut TypeDef),
    {
        for nested in &mut self.nested_types {
            f(nested);
            nested.mutate_nested_types_ref(&mut f);
        }
    }

    fn mutate_nested_types_ref<F>(&mut self, f: &mut F)
    where
        F: FnMut(&mut TypeDef),
    {
        for nested in &mut self.nested_types {
            f(nested);
            nested.mutate_nested_types_ref(f);
        }
    }
}


#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct AstSymbolFields {
    pub guid: Uuid,
    pub name: String,
    pub language: LanguageId,
    pub file_path: PathBuf,
    pub namespace: String,
    pub parent_guid: Option<Uuid>,
    pub childs_guid: Vec<Uuid>,
    #[serde(with = "RangeDef")]
    pub full_range: Range,
    #[serde(with = "RangeDef")]
    pub declaration_range: Range,
    #[serde(with = "RangeDef")]
    pub definition_range: Range,
    // extra fields for usage structs to prevent multiple downcast operations
    pub linked_decl_guid: Option<Uuid>,
    pub linked_decl_type: Option<TypeDef>,
    pub caller_guid: Option<Uuid>,
    pub is_error: bool,
    pub caller_depth: Option<usize>,
}

impl AstSymbolFields {
    pub fn from_data(language: LanguageId, file_path: PathBuf, is_error: bool) -> Self {
        AstSymbolFields {
            language,
            file_path,
            is_error,
            ..Default::default()
        }
    }

    pub fn from_fields(fields: &AstSymbolFields) -> Self {
        Self {
            language: fields.language,
            file_path: fields.file_path.clone(),
            is_error: fields.is_error,
            ..Default::default()
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct SymbolInformation {
    pub guid: Uuid,
    pub name: String,
    pub parent_guid: Uuid,
    pub linked_decl_guid: Uuid,
    pub caller_guid: Uuid,
    pub symbol_type: SymbolType,
    pub symbol_path: String,
    pub language: LanguageId,
    pub file_path: PathBuf,
    pub namespace: String,
    pub is_error: bool,
    #[serde(with = "RangeDef")]
    pub full_range: Range,
    #[serde(with = "RangeDef")]
    pub declaration_range: Range,
    #[serde(with = "RangeDef")]
    pub definition_range: Range,
}

impl SymbolInformation {
    pub fn get_content(&self, content: &String) -> io::Result<String> {
        let lines: Vec<&str> = content.split("\n").collect();
        let mut end_row = self.full_range.end_point.row + 1;
        let raw_content = content.get(self.full_range.start_byte..self.full_range.end_byte);
        if raw_content.is_none() {
            return Err(io::Error::other("Incorrect content range"));
        }
        let raw_content = raw_content.unwrap();
        if raw_content.ends_with("\n") {
            end_row -= 1;
        }

        let mut start_row = min(self.full_range.start_point.row, lines.len());
        let end_row = min(end_row, lines.len());
        start_row = min(start_row, end_row);
        let selected_text = lines[start_row..end_row].join("\n");
        Ok(selected_text)
    }

    pub async fn get_content_from_file(&self) -> io::Result<String> {
        let content = read_to_string(&self.file_path).await?;
        self.get_content(&content)
    }

    pub fn get_content_from_file_blocked(&self) -> io::Result<String> {
        let content = fs::read_to_string(&self.file_path)?;
        self.get_content(&content)
    }

    pub fn get_declaration_content(&self, content: &String) -> io::Result<String> {
        let content = content.get(self.declaration_range.start_byte..self.declaration_range.end_byte);
        if content.is_none() {
            return Err(io::Error::other("Incorrect declaration range"));
        }
        Ok(content.unwrap().to_string())
    }

    pub async fn get_declaration_content_from_file(&self) -> io::Result<String> {
        let content = read_to_string(&self.file_path).await?;
        self.get_declaration_content(&content)
    }

    pub fn get_declaration_content_from_file_blocked(&self) -> io::Result<String> {
        let content = fs::read_to_string(&self.file_path)?;
        self.get_declaration_content(&content)
    }
}

impl Default for AstSymbolFields {
    fn default() -> Self {
        AstSymbolFields {
            guid: Uuid::default(),
            name: "".to_string(),
            language: LanguageId::Unknown,
            file_path: PathBuf::new(),
            namespace: "".to_string(),
            parent_guid: None,
            childs_guid: vec![],
            full_range: Range {
                start_byte: 0,
                end_byte: 0,
                start_point: Default::default(),
                end_point: Default::default(),
            },
            declaration_range: Range {
                start_byte: 0,
                end_byte: 0,
                start_point: Default::default(),
                end_point: Default::default(),
            },
            definition_range: Range {
                start_byte: 0,
                end_byte: 0,
                start_point: Default::default(),
                end_point: Default::default(),
            },
            linked_decl_guid: None,
            linked_decl_type: None,
            caller_guid: None,
            is_error: false,
            caller_depth: None,
        }
    }
}


#[async_trait]
#[typetag::serde]
#[dyn_partial_eq]
pub trait AstSymbolInstance: Debug + Send + Sync + Any {
    fn fields(&self) -> &AstSymbolFields;

    fn fields_mut(&mut self) -> &mut AstSymbolFields;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn symbol_info_struct(&self) -> SymbolInformation {
        SymbolInformation {
            guid: self.guid().clone(),
            name: self.name().to_string(),
            parent_guid: self.parent_guid().clone().unwrap_or_default(),
            linked_decl_guid: self.get_linked_decl_guid().clone().unwrap_or_default(),
            caller_guid: self.get_caller_guid().clone().unwrap_or_default(),
            symbol_type: self.symbol_type(),
            symbol_path: "".to_string(),
            language: self.language().clone(),
            file_path: self.file_path().clone(),
            namespace: self.namespace().to_string(),
            is_error: self.is_error(),
            full_range: self.full_range().clone(),
            declaration_range: self.declaration_range().clone(),
            definition_range: self.definition_range().clone(),
        }
    }

    fn guid(&self) -> &Uuid {
        &self.fields().guid
    }

    fn name(&self) -> &str {
        &self.fields().name
    }

    fn language(&self) -> &LanguageId {
        &self.fields().language
    }

    fn file_path(&self) -> &PathBuf { &self.fields().file_path }

    fn is_type(&self) -> bool;

    fn is_declaration(&self) -> bool;

    fn types(&self) -> Vec<TypeDef>;

    fn set_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>);

    fn set_inference_info_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>);

    fn namespace(&self) -> &str {
        &self.fields().namespace
    }

    fn parent_guid(&self) -> &Option<Uuid> {
        &self.fields().parent_guid
    }

    fn childs_guid(&self) -> &Vec<Uuid> {
        &self.fields().childs_guid
    }

    fn symbol_type(&self) -> SymbolType;

    fn full_range(&self) -> &Range {
        &self.fields().full_range
    }

    // ie function signature, class signature, full range otherwise
    fn declaration_range(&self) -> &Range {
        &self.fields().declaration_range
    }

    // ie function body, class body, full range otherwise
    fn definition_range(&self) -> &Range {
        &self.fields().definition_range
    }

    fn get_caller_guid(&self) -> &Option<Uuid> {
        &self.fields().caller_guid
    }

    fn set_caller_guid(&mut self, caller_guid: Uuid) {
        self.fields_mut().caller_guid = Some(caller_guid);
    }

    fn get_linked_decl_guid(&self) -> &Option<Uuid> {
        &self.fields().linked_decl_guid
    }

    fn set_linked_decl_guid(&mut self, linked_decl_guid: Option<Uuid>) {
        self.fields_mut().linked_decl_guid = linked_decl_guid;
    }

    fn get_linked_decl_type(&self) -> &Option<TypeDef> {
        &self.fields().linked_decl_type
    }

    fn set_linked_decl_type(&mut self, linked_decl_type: TypeDef) {
        self.fields_mut().linked_decl_type = Some(linked_decl_type);
    }

    fn temporary_types_cleanup(&mut self);

    fn get_caller_depth(&self) -> &Option<usize> {
        &self.fields().caller_depth
    }

    fn set_caller_depth(&mut self, caller_depth: Option<usize>) {
        self.fields_mut().caller_depth = caller_depth;
    }

    fn is_error(&self) -> bool {
        self.fields().is_error
    }

    fn remove_linked_guids(&mut self, guids: &HashSet<Uuid>) {
        let mut new_guids = vec![];
        for t in self
            .types()
            .iter() {
            if guids.contains(&t.guid.unwrap_or_default()) {
                new_guids.push(None);
            } else {
                new_guids.push(t.guid.clone());
            }
        }
        self.set_guids_to_types(&new_guids);

        match self.get_linked_decl_guid() {
            Some(guid) => {
                if guids.contains(guid) {
                    self.set_linked_decl_guid(None);
                }
            }
            None => {}
        }
    }

    fn distance_to_cursor(&self, cursor: &Point) -> usize {
        cursor.row.abs_diff(self.full_range().start_point.row)
    }
}

// pub type AstSymbolInstanceRc = Rc<RefCell<Box<dyn AstSymbolInstance>>>;
pub type AstSymbolInstanceArc = Arc<RwLock<Box<dyn AstSymbolInstance>>>;


/*
StructDeclaration
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct StructDeclaration {
    pub ast_fields: AstSymbolFields,
    pub template_types: Vec<TypeDef>,
    pub inherited_types: Vec<TypeDef>,
}

impl Default for StructDeclaration {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
            template_types: vec![],
            inherited_types: vec![],
        }
    }
}


#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for StructDeclaration {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn fields_mut(&mut self) -> &mut AstSymbolFields {
        &mut self.ast_fields
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn types(&self) -> Vec<TypeDef> {
        let mut types: Vec<TypeDef> = vec![];
        for t in self.inherited_types.iter() {
            types.push(t.clone());
            types.extend(t.get_nested_types());
        }
        for t in self.template_types.iter() {
            types.push(t.clone());
            types.extend(t.get_nested_types());
        }
        types
    }

    fn set_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        for t in self.inherited_types.iter_mut() {
            t.guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.guid = guids[idx].clone();
                idx += 1;
            })
        }
        for t in self.template_types.iter_mut() {
            t.guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.guid = guids[idx].clone();
                idx += 1;
            })
        }
    }

    fn set_inference_info_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        for t in self.inherited_types.iter_mut() {
            t.inference_info_guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.inference_info_guid = guids[idx].clone();
                idx += 1;
            })
        }
        for t in self.template_types.iter_mut() {
            t.inference_info_guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.inference_info_guid = guids[idx].clone();
                idx += 1;
            })
        }
    }

    fn temporary_types_cleanup(&mut self) {
        for t in self.inherited_types.iter_mut() {
            t.inference_info = None;
            t.mutate_nested_types(|t| {
                t.inference_info = None
            })
        }
        for t in self.template_types.iter_mut() {
            t.inference_info = None;
            t.mutate_nested_types(|t| {
                t.inference_info = None
            })
        }
    }

    fn is_type(&self) -> bool {
        true
    }

    fn is_declaration(&self) -> bool { true }

    fn symbol_type(&self) -> SymbolType {
        SymbolType::StructDeclaration
    }
}


/*
TypeAlias
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct TypeAlias {
    pub ast_fields: AstSymbolFields,
    pub types: Vec<TypeDef>,
}

impl Default for TypeAlias {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
            types: vec![],
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for TypeAlias {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn fields_mut(&mut self) -> &mut AstSymbolFields {
        &mut self.ast_fields
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn types(&self) -> Vec<TypeDef> {
        let mut types: Vec<TypeDef> = vec![];
        for t in self.types.iter() {
            types.push(t.clone());
            types.extend(t.get_nested_types());
        }
        types
    }

    fn set_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        for t in self.types.iter_mut() {
            t.guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.guid = guids[idx].clone();
                idx += 1;
            })
        }
    }

    fn set_inference_info_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        for t in self.types.iter_mut() {
            t.inference_info_guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.inference_info_guid = guids[idx].clone();
                idx += 1;
            })
        }
    }

    fn temporary_types_cleanup(&mut self) {
        for t in self.types.iter_mut() {
            t.inference_info = None;
            t.mutate_nested_types(|t| {
                t.inference_info = None
            })
        }
    }

    fn is_type(&self) -> bool {
        true
    }

    fn is_declaration(&self) -> bool { true }

    fn symbol_type(&self) -> SymbolType {
        SymbolType::TypeAlias
    }
}


/*
ClassFieldDeclaration
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct ClassFieldDeclaration {
    pub ast_fields: AstSymbolFields,
    pub type_: TypeDef,
}

impl Default for ClassFieldDeclaration {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
            type_: TypeDef::default(),
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for ClassFieldDeclaration {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn fields_mut(&mut self) -> &mut AstSymbolFields {
        &mut self.ast_fields
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn types(&self) -> Vec<TypeDef> {
        let mut types: Vec<TypeDef> = vec![];
        types.push(self.type_.clone());
        types.extend(self.type_.get_nested_types());
        types
    }

    fn set_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        self.type_.guid = guids[idx].clone();
        idx += 1;
        self.type_.mutate_nested_types(|t| {
            t.guid = guids[idx].clone();
            idx += 1;
        })
    }

    fn set_inference_info_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        self.type_.inference_info_guid = guids[idx].clone();
        idx += 1;
        self.type_.mutate_nested_types(|t| {
            t.inference_info_guid = guids[idx].clone();
            idx += 1;
        })
    }

    fn temporary_types_cleanup(&mut self) {
        self.type_.inference_info = None;
        self.type_.mutate_nested_types(|t| {
            t.inference_info = None
        })
    }

    fn is_type(&self) -> bool {
        false
    }

    fn is_declaration(&self) -> bool { true }

    fn symbol_type(&self) -> SymbolType {
        SymbolType::ClassFieldDeclaration
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ImportType {
    System,
    Library,
    UserModule,
    Unknown,
}

/*
ImportDeclaration
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct ImportDeclaration {
    pub ast_fields: AstSymbolFields,
    pub path_components: Vec<String>,
    pub alias: Option<String>,
    pub import_type: ImportType,
    pub filepath_ref: Option<PathBuf>,
}

impl Default for ImportDeclaration {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
            path_components: vec![],
            alias: None,
            import_type: ImportType::Unknown,
            filepath_ref: None,
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for ImportDeclaration {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn fields_mut(&mut self) -> &mut AstSymbolFields {
        &mut self.ast_fields
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn types(&self) -> Vec<TypeDef> {
        vec![]
    }

    fn set_guids_to_types(&mut self, _: &Vec<Option<Uuid>>) {}

    fn set_inference_info_guids_to_types(&mut self, _: &Vec<Option<Uuid>>) {}

    fn temporary_types_cleanup(&mut self) {}

    fn is_type(&self) -> bool {
        false
    }

    fn is_declaration(&self) -> bool { false }

    fn symbol_type(&self) -> SymbolType {
        SymbolType::ImportDeclaration
    }
}


/*
VariableDefinition
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct VariableDefinition {
    pub ast_fields: AstSymbolFields,
    pub type_: TypeDef,
}

impl Default for VariableDefinition {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
            type_: TypeDef::default(),
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for VariableDefinition {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn fields_mut(&mut self) -> &mut AstSymbolFields {
        &mut self.ast_fields
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn types(&self) -> Vec<TypeDef> {
        let mut types: Vec<TypeDef> = vec![];
        types.push(self.type_.clone());
        types.extend(self.type_.get_nested_types());
        types
    }

    fn set_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        self.type_.guid = guids[idx].clone();
        idx += 1;
        self.type_.mutate_nested_types(|t| {
            t.guid = guids[idx].clone();
            idx += 1;
        })
    }

    fn set_inference_info_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        self.type_.inference_info_guid = guids[idx].clone();
        idx += 1;
        self.type_.mutate_nested_types(|t| {
            t.inference_info_guid = guids[idx].clone();
            idx += 1;
        })
    }

    fn temporary_types_cleanup(&mut self) {
        self.type_.inference_info = None;
        self.type_.mutate_nested_types(|t| {
            t.inference_info = None
        })
    }

    fn is_type(&self) -> bool {
        false
    }

    fn is_declaration(&self) -> bool { true }

    fn symbol_type(&self) -> SymbolType {
        SymbolType::VariableDefinition
    }
}


/*
FunctionDeclaration
*/
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCaller {
    pub inference_info: String,
    pub guid: Option<Uuid>,
}

#[derive(Eq, Hash, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionArg {
    pub name: String,
    pub type_: Option<TypeDef>,
}

impl Default for FunctionArg {
    fn default() -> Self {
        Self {
            name: String::default(),
            type_: None,
        }
    }
}

#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionDeclaration {
    pub ast_fields: AstSymbolFields,
    pub template_types: Vec<TypeDef>,
    pub args: Vec<FunctionArg>,
    pub return_type: Option<TypeDef>,
}

impl Default for FunctionDeclaration {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
            template_types: vec![],
            args: vec![],
            return_type: None,
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for FunctionDeclaration {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn fields_mut(&mut self) -> &mut AstSymbolFields {
        &mut self.ast_fields
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn is_type(&self) -> bool {
        false
    }

    fn types(&self) -> Vec<TypeDef> {
        let mut types = vec![];
        if let Some(t) = self.return_type.clone() {
            types.push(t.clone());
            types.extend(t.get_nested_types());
        }
        for t in self.args.iter() {
            if let Some(t) = t.type_.clone() {
                types.push(t.clone());
                types.extend(t.get_nested_types());
            }
        }
        types
    }

    fn set_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        if let Some(t) = &mut self.return_type {
            t.guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.guid = guids[idx].clone();
                idx += 1;
            })
        }
        for t in self.args.iter_mut() {
            if let Some(t) = &mut t.type_ {
                t.guid = guids[idx].clone();
                idx += 1;
                t.mutate_nested_types(|t| {
                    t.guid = guids[idx].clone();
                    idx += 1;
                })
            }
        }
    }

    fn set_inference_info_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        if let Some(t) = &mut self.return_type {
            t.inference_info_guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.inference_info_guid = guids[idx].clone();
                idx += 1;
            })
        }
        for t in self.args.iter_mut() {
            if let Some(t) = &mut t.type_ {
                t.inference_info_guid = guids[idx].clone();
                idx += 1;
                t.mutate_nested_types(|t| {
                    t.inference_info_guid = guids[idx].clone();
                    idx += 1;
                })
            }
        }
    }

    fn temporary_types_cleanup(&mut self) {
        if let Some(t) = &mut self.return_type {
            t.inference_info = None;
            t.mutate_nested_types(|t| {
                t.inference_info = None
            });
        }
        for t in self.args.iter_mut() {
            if let Some(t) = &mut t.type_ {
                t.inference_info = None;
                t.mutate_nested_types(|t| {
                    t.inference_info = None
                });
            }
        }
    }

    fn is_declaration(&self) -> bool { true }

    fn symbol_type(&self) -> SymbolType {
        SymbolType::FunctionDeclaration
    }
}


/*
CommentDefinition
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct CommentDefinition {
    pub ast_fields: AstSymbolFields,
}

impl Default for CommentDefinition {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for CommentDefinition {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn fields_mut(&mut self) -> &mut AstSymbolFields {
        &mut self.ast_fields
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn is_type(&self) -> bool {
        false
    }

    fn types(&self) -> Vec<TypeDef> {
        vec![]
    }

    fn set_guids_to_types(&mut self, _: &Vec<Option<Uuid>>) {}

    fn set_inference_info_guids_to_types(&mut self, _: &Vec<Option<Uuid>>) {}

    fn temporary_types_cleanup(&mut self) {}

    fn is_declaration(&self) -> bool { true }

    fn symbol_type(&self) -> SymbolType {
        SymbolType::CommentDefinition
    }
}


/*
FunctionCall
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub ast_fields: AstSymbolFields,
    pub template_types: Vec<TypeDef>,
}

impl Default for FunctionCall {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
            template_types: vec![],
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for FunctionCall {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn fields_mut(&mut self) -> &mut AstSymbolFields {
        &mut self.ast_fields
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn is_type(&self) -> bool {
        false
    }

    fn types(&self) -> Vec<TypeDef> {
        let mut types = vec![];
        if let Some(t) = self.ast_fields.linked_decl_type.clone() {
            types.push(t.clone());
            types.extend(t.get_nested_types());
        }
        for t in self.template_types.iter() {
            types.push(t.clone());
            types.extend(t.get_nested_types());
        }
        types
    }

    fn set_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        if let Some(t) = &mut self.ast_fields.linked_decl_type {
            t.guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.guid = guids[idx].clone();
                idx += 1;
            })
        }
        for t in self.template_types.iter_mut() {
            t.guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.guid = guids[idx].clone();
                idx += 1;
            })
        }
    }

    fn set_inference_info_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        if let Some(t) = &mut self.ast_fields.linked_decl_type {
            t.inference_info_guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.inference_info_guid = guids[idx].clone();
                idx += 1;
            })
        }
        for t in self.template_types.iter_mut() {
            t.inference_info_guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.inference_info_guid = guids[idx].clone();
                idx += 1;
            })
        }
    }

    fn temporary_types_cleanup(&mut self) {
        if let Some(t) = &mut self.ast_fields.linked_decl_type {
            t.inference_info = None;
            t.mutate_nested_types(|t| {
                t.inference_info = None
            });
        }
        for t in self.template_types.iter_mut() {
            t.inference_info = None;
            t.mutate_nested_types(|t| {
                t.inference_info = None
            });
        }
    }

    fn is_declaration(&self) -> bool { false }

    fn symbol_type(&self) -> SymbolType {
        SymbolType::FunctionCall
    }
}


/*
VariableUsage
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct VariableUsage {
    pub ast_fields: AstSymbolFields,
}

impl Default for VariableUsage {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for VariableUsage {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn fields_mut(&mut self) -> &mut AstSymbolFields {
        &mut self.ast_fields
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn is_type(&self) -> bool {
        false
    }

    fn types(&self) -> Vec<TypeDef> {
        let mut types = vec![];
        if let Some(t) = self.ast_fields.linked_decl_type.clone() {
            types.push(t.clone());
            types.extend(t.get_nested_types());
        }
        types
    }

    fn set_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        if let Some(t) = &mut self.ast_fields.linked_decl_type {
            t.guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.guid = guids[idx].clone();
                idx += 1;
            })
        }
    }

    fn set_inference_info_guids_to_types(&mut self, guids: &Vec<Option<Uuid>>) {
        let mut idx = 0;
        if let Some(t) = &mut self.ast_fields.linked_decl_type {
            t.inference_info_guid = guids[idx].clone();
            idx += 1;
            t.mutate_nested_types(|t| {
                t.inference_info_guid = guids[idx].clone();
                idx += 1;
            })
        }
    }

    fn temporary_types_cleanup(&mut self) {
        if let Some(t) = &mut self.ast_fields.linked_decl_type {
            t.inference_info = None;
            t.mutate_nested_types(|t| {
                t.inference_info = None
            });
        }
    }

    fn is_declaration(&self) -> bool { false }

    fn symbol_type(&self) -> SymbolType {
        SymbolType::VariableUsage
    }
}
