pub mod DataSet {
    /// The link to the dataset enpoint
    const REST_URL: &str = "https://www.omicsdi.org/ws/dataset/search";
    use reqwest;
    use serde::{Deserialize, Serialize};
    use std::error::Error;

    #[derive(Deserialize, Debug, Clone)]
    pub struct OmicsDiResponse {
        count: u64,
        pub datasets: Vec<DataSet>,
        facets: Vec<Facet>,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct DataSet {
        pub id: String,
        source: String,
        title: String,
        description: Option<String>,
        organisms: Vec<Organism>,
        publicationDate: Option<String>,
        omicsType: Vec<String>,
        citationsCount: Option<u64>,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct Organism {
        acc: Option<String>,
        name: String,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct Facet {
        id: String,
        label: String,
        total: u64,
        facetValues: Vec<FacetValue>,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct FacetValue {
        label: String,
        value: String,
        #[serde(deserialize_with = "string_to_u64")]
        count: u64,
    }
    use serde::de::{self, Deserializer};

    fn string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<u64>().map_err(de::Error::custom)
    }

    pub async fn search(x: String) -> Result<OmicsDiResponse, Box<dyn std::error::Error>> {
        let accept_header = "application/json";
        let x: String =
            "TAXONOMY%3A%204787%20AND%20omics_type%3A%20Transcriptomics&start=0&size=2".into();
        let query_url = format!(
            "{}?query={}&start=2&size=4",
            REST_URL,
            x, // args.start, args.size
        );

        let client = reqwest::Client::new();
        let response = client
            .get(query_url)
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
}
