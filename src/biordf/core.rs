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
        fn to_quads(
            self
        ) -> Result<Vec<Quad<Id, IriBuf, Term>>, IntoQuadsError>;
    }

    impl ToRDF for DataSet {
        /// Serialise a Dataset to quads.
        fn to_quads(
            self
        ) -> Result<Vec<Quad<Id, IriBuf, Term>>, IntoQuadsError> {
            let quads = linked_data::to_quads(
                rdf_types::generator::Blank::new(),
                &self,
            )?;
            Ok(quads)
        }
    }

    impl ToRDF for OmicsDiResponse {
        /// Serialise an omics Di response to quads.
        /// Ignore the empty taxa slots...
        fn to_quads(
            self
        ) -> Result<Vec<Quad<Id, IriBuf, Term>>, IntoQuadsError> {
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
                                org_quads.0 =
                                    rdf_types::Id::Iri(focus.clone());
                                match org_quads.2.clone() {
                                    rdf_types::Term::Literal(Literal {
                                        value: l,
                                        type_,
                                    }) => {
                                        if l != "" {
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
        fn to_quads(
            self
        ) -> Result<Vec<Quad<Id, IriBuf, Term>>, IntoQuadsError> {
            let quads = linked_data::to_quads(
                rdf_types::generator::Blank::new(),
                &self,
            )?;
            Ok(quads)
        }
    }
}

pub mod searching {
    use crate::biordf::omicsdi::api::Search;

    /// For API methods that have a known size, and collect up to a max of the total size
    pub trait Pageable {
        /// Get the total request size for this endpoint
        fn max_size(&self) -> i64;

        /// Get the total amount of hits for a search.
        fn total_hits(&self) -> i64;
    }

    // impl Ord for Search {
    //     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    //         self.start() >=
    //     }
    // }
    impl Pageable for Search {
        fn max_size(&self) -> i64 {
            Self::MAX_REQUEST_SIZE as i64
        }

        fn total_hits(&self) -> i64 {
            todo!()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use iref::IriBuf;

    use crate::biordf::{
        core::searching::Pageable, omicsdi::api::SearchBuilder,
    };

    #[tokio::test]
    async fn test_basic_traits() -> Result<(), Box<dyn Error>> {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        let query_1 = x.query(q.clone()).build()?;
        let query_2 = x.query(q.clone()).start(2).size(20).build()?;

        assert!(query_1 < query_2, "Check if the default is oke");
        assert!(query_1.max_size() == 1_000, "The max should be oke");

        let query_1 = x.query(q.clone()).start(2).size(20).build()?;
        let query_2 = x.query(q).start(2).size(20).build()?;
        assert!(query_1 == query_2, "equal queries");

        let query_1 = x.query("".into()).start(2).size(200).build()?;
        let query_2 = x.query("".into()).start(2).size(20).build()?;
        assert!(query_1 != query_2, "equal queries");

        let query_1 = x.query("".into()).start(2).size(20).build()?;
        let query_2 = x
            .query("".into())
            .start(2)
            .size(20)
            .facet_size(10)
            .build()?;
        assert!(query_1 != query_2, "equal queries");

        let query_1 = x.query("".into()).start(2).size(20).build()?;
        let query_2 = x.query("".into()).start(3).size(20).build()?;
        assert!(query_1 != query_2);
        assert!(query_1 < query_2);

        let query_1 = x.query("".into()).start(2).size(200).build()?;
        let query_2 = x.query("".into()).start(3).size(20).build()?;
        assert!(query_1 != query_2);
        assert!(query_1 < query_2);

        let query_1 = x.query("".into()).start(1000).size(20).build()?;
        let query_2 = x.query("".into()).start(3).size(20).build()?;
        assert!(query_1 != query_2);
        assert!(query_1 > query_2);

        let query_1 = x.query("".into()).start(1000).size(200).build()?;
        let query_2 = x.query("".into()).start(3).size(20).build()?;
        assert!(query_1 != query_2);
        assert!(query_1 > query_2);
        Ok(())
    }

    use rdf_types::static_iref::iri;
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
