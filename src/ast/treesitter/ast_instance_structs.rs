use std::fmt::Debug;

use async_trait::async_trait;
use dyn_partial_eq::{dyn_partial_eq, DynPartialEq};
use serde::{Deserialize, Serialize};
use tree_sitter::Range;
use url::Url;
use crate::ast::treesitter::structs::RangeDef;

#[derive(Eq, Hash, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct TypeDef {
    pub name: Option<String>,
    pub inference_info: Option<String>,
    pub is_pod: bool,
    pub namespace: String,
    // it's filled if we don't know the type name yet
    pub guid: Option<String>,
    // it's filled in a separate stage
    pub nested_types: Vec<TypeDef>, // for nested types, presented in templates
}

impl Default for TypeDef {
    fn default() -> Self {
        TypeDef {
            name: None,
            inference_info: None,
            is_pod: false,
            namespace: String::from(""),
            guid: None,
            nested_types: vec![],
        }
    }
}

impl TypeDef {
    fn from_name(name: &str, is_pod: bool) -> TypeDef {
        TypeDef {
            name: Some(name.to_string()),
            inference_info: None,
            is_pod: is_pod,
            namespace: "".to_string(),
            guid: None,
            nested_types: vec![],
        }
    }

    fn from_inference_info(info: &str) -> TypeDef {
        TypeDef {
            name: None,
            inference_info: Some(info.to_string()),
            is_pod: false,
            namespace: "".to_string(),
            guid: None,
            nested_types: vec![],
        }
    }

    fn from_name_and_inference_info(
        name: &str, is_pod: bool, info: &str,
    ) -> TypeDef {
        TypeDef {
            name: Some(name.to_string()),
            inference_info: Some(info.to_string()),
            is_pod: is_pod,
            namespace: "".to_string(),
            guid: None,
            nested_types: vec![],
        }
    }

    fn set_guid(&mut self, guid: String) {
        self.guid = Some(guid);
    }

    fn add_nested_types(&mut self, types: Vec<TypeDef>) {
        self.nested_types.extend(types)
    }

    fn is_pod(&self) -> bool { self.is_pod }
    
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
}


#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct AstSymbolFields {
    pub guid: String,
    pub name: String,
    pub file_url: Url,
    pub content_hash: String,
    pub namespace: String,
    pub parent_guid: Option<String>,
    pub childs_guid: Vec<String>,
    #[serde(with = "RangeDef")]
    pub full_range: Range,
    #[serde(with = "RangeDef")]
    pub declaration_range: Range,
    #[serde(with = "RangeDef")]
    pub definition_range: Range,
}

impl Default for AstSymbolFields {
    fn default() -> Self {
        AstSymbolFields {
            guid: "".to_string(),
            name: "".to_string(),
            file_url: Url::parse("file:///").unwrap(),
            content_hash: "".to_string(),
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
        }
    }
}


#[async_trait]
#[typetag::serde]
#[dyn_partial_eq]
pub trait AstSymbolInstance: Debug + Send + Sync {
    fn fields(&self) -> &AstSymbolFields;

    fn guid(&self) -> &str {
        &self.fields().guid
    }

    fn name(&self) -> &str {
        &self.fields().name
    }

    fn file_url(&self) -> Url {
        self.fields().file_url.clone()
    }

    fn content_hash(&self) -> &str {
        &self.fields().content_hash
    }

    fn is_type(&self) -> bool;

    fn type_names(&self) -> Vec<TypeDef>;

    fn namespace(&self) -> &str {
        &self.fields().namespace
    }

    fn parent_guid(&self) -> Option<String> {
        self.fields().parent_guid.clone()
    }

    fn childs_guid(&self) -> Vec<String> {
        self.fields().childs_guid.clone()
    }

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
}


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

    fn type_names(&self) -> Vec<TypeDef> {
        let mut types = self.inherited_types.clone();
        types.extend(self.template_types.clone());
        types
    }

    fn is_type(&self) -> bool {
        true
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

    fn type_names(&self) -> Vec<TypeDef> {
        self.types.clone()
    }

    fn is_type(&self) -> bool {
        true
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

    fn type_names(&self) -> Vec<TypeDef> {
        vec![self.type_.clone()]
    }

    fn is_type(&self) -> bool {
        false
    }
}


/*
ImportDeclaration
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct ImportDeclaration {
    pub ast_fields: AstSymbolFields,
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for ImportDeclaration {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn type_names(&self) -> Vec<TypeDef> {
        vec![]
    }

    fn is_type(&self) -> bool {
        false
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

    fn type_names(&self) -> Vec<TypeDef> {
        vec![self.type_.clone()]
    }

    fn is_type(&self) -> bool {
        false
    }
}


/*
FunctionDeclaration
*/
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCaller {
    pub inference_info: String,
    pub guid: Option<String>,
}

#[derive(Eq, Hash, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionArg {
    pub name: String,
    pub type_: Option<TypeDef>,
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

    fn is_type(&self) -> bool {
        false
    }

    fn type_names(&self) -> Vec<TypeDef> {
        let mut types = vec![];
        if let Some(t) = self.return_type.clone() { 
            types.push(t);
        }
        types.extend(
            self.args.iter().filter_map(|x| x.type_.clone()).collect::<Vec<TypeDef>>()
        );
        types
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

    fn is_type(&self) -> bool {
        false
    }

    fn type_names(&self) -> Vec<TypeDef> {
        vec![]
    }
}


/*
FunctionCall
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub ast_fields: AstSymbolFields,
    pub caller_guid: Option<String>,
    pub args_guids: Vec<String>,
    pub func_decl_guid: Option<String>,
}

impl Default for FunctionCall {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
            caller_guid: None,
            args_guids: vec![],
            func_decl_guid: None,
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for FunctionCall {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn is_type(&self) -> bool {
        false
    }

    fn type_names(&self) -> Vec<TypeDef> {
        vec![]
    }
}


/*
VariableUsage
*/
#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct VariableUsage {
    pub ast_fields: AstSymbolFields,
    pub var_decl_guid: Option<String>,
}

impl Default for VariableUsage {
    fn default() -> Self {
        Self {
            ast_fields: AstSymbolFields::default(),
            var_decl_guid: None,
        }
    }
}

#[async_trait]
#[typetag::serde]
impl AstSymbolInstance for VariableUsage {
    fn fields(&self) -> &AstSymbolFields {
        &self.ast_fields
    }

    fn is_type(&self) -> bool {
        false
    }

    fn type_names(&self) -> Vec<TypeDef> {
        vec![]
    }
}