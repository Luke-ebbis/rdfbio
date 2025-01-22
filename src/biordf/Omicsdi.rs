pub mod DataSet {
    /// The link to the dataset enpoint
    const REST_URL: &str = "https://www.omicsdi.org/ws/dataset/search";
    use reqwest::{self, Url};
    use serde::{Deserialize, Serialize};
    use std::error::Error;

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct OmicsDiResponse {
        pub count: u64,
        pub datasets: Vec<DataSet>,
        pub facets: Vec<Facet>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct DataSet {
        pub id: String,
        pub source: String,
        pub title: String,
        pub description: Option<String>,
        pub organisms: Vec<Organism>,
        pub publicationDate: Option<String>,
        pub omicsType: Vec<String>,
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
        pub facetValues: Vec<FacetValue>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct FacetValue {
        pub label: String,
        pub value: String,
        #[serde(deserialize_with = "string_to_u64")]
        pub count: u64,
    }

    use serde::de::{self, Deserializer};

    fn string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<u64>().map_err(de::Error::custom)
    }

    /// Search the OmicsDi database with a search string.
    ///
    pub async fn search(x: String) -> Result<OmicsDiResponse, Box<dyn std::error::Error>> {
        let accept_header = "application/json";
        let start = 1;
        let size = 2;
        let params = [
            ("query", x),
            ("start", start.to_string()),
            ("size", size.to_string()),
        ];
        let url = Url::parse_with_params(REST_URL, params)?;
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
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use DataSet::search;

    use super::*;
    #[tokio::test]
    async fn test_input() -> Result<(), Box<dyn Error>> {
        let x: String = "TAXONOMY: 164328 AND omics_type:Transcriptomics".into();
        let search_string = x;
        let mut result = search(search_string).await?;
        assert_eq!("E-GEOD-13580", result.datasets.pop().unwrap().id);
        Ok(())
    }
}
