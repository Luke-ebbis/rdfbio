#[allow(non_snake_case)]
pub mod Api {

    use crate::biordf::Omicsdi::data::OmicsDiResponse;
    use derive_builder::Builder;
    use reqwest::{self, Url};

    #[derive(Clone, Debug)]
    pub enum Domain {
        /// The omics domain.
        omics,
        /// The Pride database
        pride,
        MassIVE,
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

    /// Search the OmicsDI rest endpoint
    ///
    /// Documentation for the parameters is copied from there.
    #[derive(Builder)]
    pub struct Search {
        // domain: Domain,
        /// General search term against multiple fields including, e.g: cancer human
        query: String,
        // /// Field to sort the output of the search results, e.g: id, publication_date
        // sort: Option<Field>,
        // start: Option<u32>,
        // end: Option<u32>,
        // order: Option<Order>,
    }

    impl Search {
        const REST_URL: &str = "https://www.omicsdi.org/ws/dataset/search";
        /// Search the OmicsDi database with a search string.
        ///
        pub async fn search(self) -> Result<OmicsDiResponse, Box<dyn std::error::Error>> {
            let accept_header = "application/json";
            let x = self.query;
            let start = 1;
            let size = 2;
            let params = [
                ("query", x),
                // ("start", start.to_string()),
                // ("size", size.to_string()),
            ];
            let url = Url::parse_with_params(Self::REST_URL, params);
            match url {
                Ok(url) => {
                    let url = url.to_string().replace("+", "%20");
                    dbg!(&url);
                    let client = reqwest::Client::new();
                    let response = client
                        .get(url)
                        .header("accept", accept_header)
                        .send()
                        .await?;
                    if response.status().is_success() {
                        let json_text: String = response.text().await?;

                        let deserialized: OmicsDiResponse = serde_json::from_str(&json_text)?;
                        Ok(deserialized)
                    } else {
                        Err(Box::from(format!(
                            "Failed to fetch data: {}",
                            response.status()
                        )))
                    }
                }
                Err(e) => Err(Box::from("The url could not be made".to_string())),
            }
        }
    }
}

pub mod data {
    use serde::Serializer;
    /// The link to the dataset enpoint
    use serde::{Deserialize, Serialize};
    use std::error::Error;

    use serde::de::{self, Deserializer};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct OmicsDiResponse {
        pub count: u64,
        pub datasets: Option<Vec<DataSet>>,
        pub facets: Option<Vec<Facet>>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct DataSet {
        pub id: String,
        pub source: String,
        pub title: String,
        pub description: Option<String>,
        pub organisms: Option<Vec<Organism>>,
        pub publicationDate: Option<String>,
        pub omicsType: Option<Vec<String>>,
        pub citationsCount: Option<u64>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Organism {
        pub acc: Option<String>,
        pub name: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Facet {
        pub id: String,
        pub label: String,
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
        let s = String::deserialize(deserializer)?;
        s.parse::<u64>().map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::biordf::Omicsdi::Api::SearchBuilder;

    #[tokio::test]
    async fn test_input() -> Result<(), Box<dyn Error>> {
        let mut x = SearchBuilder::default();
        let q: String = "TAXONOMY: 164328 AND omics_type:Transcriptomics".into();
        let mut query = x.query(q).build()?;
        let mut results = query.search().await?;
        let first_identifier = results.datasets.unwrap().pop().unwrap().id;
        dbg!(first_identifier.clone());
        assert_eq!(first_identifier, "E-GEOD-50033");
        Ok(())
    }
}
