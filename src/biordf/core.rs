/// Dealing with the data from an endpoint.
pub mod data {
    // TODO: here will be a method to request data from endpoints in various formats.

    use iref::IriBuf;
    use linked_data::IntoQuadsError;
    use log::warn;
    use rdf_types::{Id, Literal, Quad, Term};

    use crate::biordf::omicsdi::data::{DataSet, OmicsDiResponse, Organism};
    pub fn dump_quads(quads: Vec<Quad<Id, IriBuf, Term>>) -> String {
        use rdf_types::RdfDisplay;
        let mut output = String::new();
        for quad in quads {
            output.push_str(&format!("{} .\n", quad.rdf_display()));
        }
        output
    }

    /// Trait to serialise different kinds of datastructures to their RDF representations.
    pub trait ToRDF {
        /// Serialise a struct to quads.
        fn to_quads(self) -> Result<Vec<Quad<Id, IriBuf, Term>>, IntoQuadsError>;
    }

    impl ToRDF for DataSet {
        /// Serialise a Dataset to quads.
        fn to_quads(self) -> Result<Vec<Quad<Id, IriBuf, Term>>, IntoQuadsError> {
            let quads = linked_data::to_quads(rdf_types::generator::Blank::new(), &self)?;
            Ok(quads)
        }
    }

    impl ToRDF for OmicsDiResponse {
        /// Serialise an omics Di response to quads.
        /// Ignore the empty taxa slots...
        fn to_quads(self) -> Result<Vec<Quad<Id, IriBuf, Term>>, IntoQuadsError> {
            let mut quads: Vec<Quad<Id, IriBuf, Term>> = Vec::new();
            for dataset in self.datasets.unwrap().iter() {
                let quad_data = dataset.clone().to_quads()?;
                let focus = dataset.clone().id;
                match dataset.clone().organisms {
                    Some(data) => {
                        for organism in data {
                            let organism_quads: Vec<Quad<Id, IriBuf, Term>> =
                                organism.to_quads()?;
                            for mut org_quads in organism_quads {
                                org_quads.0 = rdf_types::Id::Iri(focus.clone());
                                match org_quads.2.clone() {
                                    rdf_types::Term::Literal(Literal { value: l, type_: _ }) => {
                                        if !l.is_empty() {
                                            quads.push(org_quads.to_owned());
                                        }
                                    }
                                    rdf_types::Term::Id(_) => todo!(),
                                }
                            }
                        }
                    }
                    None => {
                        warn!("focus {focus} has no associated taxa data.")
                    }
                }
                // let organism_quads = dataset.organisms.to_quads()?;
                for q in quad_data.iter() {
                    quads.push(q.to_owned());
                }
            }
            Ok(quads)
        }
    }

    impl ToRDF for Organism {
        /// Serialise an OmicsDi organism to quads.
        fn to_quads(self) -> Result<Vec<Quad<Id, IriBuf, Term>>, IntoQuadsError> {
            let quads = linked_data::to_quads(rdf_types::generator::Blank::new(), &self)?;
            Ok(quads)
        }
    }
}

// pub mod searching {
//     use std::{
//         pin::Pin,
//         sync::WaitTimeoutResult,
//         task::{Context, Poll},
//     };

//     use crate::biordf::omicsdi::{
//         api::{Search, SearchBuilder, SearchBuilderError, SearchError},
//         data::{self, OmicsDiResponse},
//     };
//     use async_trait::async_trait;

//     use futures::{
//         future::BoxFuture,
//         stream::{Stream, StreamExt},
//         FutureExt,
//     };
//     use rdf_types::dataset::DatasetMut;
//     use thiserror::Error;
//     #[derive(Debug, Error)]
//     pub enum PagerError {
//         #[error("Request failed: {0}")]
//         Request(#[from] reqwest::Error),

//         #[error("Failed to parse response: {0}")]
//         Parse(#[from] serde_json::Error),

//         #[error("API returned an error: {0}")]
//         Api(String), // Captures errors from APIs

//         #[error("Pagination limit exceeded. Start: {start}, Total Hits: {total_hits}")]
//         PaginationLimit { start: i32, total_hits: i32 },
//     }
//     /// For API methods that have a known size, and collect up to a max of the total size
//     pub trait Pageable {
//         fn max_size(&self) -> i32;
//         fn total_hits(&self) -> Result<i32, PagerError>;
//     }

//     impl Pageable for Search {
//         /// Retrieve the max hits that can be retrieved in one go.
//         fn max_size(&self) -> i32 {
//             Self::MAX_REQUEST_SIZE
//         }

//         /// Ask for the total amount of hits.
//         fn total_hits(&self) -> Result<i32, PagerError> {
//             self.total_hit()
//                 .map_err(|arg0: SearchError| PagerError::Api(arg0.to_string()))
//         }
//     }
// }

// #[cfg(test)]
// mod tests {
//     use std::error::Error;

//     use iref::IriBuf;

//     use crate::biordf::core::searching::Pageable;
//     use crate::biordf::omicsdi::{api::SearchBuilder, data::OmicsDiResponse};

//     #[test]
//     fn test_paging() -> Result<(), Box<dyn Error>> {
//         let mut x = SearchBuilder::default();
//         let q: String = "E-GEOD-5003".into();
//         // This is invalid and should not be allowed.
//         let r = x.query(q.to_owned()).build().unwrap();
//         let out = r.total_hits().await?;
//         assert_eq!(out, 1);

//         Ok(())
//     }
//     use futures::stream::StreamExt;

//     #[test]
//     #[ignore = "No paging capacities yet"]
//     fn test_paging_stream() -> Result<(), Box<dyn std::error::Error>> {
//         let mut binding = SearchBuilder::default();
//         let mut search = binding.query("Fish".to_owned());
//         let search_max = 10;
//         // let pager = SearchPager::new(&mut search, search_max)?;

//         let mut stream = pager.boxed();
//         let mut res: Vec<OmicsDiResponse> = Vec::new();
//         while let Some(result) = stream.next() {
//             let results = result?;
//             dbg!(&results);
//             res.push(results);
//         }
//         dbg!(res);

//         Ok(())
//     }

//     #[test]
//     fn test_basic_traits() -> Result<(), Box<dyn Error>> {
//         let mut x = SearchBuilder::default();
//         let q: String = "E-GEOD-5003".into();
//         let query_1 = x.query(q.clone()).build()?;
//         let query_2 = x.query(q.clone()).start(2).size(20).build()?;

//         assert!(query_1 < query_2, "Check if the default is oke");
//         assert!(query_1.max_size() == 1_000, "The max should be oke");

//         let query_1 = x.query(q.clone()).start(2).size(20).build()?;
//         let query_2 = x.query(q).start(2).size(20).build()?;
//         assert!(query_1 == query_2, "equal queries");

//         let query_1 = x.query("".into()).start(2).size(200).build()?;
//         let query_2 = x.query("".into()).start(2).size(20).build()?;
//         assert!(query_1 != query_2, "equal queries");

//         let query_1 = x.query("".into()).start(2).size(20).build()?;
//         let query_2 = x
//             .query("".into())
//             .start(2)
//             .size(20)
//             .facet_size(10)
//             .build()?;
//         assert!(query_1 != query_2, "equal queries");

//         let query_1 = x.query("".into()).start(2).size(20).build()?;
//         let query_2 = x.query("".into()).start(3).size(20).build()?;
//         assert!(query_1 != query_2);
//         assert!(query_1 < query_2);

//         let query_1 = x.query("".into()).start(2).size(200).build()?;
//         let query_2 = x.query("".into()).start(3).size(20).build()?;
//         assert!(query_1 != query_2);
//         assert!(query_1 < query_2);

//         let query_1 = x.query("".into()).start(1000).size(20).build()?;
//         let query_2 = x.query("".into()).start(3).size(20).build()?;
//         assert!(query_1 != query_2);
//         assert!(query_1 > query_2);

//         let query_1 = x.query("".into()).start(1000).size(200).build()?;
//         let query_2 = x.query("".into()).start(3).size(20).build()?;
//         assert!(query_1 != query_2);
//         assert!(query_1 > query_2);
//         Ok(())
//     }

//     use rdf_types::{dataset::DatasetView, static_iref::iri};
//     #[test]
//     fn test_ld() -> () {
//         #[derive(linked_data::Serialize, linked_data::Deserialize)]
//         #[ld(prefix("ex" = "http://example.org/"))]
//         struct Foo {
//             #[ld(id)]
//             id: IriBuf,

//             #[ld("ex:name")]
//             name: String,

//             #[ld("ex:email")]
//             email: String,

//             #[ld("ex:numbers")]
//             numbers: Vec<i64>,
//             #[ld("ex:maybe")]
//             maybe: Option<String>,
//             #[ld("ex:alot")]
//             alot: Vec<Nested>,
//         }

//         #[derive(linked_data::Serialize, linked_data::Deserialize)]
//         #[ld(prefix("ex" = "http://example.org/"))]
//         #[ld(type = "ex:object")]
//         struct Nested {
//             #[ld("ex:num")]
//             m: i64,
//         }

//         let _value = Foo {
//             id: iri!("http://example.org/JohnSmith").to_owned(),
//             name: "John Smith".to_owned(),
//             email: "john.smith@example.org".to_owned(),
//             numbers: vec![1, 133],
//             maybe: Some("S".into()),
//             alot: vec![Nested { m: 10 }],
//         };

//         // for quad in quads {
//         //     use rdf_types::RdfDisplay;
//         //     println!("{} .", quad.rdf_display())
//         // }
//     }
// }
