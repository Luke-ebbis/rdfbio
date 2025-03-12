//! Interacting with the OLS
//!

/// Queries to the OLS v4 endpoint
pub mod api {

    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]

    #[derive(Default, Clone, Copy, PartialEq, PartialOrd, Eq, Debug, Ord)]
    pub enum Ontologies {
        #[default]
        NcbiTaxon,
    }

    impl fmt::Display for Ontologies {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let out: &str = match &self {
                Ontologies::NcbiTaxon => "ncbitaxon",
            };

            write!(f, "{}", out)
        }
    }

    use crate::biordf::ols::data::ApiResponse;
    use core::fmt;
    use derive_builder::Builder;

    use iref::IriBuf;
    use log::info;
    use reqwest::{self};
    // use serde::ser::StdError;
    use std::error::Error;

    #[derive(Debug)]
    pub enum SearchError {
        InvalidStartValue(i32, i32),                        // the URL that failed
        Other(String),                                      // (start, total hits)
        JsonParseFailed(serde_json::Error),                 // Store the serde error here
        RequestFailed(reqwest::StatusCode, String, String), // (status code, error message, query)
        UrlParseFailed(String),                             // Generic catch-all error
    }
    use quick_xml::events::Event;
    use quick_xml::Reader;

    impl SearchError {
        /// Parse an XML error response and map it to `SearchError`
        fn from_xml(xml: &str) -> Self {
            let mut reader = Reader::from_str(xml);
            // reader.trim_text(true);

            let mut message = String::new();

            loop {
                match reader.read_event() {
                    Ok(Event::Eof) => break, // End of file
                    Ok(Event::Text(e)) => {
                        let text = e.unescape().unwrap_or_default();
                        if message.is_empty() {
                            message = text.to_string();
                        }
                    }
                    Err(_) => return SearchError::Other("Failed to parse XML response".into()),
                    _ => {}
                }
            }

            SearchError::Other(message)
        }
    }

    impl fmt::Display for SearchError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                SearchError::InvalidStartValue(start, hits) => {
                    write!(f, "Invalid 'start' value {}. It must be less than the total number of hits ({}). Rerun with start of <={}. ", start, hits, hits-1)
                }
                SearchError::RequestFailed(status, message, url_string) => {
                    write!(
                        f,
                        "Request failed with status code {}: {} url was {}",
                        status, message, url_string
                    )
                }
                SearchError::UrlParseFailed(url) => {
                    write!(f, "Failed to parse URL: {}", url)
                }
                SearchError::JsonParseFailed(err) => {
                    write!(f, "Failed to parse JSON response: {}", err)
                }
                SearchError::Other(msg) => {
                    write!(f, "An unknown error occurred: {}", msg)
                }
            }
        }
    }
    // impl StdError for SearchError {}
    impl Error for SearchError {}

    impl From<reqwest::Error> for SearchError {
        fn from(err: reqwest::Error) -> Self {
            // If the reqwest error is related to status codes, you can extract them
            let status = err
                .status()
                .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            let url = err.url().unwrap().to_string();
            SearchError::RequestFailed(status, err.to_string(), url)
        }
    }

    impl From<serde_json::Error> for SearchError {
        fn from(err: serde_json::Error) -> Self {
            SearchError::JsonParseFailed(err)
        }
    }

    /// The data fields of the dataset REST endpoint
    #[derive(Clone, Debug)]
    pub enum Field {
        /// The omics domain.
        publication_date,
    }

    #[derive(Clone, Debug)]
    pub enum Order {
        ascending,
        descending,
    }

    /// Search the OmicsDI rest database endpoint
    ///
    /// Documentation for the parameters is copied from there.
    #[derive(Builder, std::marker::Copy, Default, Debug, PartialEq, Eq, Ord, PartialOrd, Clone)]
    #[builder(build_fn(validate = "Self::validate"))]
    pub struct Search<'a> {
        // domain: Domain,
        /// General search term against multiple fields including, e.g: cancer human
        query: &'a str,

        #[builder(default=Ontologies::NcbiTaxon)]
        pub(crate) ontology: Ontologies,
        /// Size of the return, needs to be below 1000.
        #[builder(setter(into), default = "2")]
        pub(crate) size: i32,
        /// face count. The summary of the dataset that is returned.
        #[builder(setter(into), default = "0")]
        facet_size: i32, // order: Option<Order>,
    }

    impl SearchBuilder<'_> {
        const MAX_REQUEST_SIZE: i32 = 1000;

        fn validate_size(size: i32) -> Result<(), String> {
            if size > Self::MAX_REQUEST_SIZE {
                Err(format!(
                    "Search size must be less than {}, size was {}!",
                    Self::MAX_REQUEST_SIZE,
                    size
                ))
            } else {
                Ok(())
            }
        }

        /// Check that the size of the query is smaller than the start.
        /// This is to conform to the ENA api requirements.
        fn validate(&self) -> Result<(), String> {
            let size = self.size.unwrap_or(2);
            let search_size = size;
            Self::validate_size(search_size)?;
            Ok(())
        }
    }

    impl Search<'_> {
        const REST_URL: &'static str = "https://www.ebi.ac.uk/ols4/api/v2/ontologies/";
        pub const MAX_REQUEST_SIZE: i32 = SearchBuilder::MAX_REQUEST_SIZE;

        pub fn total_hit(&self) -> Result<i32, SearchError> {
            let mut search = *self;
            search.size = 1;
            let hits = search.search();
            match hits {
                Err(SearchError::InvalidStartValue(_, end)) => Ok(end),
                Ok(r) => Ok(r.numElements as i32),
                Err(e) => Err(e),
            }
        }

        fn request(
            params: Vec<(&str, &str)>,
            header: &str,
            ontology: Ontologies,
        ) -> Result<ApiResponse, SearchError> {
            let url = format!("{}{}/classes", Self::REST_URL, ontology);
            let url = reqwest::Url::parse_with_params(&url, params)
                .map_err(|e| SearchError::UrlParseFailed(e.to_string()))?;
            info!("Sending query: {}", url);
            let client = reqwest::blocking::Client::new();
            let response = client
                .get(url.clone())
                .header("accept", header)
                .send()
                .map_err(|_| {
                    SearchError::RequestFailed(
                        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to send request".into(),
                        url.to_string(),
                    )
                })?;

            let status = response.status();
            let text = response.text()?;

            if status.is_success() {
                let json_text: String = text;
                let _ = super::data::check_for_null_fields(&json_text);
                let deserialized: ApiResponse = serde_json::from_str(&json_text)?;
                return Ok(deserialized);
            }

            // If an error occurs, parse the XML response
            Err(SearchError::from_xml(&text))
        }
        /// Search the OmicsDi database with a search string.
        ///
        pub fn search(self) -> Result<ApiResponse, SearchError> {
            let accept_header = "application/json";
            let x = self.query.to_string();
            let size = self.size;
            let size = size.to_string();
            let params: Vec<(&str, &str)> = vec![("size", &size), ("search", &x.as_str())];
            let out = Self::request(params, accept_header, self.ontology)?;
            Ok(out)
        }
    }
}

pub mod data {

    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]

    use serde_json::Value;
    use std::collections::HashMap;

    use iref::IriBuf;
    use serde::Serializer;
    /// The link to the dataset enpoint
    use serde::{Deserialize, Serialize};

    use serde::de::{self, Deserializer};
    #[derive(Debug, Deserialize)]
    pub(crate) struct ApiResponse {
        pub(crate) page: u32,
        pub(crate) numElements: u32,
        pub(crate) elements: Vec<Element>,
    }

    #[derive(Debug, Deserialize)]
    pub(crate) struct Element {
        pub(crate) appearsIn: Vec<String>,
        pub(crate) curie: String,
        pub(crate) definedBy: Vec<String>,
        pub(crate) directAncestor: Option<Vec<String>>,
        pub(crate) directParent: Option<Vec<String>>,
        pub(crate) hasDirectChildren: bool,
        pub(crate) hasDirectParents: bool,
        pub(crate) hasHierarchicalChildren: bool,
        pub(crate) hasHierarchicalParents: bool,
        pub(crate) hierarchicalAncestor: Option<Vec<String>>,
        pub(crate) hierarchicalParent: Option<Vec<String>>,
        pub(crate) hierarchicalProperty: Option<String>,
        pub(crate) imported: bool,
        pub(crate) iri: String,
        pub(crate) isDefiningOntology: bool,
        pub(crate) isObsolete: bool,
        pub(crate) isPreferredRoot: bool,
        pub(crate) label: Vec<String>,
        pub(crate) linkedEntities: Option<serde_json::Value>,
    }
    fn null_check<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(value) => Ok(value),
            None => Err(de::Error::custom("Field is null")),
        }
    }

    /// Custom serializer for converting a u64 to a string
    fn u64_to_string<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    fn string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        match s {
            Some(value) => value.parse::<u64>().map_err(de::Error::custom),
            None => Err(de::Error::custom("Count field is null")),
        }
    }
    /// Making a uri for OmicsDB records
    fn string_to_uri<'de, D>(deserializer: D) -> Result<IriBuf, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let st = format!("https://www.omicsdi.org/dataset/{}", s);
        IriBuf::new(st).map_err(de::Error::custom)
    }
    /// Making a string
    fn uri_to_string<S>(value: &IriBuf, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(value.as_str())
    }

    pub fn check_for_null_fields(json: &str) -> Result<(), String> {
        let value: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        check_for_null_fields_recursive(&value, "");
        Ok(())
    }

    fn check_for_null_fields_recursive(value: &Value, parent_key: &str) {
        match value {
            Value::Null => {
                ()
                // log::warn!("Field '{}' is null", parent_key);
            }
            Value::Object(map) => {
                for (key, val) in map {
                    let full_key = if parent_key.is_empty() {
                        key.to_string()
                    } else {
                        format!("{}.{}", parent_key, key)
                    };
                    check_for_null_fields_recursive(val, &full_key);
                }
            }
            Value::Array(arr) => {
                for (index, val) in arr.iter().enumerate() {
                    let full_key = format!("{}[{}]", parent_key, index);
                    check_for_null_fields_recursive(val, &full_key);
                }
            }
            _ => {

                // log::info!("Field '{}' has a valid value: {:?}", parent_key, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use test_log::test;

    use super::{api::SearchBuilder, data::ApiResponse};

    fn test_ols_taxa_parse() {
        let json_data = r#"
            {
                "page": 0,
                "numElements": 1,
                "totalPages": 1,
                "totalElements": 1,
                "elements": [
                    {
                        "appearsIn": ["ons", "ncbitaxon", "foodon"],
                        "curie": "NCBITaxon:34772",
                        "definedBy": ["ncbitaxon"],
                        "directAncestor": ["http://purl.obolibrary.org/obo/NCBITaxon_55119"],
                        "directParent": ["http://purl.obolibrary.org/obo/NCBITaxon_55119"],
                        "hasDirectChildren": true,
                        "hasDirectParents": true,
                        "hasHierarchicalChildren": true,
                        "hasHierarchicalParents": true,
                        "hierarchicalAncestor": ["http://purl.obolibrary.org/obo/NCBITaxon_55119"],
                        "hierarchicalParent": ["http://purl.obolibrary.org/obo/NCBITaxon_55119"],
                        "hierarchicalProperty": "http://www.w3.org/2000/01/rdf-schema#subClassOf",
                        "imported": false,
                        "iri": "http://purl.obolibrary.org/obo/NCBITaxon_34772",
                        "isDefiningOntology": true,
                        "isObsolete": false,
                        "isPreferredRoot": false,
                        "label": ["Alosa"],
                        "linkedEntities": {
                            "http://purl.obolibrary.org/obo/NCBITaxon_131567": {
                                "definedBy": ["ncbitaxon"],
                                "label": ["cellular organisms"],
                                "curie": "NCBITaxon:131567"
                            }
                        }
                    }
                ]
            }
            "#;

        let parsed: ApiResponse = serde_json::from_str(json_data).expect("Failed to parse JSON");
    }

    #[test]
    fn test_ols_ncbi() -> Result<(), Box<dyn Error>> {
        let mut binding = SearchBuilder::default();
        let ols_builder = binding.query("Bremia");
        let result = ols_builder.build()?.search()?;
        let element_1 = result.elements.get(0);
        match element_1.clone() {
            Some(e) => {
                dbg!(&e.hierarchicalParent);
            }
            None => (),
        }
        Ok(())
    }
}
