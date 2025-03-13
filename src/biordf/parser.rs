use log::{self, info};
use reqwest::blocking::get;
use serde_json;

use crate::biordf::{
    core::searching::{page, Pager, SearchSize},
    ols::api::SearchBuilder,
};

use super::omicsdi::data::OmicsDiResponse;

#[derive(Debug, Clone)]
enum QueryExpr {
    Not(Box<QueryExpr>),
    And(Box<QueryExpr>, Box<QueryExpr>),
    Or(Box<QueryExpr>, Box<QueryExpr>),
    SearchFunction(SearchFunction),
    FreeText(String),
}

#[derive(Debug, Clone)]
enum SearchFunction {
    TaxTree(u32), // e.g., tax-tree 53985
}

fn tokenize_lisp_query(query: &str) -> Vec<String> {
    info!("processing query\n {}", query);
    let mut tokens = Vec::new();
    let mut current_token = String::new();

    for c in query.chars() {
        match c {
            '(' | ')' => {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                tokens.push(c.to_string());
            }
            ' ' => {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            _ => current_token.push(c),
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
}

fn resolve_tax_tree(id: u32) -> Vec<String> {
    use crate::biordf::ols;
    info!("looking at the synonyms for {}", id);
    let mut builder = ols::api::SearchBuilder::default();
    let binding = id.clone().to_string();
    builder.query(&binding).mode(ols::api::Mode::Forward);
    let result = builder.build().unwrap().search();
    vec!["".to_string()]
}

fn execute_query(expr: QueryExpr) -> Vec<OmicsDiResponse> {
    info!("Executing {:?}", expr);

    match expr {
        QueryExpr::Not(inner) => {
            info!("looking at negated terms");
            let excluded_terms = match *inner {
                QueryExpr::SearchFunction(SearchFunction::TaxTree(id)) => resolve_tax_tree(id),
                _ => vec![],
            };

            let results = fetch_omicsdi_results(""); // ✅ Now query OmicsDI with an empty string
            results
                .into_iter()
                .filter(|dataset| {
                    !excluded_terms
                        .iter()
                        .any(|term| dataset.clone().contains(term)) // ✅ Finally, apply `Not`
                })
                .collect()
        }

        QueryExpr::And(left, right) => {
            let left_results = execute_query(*left);
            let right_results = execute_query(*right);
            left_results
                .into_iter()
                .filter(|d| right_results.contains(d))
                .collect()
        }
        QueryExpr::Or(left, right) => {
            let mut results = execute_query(*left);
            results.extend(execute_query(*right));
            results
        }
        QueryExpr::SearchFunction(SearchFunction::TaxTree(id)) => {
            todo!("Taxtree not yet implemented")
        }
        QueryExpr::FreeText(text) => fetch_omicsdi_results(&text),
    }
}

trait Searching {
    fn contains(self, query: &str) -> bool;
}

impl Searching for OmicsDiResponse {
    fn contains(self, query: &str) -> bool {
        todo!("filtering not yet implemented")
    }
}

fn fetch_omicsdi_results(query: &str) -> Vec<OmicsDiResponse> {
    use crate::biordf::omicsdi::api::SearchBuilder as omicsdiSearchBuilder;
    info!("Fetching omics results");
    let size = 1;
    let start = 1;
    let size: i32 = size.try_into().expect("Size too large for i32");
    let start: i32 = start.try_into().expect("Start value too large for i32");
    let search_size = {
        if size > 1000 {
            1000
        } else {
            size
        }
    };

    let mut binding = omicsdiSearchBuilder::default();
    let user_query = query.clone();
    let query_builder = binding.size(search_size).start(start).query(&user_query);
    let pager: Pager<omicsdiSearchBuilder> = Pager::new(query_builder, SearchSize::Amount(size));
    let results = page(pager).unwrap();
    vec![results]
}

fn parse_lisp_query(tokens: &mut Vec<String>) -> QueryExpr {
    info!("recieved tokens {:?}", tokens.clone());
    if tokens.is_empty() {
        panic!("Unexpected end of tokens");
    }

    let token = tokens.remove(0);
    match token.as_str() {
        "(" => {
            if tokens.is_empty() {
                panic!("Unexpected end of tokens after '('");
            }

            let func = tokens.remove(0);
            let expr = match func.as_str() {
                "text" => {
                    let text = tokens.remove(0).trim_matches('"').to_string();
                    QueryExpr::FreeText(text)
                }
                "not" => QueryExpr::Not(Box::new(parse_lisp_query(tokens))),
                "and" => QueryExpr::And(
                    Box::new(parse_lisp_query(tokens)),
                    Box::new(parse_lisp_query(tokens)),
                ),
                "or" => QueryExpr::Or(
                    Box::new(parse_lisp_query(tokens)),
                    Box::new(parse_lisp_query(tokens)),
                ),
                "tax-tree" => {
                    let id = tokens.remove(0).parse::<u32>().unwrap();
                    QueryExpr::SearchFunction(SearchFunction::TaxTree(id))
                }
                _ => panic!("Unknown function: {}", func),
            };

            if tokens.remove(0) != ")" {
                panic!("Expected closing ')'");
            }

            expr
        }
        ")" => panic!("Unexpected `)`"),
        _ => panic!("Unknown token: {}", token),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    #[ignore]
    fn test_dsl_complex() {
        let query = "(not (tax-tree 53985))";
        let mut tokens: Vec<String> = tokenize_lisp_query(query);
        let parsed = parse_lisp_query(&mut tokens);
        let results = execute_query(parsed);
        println!("{:?}", results);
    }
    #[test]
    #[ignore]
    fn test_dsl_easy() {
        let query = "(text \"fish\"))";
        let mut tokens: Vec<String> = tokenize_lisp_query(query);
        let parsed = parse_lisp_query(&mut tokens);
        let results = execute_query(parsed);
        println!("{:?}", results);
    }
    #[test]
    #[ignore]
    #[should_panic]
    fn test_dsl_easy_wrong_paren() {
        let query = "(text fish))";
        let mut tokens: Vec<String> = tokenize_lisp_query(query);
        let parsed = parse_lisp_query(&mut tokens);
        let results = execute_query(parsed);
        println!("{:?}", results);
    }
}
