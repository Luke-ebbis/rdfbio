//! Using the endpoint, access the datasets.

pub mod access {

    pub enum Endpoints {
        /// The OmicsDI endpoint
        OmicsDI,
    }
}

pub mod api {

    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]
    use crate::biordf::api::omicsdi::data::OmicsDiResponse;
    use core::fmt;
    use derive_builder::Builder;
    use std::str::FromStr;

    use iref::IriBuf;
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

            // Check if the error message contains "The start parameter (100) is bigger than or equal to the number of hits (94)."
            if let Some((start, hits)) = Self::extract_start_error(&message) {
                return SearchError::InvalidStartValue(start, hits);
            }

            SearchError::Other(message)
        }

        /// Extract start and total hits from the error message
        fn extract_start_error(message: &str) -> Option<(i32, i32)> {
            let message = message.replace(",", ""); // Removes commas from numbers
            let re = regex::Regex::new(r"The start parameter \((\d+)\) is bigger than or equal to the number of hits \((\d+)\)\.").ok()?;
            let caps = re.captures(&message)?;
            let start = caps.get(1)?.as_str().parse().ok()?;
            let hits = caps.get(2)?.as_str().parse().ok()?;
            Some((start, hits))
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

    #[derive(Clone, Debug)]
    pub enum Domain {
        /// The omics domain.
        omics,
        /// The Pride database
        pride,
        /// The massive domain.
        MassIVE,
        /// JPost domain.
        jpost,
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
        pub(crate) query: &'a str,
        // /// Field to sort the output of the search results, e.g: id, publication_date
        #[builder(setter(into), default = "0")]
        // sort: Option<Field>,
        /// The start of the query. Increment this to page.
        pub(crate) start: i32,
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

        pub fn get_query(&self) -> String {
            match self.query {
                Some(q) => q.to_string(),
                None => "".to_string(),
            }
        }
    }

    impl Search<'_> {
        const REST_URL: &'static str = "https://www.omicsdi.org/ws/dataset/search";
        pub const MAX_REQUEST_SIZE: i32 = SearchBuilder::MAX_REQUEST_SIZE;

        pub fn total_hit(&self) -> Result<i32, SearchError> {
            let mut search = *self;
            search.size = 1;
            let hits = search.search();
            match hits {
                Err(SearchError::InvalidStartValue(_, end)) => Ok(end),
                Ok(r) => Ok(r.count as i32),
                Err(e) => Err(e),
            }
        }

        fn request(
            params: Vec<(&str, &str)>,
            header: &str,
        ) -> Result<OmicsDiResponse, SearchError> {
            let url = Self::REST_URL;
            let url = reqwest::Url::parse_with_params(url, params)
                .map_err(|e| SearchError::UrlParseFailed(e.to_string()))?;
            let url_string = url.clone().to_string();
            let client = reqwest::blocking::Client::new();
            let response = client
                .get(url)
                .header("accept", header)
                .send()
                .map_err(|_| {
                    SearchError::RequestFailed(
                        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to send request".into(),
                        url_string.to_string(),
                    )
                })?;

            let status = response.status();
            let text = response.text()?;

            if status.is_success() {
                let json_text: String = text;
                let _ = super::data::check_for_null_fields(&json_text);
                let mut deserialized: OmicsDiResponse = serde_json::from_str(&json_text)?;
                let mut sets: Vec<super::data::DataSet> = Vec::new();
                for ds in deserialized.datasets.clone().unwrap().iter_mut() {
                    ds.id = IriBuf::from_str(&format!(
                        "https://www.omicsdi.org/dataset/{}/{}",
                        ds.source,
                        ds.id
                            .strip_prefix("https://www.omicsdi.org/dataset/")
                            .unwrap()
                    ))
                    .unwrap_or(ds.id.clone());
                    sets.push(ds.clone());
                }
                deserialized.datasets = Some(sets);
                return Ok(deserialized);
            }

            // If an error occurs, parse the XML response
            Err(SearchError::from_xml(&text))
        }
        /// Search the OmicsDi database with a search string.
        ///
        pub fn search(self) -> Result<OmicsDiResponse, SearchError> {
            let accept_header = "application/json";
            let x = self.query;
            let start = self.start;
            let size = self.size;
            let start = start.to_string();
            let size = size.to_string();
            let params = vec![("query", x), ("start", &start), ("size", &size)];
            let out = Self::request(params, accept_header)?;
            Ok(out)
        }
    }
}

pub mod data {

    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]
    use iref::IriBuf;
    use linked_data_next::Serialize as ldSerialize;
    use serde_with::SerializeDisplay;
    use std::error::Error;

    use serde_json::Value;
    use std::collections::HashMap;

    use linked_data_next;
    use serde::de::{self, Deserializer};
    use serde::Serializer;
    /// The link to the dataset enpoint
    use serde::{Deserialize, Serialize};
    use serde_with::{formats::ColonSeparator, serde_as, StringWithSeparator};


    use linked_data_next::{LinkedDataResource, LinkedDataSubject};
    use rdf_types::{Interpretation, Vocabulary, Term};
    use std::ops::Deref;

    #[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
    pub struct OmicsDiResponse {
        pub count: u64,
        pub datasets: Option<Vec<DataSet>>,
        // pub facets: Option<Vec<Facet>>,
    }

    #[derive(
        serde::Serialize,
        serde::Deserialize,
        linked_data_next::Serialize,
        linked_data_next::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        PartialOrd,
        Ord,
    )]
    #[serde_as]
    #[ld(prefix("id" = "http://example.com/unprocessed"))]
    #[ld(prefix("ex" = "http://example.com/verbs/"))]
    #[ld(prefix("rdf" = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"))]
    #[ld(prefix("dcterms" = "http://purl.org/dc/terms/" ))]
    #[ld(type = "ex:OmicDiDataSet")]
    pub struct DataSet {
        #[ld(id)]
        #[serde(deserialize_with = "string_to_uri", serialize_with = "uri_to_string")]
        pub id: IriBuf,
        #[ld("ex:source")]
        #[serde(deserialize_with = "null_check")]
        pub source: String,
        #[ld("dcterms:title")]
        pub title: Option<String>,
        // #[ld(ignore)]
        // pub keywords: Option<String>,
        // #[ld(ignore)]
        // pub score: Option<u64>,
        #[ld("dcterms:description")]
        pub description: Option<String>,
        #[ld(ignore)]
        pub organisms: Option<Vec<Organism>>,
        #[ld(ignore)]
        pub publicationDate: Option<String>,
        #[ld("rdf:type")]
        #[serde(deserialize_with = "vec_string_to_iri", serialize_with = "vec_iri_to_string")]
        pub omicsType: Vec<IriBuf>,
        // #[ld("ex:citations")]
        // pub citationsCount: Option<u64>,
        // #[ld(ignore)]
        // #[serde(flatten)]
        // pub extra_fields: HashMap<String, serde_json::Value>,
    }

    use derive_more::Display;
    #[derive(
        Deserialize,
        Serialize,
        Debug,
        Clone,
        Eq,
        PartialEq,
        PartialOrd,
        Ord,
        Display
    )]
    pub enum OmicsType {
        //  TODO
        #[display("proteomics")]
        Proteomics,
        #[display("genomics")]
        Genomics,
        #[display("uknown")]
        Unknown,
        Multiomics,
    }

    #[derive(
        linked_data_next::Serialize,
        linked_data_next::Deserialize,
        Deserialize,
        Serialize,
        Debug,
        Clone,
        Eq,
        PartialEq,
        PartialOrd,
        Ord,
    )]
    #[ld(prefix("ex" = "http://example.org/verbs/"))]
    #[ld(type = "ex:OmicsDiOrganism")]
    pub struct Organism {
        #[ld("ex:taxid")]
        pub acc: String,
        #[ld("ex:aka")]
        pub name: String,
    }

    use serde::ser::SerializeSeq;

    /// Serialize `Option<Vec<IriBuf>>` as a JSON array of strings
    pub fn vec_iri_to_string<S>(
        value: &Vec<IriBuf>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec = value;
        let vec_len = Some(vec.len());
                let mut seq = serializer.serialize_seq(vec_len)?;
                for iri in vec {
                    seq.serialize_element(iri.as_str())?;
                }
                seq.end()
        }

    fn vec_string_to_iri<'de, D>(deserializer: D) -> Result<Vec<IriBuf>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Vec<String> = Option::deserialize(deserializer)?.expect("msg");
                let mut iri_vec = Vec::new();
                for v in s  {
                    // dbg!(&v);
                    let iri_string = format!("https://example.com/{}", v.replace(" ", "_"));
                    iri_vec.push(IriBuf::new(iri_string).map_err(de::Error::custom)?);
                }
                Ok(iri_vec)
        }


    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Facet {
        pub id: String,
        pub label: Option<String>,
        pub total: u64,
        pub facetValues: Option<Vec<FacetValue>>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct FacetValue {
        pub label: String,
        pub value: String,
        #[serde(deserialize_with = "string_to_u64", serialize_with = "u64_to_string")]
        pub count: u64,
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
    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]
    use std::error::Error;

    use crate::biordf::{
        api::omicsdi::api::{SearchBuilder, SearchError},
        core::searching::Pageable,
    };

    /// Database connection check...
    #[test]
    fn test_input() -> Result<(), Box<dyn Error>> {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        let query = x.query(&q).build()?;
        let results = query.search()?;
        let first_identifier = results.clone().datasets.unwrap().pop().unwrap().id;
        assert_eq!(
            first_identifier,
            "https://www.omicsdi.org/dataset/biostudies-arrayexpress/E-GEOD-5003"
        );

        let _ = x.query("fish".into()).facet_size(1000).build()?;
        Ok(())
    }

    #[should_panic]
    #[test]
    fn test_pre_search_validation_error_size_and_start() -> () {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        // This is invalid and should not be allowed.
        let _ = x.query(&q).start(19).size(10005).build().unwrap();
    }

    #[test]
    /// For the mut, it alters all subsequent setters...
    fn test_setters() -> Result<(), Box<dyn Error>> {
        let mut binding = SearchBuilder::default();
        let mut x = binding.start(1).size(10).query("s");
        let x2 = x.start(30).size(100);
        let test_start = x2.get_start()?;
        let test_size = x2.build()?.size;
        assert!(test_start == 30);
        assert!(test_size == 100);
        let other = x.get_start()?;
        assert!(other == 30);
        Ok(())
    }
    #[should_panic]
    #[test]
    fn test_pre_search_validation_error_size() -> () {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        // This is invalid and should not be allowed.
        let _ = x.query(&q).start(19).size(500000).build().unwrap();
    }

    #[test]
    fn test_request_errors() -> Result<(), Box<dyn Error>> {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        // This is invalid and should not be allowed.
        let r = x.query(&q).start(19).build().unwrap();
        let out = r.search();
        match out {
            Err(SearchError::InvalidStartValue(start, total)) => {
                assert!(start == 19);
                assert!(total == 1);
                Ok(())
            }
            Err(_) => Err(Box::from("Wrong error value")),
            Ok(_) => Err(Box::from("this request should have failed")),
        }
    }
    #[test]
    fn test_request_errors_2() -> Result<(), Box<dyn Error>> {
        let mut x = SearchBuilder::default();
        let q = "E-GEOD-5003";
        // This is invalid and should not be allowed.
        let r = x.query(&q).build().unwrap();
        let out = r.search();
        match out {
            Err(_) => Err(Box::from("Wrong error value")),
            Ok(r) => {
                assert!(r.count == 1);
                Ok(())
            }
        }
    }
}
