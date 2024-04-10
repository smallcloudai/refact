// mod cpp;
mod rust;
mod python;
mod java;
// mod cpp;
// pub(crate) fn test_query_function(mut parser: Box<dyn LanguageParser>,
//                                   path: &PathBuf,
//                                   code: &str,
//                                   ref_indexes: HashMap<String, SymbolDeclarationStruct>,
//                                   ref_usages: Vec<Box<dyn UsageSymbolInfo>>) {
//     let indexes = parser.parse_declarations(code, &path).unwrap();
//     let usages = parser.parse_usages(code, true).unwrap();
//
//     indexes.iter().for_each(|(key, index)| {
//         assert_eq!(index, ref_indexes.get(key).unwrap());
//     });
//     ref_indexes.iter().for_each(|(key, index)| {
//         assert_eq!(index, indexes.get(key).unwrap());
//     });
//
//     usages.iter().for_each(|usage| {
//         assert!(ref_usages.contains(usage));
//     });
//     ref_usages.iter().for_each(|usage| {
//         assert!(usages.contains(usage));
//     });
// }
