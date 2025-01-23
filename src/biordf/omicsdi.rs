/// Using the endpoint, access the datasets.

pub mod access {}

pub mod api {

    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]
    use crate::biordf::omicsdi::data::{
        check_for_null_fields, OmicsDiResponse,
    };
    use derive_builder::Builder;
    use log::{info, warn};
    use reqwest::{self, Url};

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
    #[derive(Builder, Default, Debug, PartialEq)]
    #[builder(build_fn(validate = "Self::validate"))]
    pub struct Search {
        // domain: Domain,
        /// General search term against multiple fields including, e.g: cancer human
        query: String,
        // /// Field to sort the output of the search results, e.g: id, publication_date
        #[builder(setter(into), default = "0")]
        // sort: Option<Field>,
        /// The start of the query. Needs to be smaller than the return.
        start: i32,
        /// Size of the return, needs to be below 100.
        #[builder(setter(into), default = "2")]
        size: i32,
        // order: Option<Order>,
    }

    impl SearchBuilder {
        const MAX_REQUEST_SIZE: i32 = 10_000;
        /// Check that the size of the query is smaller than the start.
        /// This is to conform to the ENA api requirements.
        fn validate(&self) -> Result<(), String> {
            let start = self.start.unwrap_or_default();
            let size = self.size.unwrap_or(2);

            if size > Self::MAX_REQUEST_SIZE {
                Err(format!(
                    "Search size must be less than {}!",
                    Self::MAX_REQUEST_SIZE
                ))
            } else {
                if size <= start {
                    Err(format!(
                    "Start {} must be smaller than the size of the query {}",
                    start, size
                ))
                } else {
                    Ok(())
                }
            }
        }
    }

    impl Search {
        const REST_URL: &str = "https://www.omicsdi.org/ws/dataset/search";
        /// Search the OmicsDi database with a search string.
        ///
        pub async fn search(
            self
        ) -> Result<OmicsDiResponse, Box<dyn std::error::Error>> {
            let accept_header = "application/json";
            let x = self.query;
            let start = self.start;
            let size = self.size;
            let params = [
                ("query", x),
                ("start", start.to_string()),
                ("size", size.to_string()),
            ];
            let url = Url::parse_with_params(Self::REST_URL, params);
            match url {
                Ok(url) => {
                    let url = url.to_string().replace("+", "%20");
                    info!("Checking {url}");
                    let client = reqwest::Client::new();
                    let response = client
                        .get(url)
                        .header("accept", accept_header)
                        .send()
                        .await?;
                    if response.status().is_success() {
                        let json_text: String = response.text().await?;
                        let _ = check_for_null_fields(&json_text);

                        let deserialized: OmicsDiResponse =
                            serde_json::from_str(&json_text)?;
                        Ok(deserialized)
                    } else {
                        Err(Box::from(format!(
                            "Failed to fetch data: {}",
                            response.status()
                        )))
                    }
                }
                Err(e) => {
                    Err(Box::from(format!("The url could not be made: {e}")))
                }
            }
        }
    }
}

pub mod data {

    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]
    use log::{info, warn};
    use serde_json::{Map, Value};
    use std::collections::HashMap;

    use iref::IriBuf;
    use serde::Serializer;
    /// The link to the dataset enpoint
    use serde::{Deserialize, Serialize};

    use serde::de::{self, Deserializer};
    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct OmicsDiResponse {
        pub count: u64,
        pub datasets: Option<Vec<DataSet>>,
        // pub facets: Option<Vec<Facet>>,
    }

    #[derive(
        serde::Serialize,
        serde::Deserialize,
        linked_data::Serialize,
        linked_data::Deserialize,
        Clone,
        Debug,
    )]
    #[ld(prefix("ex" = "http://example.org/"))]
    #[ld(type = "ex:OmicDiDataSet")]
    pub struct DataSet {
        #[ld(id)]
        #[serde(
            deserialize_with = "string_to_uri",
            serialize_with = "uri_to_string"
        )]
        pub id: IriBuf,
        #[ld("ex:source")]
        #[serde(deserialize_with = "null_check")]
        pub source: String,
        #[ld("ex:title")]
        // #[serde(deserialize_with = "null_check")]
        pub title: Option<String>,
        // #[ld(ignore)]
        // pub keywords: Option<String>,
        // #[ld(ignore)]
        // pub score: Option<u64>,
        // #[ld("ex:description")]
        // pub description: Option<String>,
        // #[ld(ignore)]
        // pub organisms: Option<Vec<Organism>>,
        // #[ld(ignore)]
        // pub publicationDate: Option<String>,
        // #[ld(ignore)]
        // pub omicsType: Option<Vec<String>>,
        // #[ld("ex:citations")]
        // pub citationsCount: Option<u64>,
        #[ld(ignore)]
        #[serde(flatten)]
        pub extra_fields: HashMap<String, serde_json::Value>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Organism {
        pub acc: Option<String>,
        pub name: Option<String>,
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
        #[serde(
            deserialize_with = "string_to_u64",
            serialize_with = "u64_to_string"
        )]
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
    fn u64_to_string<S>(
        value: &u64,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
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
    /// Making a uri
    fn string_to_uri<'de, D>(deserializer: D) -> Result<IriBuf, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let st = format!("http://example.org/{}", s);
        IriBuf::new(st).map_err(de::Error::custom)
    }
    /// Making a string
    fn uri_to_string<S>(
        value: &IriBuf,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(value.as_str())
    }

    pub fn check_for_null_fields(json: &str) -> Result<(), String> {
        let value: Value =
            serde_json::from_str(json).map_err(|e| e.to_string())?;
        check_for_null_fields_recursive(&value, "");
        Ok(())
    }

    fn check_for_null_fields_recursive(
        value: &Value,
        parent_key: &str,
    ) {
        match value {
            Value::Null => {
                log::warn!("Field '{}' is null", parent_key);
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
                log::info!(
                    "Field '{}' has a valid value: {:?}",
                    parent_key,
                    value
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]
    use std::error::Error;

    use crate::biordf::omicsdi::api::SearchBuilder;

    use linked_data::iref::IriBuf;

    use rdf_types::static_iref::iri;

    /// Database connection check...
    #[tokio::test]
    async fn test_input() -> Result<(), Box<dyn Error>> {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        let query = x.query(q).build()?;
        let results = query.search().await?;
        let first_identifier =
            results.clone().datasets.unwrap().pop().unwrap().id;
        assert_eq!(first_identifier, "http://example.org/E-GEOD-5003");
        Ok(())
    }

    #[should_panic]
    #[tokio::test]
    async fn test_pre_search_validation_error_size_and_start() -> () {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        // This is invalid and should not be allowed.
        let _ = x.query(q.to_owned()).start(19).size(5).build().unwrap();
    }

    #[should_panic]
    #[tokio::test]
    async fn test_pre_search_validation_error_size() -> () {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        // This is invalid and should not be allowed.
        let _ = x
            .query(q.to_owned())
            .start(19)
            .size(500000)
            .build()
            .unwrap();
    }
    #[test]
    fn test_ld() -> () {
        #[derive(linked_data::Serialize, linked_data::Deserialize)]
        #[ld(prefix("ex" = "http://example.org/"))]
        struct Foo {
            #[ld(id)]
            id: IriBuf,

            #[ld("ex:name")]
            name: String,

            #[ld("ex:email")]
            email: String,

            #[ld("ex:numbers")]
            numbers: Vec<i64>,
            #[ld("ex:maybe")]
            maybe: Option<String>,
            #[ld("ex:alot")]
            alot: Vec<Nested>,
        }

        #[derive(linked_data::Serialize, linked_data::Deserialize)]
        #[ld(prefix("ex" = "http://example.org/"))]
        #[ld(type = "ex:object")]
        struct Nested {
            #[ld("ex:num")]
            m: i64,
        }

        let _value = Foo {
            id: iri!("http://example.org/JohnSmith").to_owned(),
            name: "John Smith".to_owned(),
            email: "john.smith@example.org".to_owned(),
            numbers: vec![1, 133],
            maybe: Some("S".into()),
            alot: vec![Nested { m: 10 }],
        };

        // for quad in quads {
        //     use rdf_types::RdfDisplay;
        //     println!("{} .", quad.rdf_display())
        // }
    }
}
