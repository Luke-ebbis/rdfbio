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
                                        type_: _,
                                    }) => {
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

    use crate::biordf::omicsdi::{
        api::{Search, SearchBuilder, SearchBuilderError, SearchError},
        data::{self, OmicsDiResponse},
    };

    pub enum Endpoint<'a, T>
    where
        T: Pageable,
    {
        OmicsDi(&'a T),
    }

    #[derive(Debug, Clone)]
    pub enum SearchSize {
        Amount(i32),
        All,
    }

    #[derive(Debug, Clone)]
    pub struct Pager<'a, T>
    where
        T: Pageable,
    {
        pub search: &'a T,
        size: SearchSize,
    }

    impl<'a, T> Pager<'a, T>
    where
        T: Pageable,
    {
        pub fn new(
            search: &'a T,
            size: SearchSize,
        ) -> Pager<T> {
            Pager {
                search: &search,
                size: size,
            }
        }

        pub(crate) fn into_iter(
            &'a self
        ) -> Result<PagerIterator<'a, T>, PagerError> {
            let target = match &self.size {
                SearchSize::Amount(i) => i.clone(),
                SearchSize::All => self.search.total_hits().unwrap(),
            };
            let step = self.search.max_size().unwrap();
            if step <= target {
                let step = target;
            }
            dbg!(step, target);
            Ok(PagerIterator {
                pages: self,
                index: 0,
                step_size: step,
                end_index: target,
            })
        }
    }

    pub struct PagerIterator<'a, T>
    where
        T: Pageable,
    {
        pages: &'a Pager<'a, T>,
        index: i32,
        step_size: i32,
        end_index: i32,
    }

    impl<'a, T> Iterator for PagerIterator<'a, T>
    where
        T: Pageable + Clone,
    {
        type Item = T; //&'a Pager<'a, T>;

        fn max(self) -> Option<Self::Item>
        where
            Self: Sized,
            Self::Item: Ord,
        {
            todo!();
        }

        fn next(&mut self) -> Option<Self::Item> {
            if self.index as i32 <= self.end_index {
                self.index += self.step_size;
                let new =
                    self.pages.search.clone().set_start(self.index).clone();
                Some(new)
            } else {
                None
            }
        }
    }

    use thiserror::Error;
    #[derive(Debug, Error)]
    pub enum PagerError {
        #[error("Request failed: {0}")]
        Request(#[from] reqwest::Error),

        #[error("Failed to parse response: {0}")]
        Parse(#[from] serde_json::Error),

        #[error("Build returned an error: {0}")]
        BuildError(String), // Captures errors from APIs

        #[error("API returned an error: {0}")]
        Api(String), // Captures errors from APIs

        #[error("Pagination limit exceeded. Start: {start}, Total Hits: {total_hits}")]
        PaginationLimit { start: i32, total_hits: i32 },
    }
    /// For API methods that have a known size, and collect up to a max of the total size
    pub trait Pageable {
        fn total_hits(&self) -> Result<i32, PagerError>;
        fn set_start(
            &self,
            start: i32,
        ) -> Self;
        fn max_size(&self) -> Result<i32, PagerError>;
        fn perform(
            &self,
            start: i32,
            size: i32,
        ) -> Result<OmicsDiResponse, PagerError>;
    }

    impl Pageable for SearchBuilder<'_> {
        /// Retrieve the max hits that can be retrieved in one go.

        /// Ask for the total amount of hits.
        fn total_hits(&self) -> Result<i32, PagerError> {
            let search = self.build().map_err(|x: SearchBuilderError| {
                PagerError::BuildError(x.to_string())
            })?;
            search
                .total_hit()
                .map_err(|arg0: SearchError| PagerError::Api(arg0.to_string()))
        }

        fn perform(
            &self,
            start: i32,
            size: i32,
        ) -> Result<OmicsDiResponse, PagerError> {
            let r = self
                .clone()
                .start(start)
                .size(size)
                .build()
                .unwrap()
                .search()
                .map_err(|x: SearchError| PagerError::Api(x.to_string()))?;
            Ok(r)
        }

        fn max_size(&self) -> Result<i32, PagerError> {
            let s = self
                .build()
                .map_err(|x| PagerError::BuildError(x.to_string()))?
                .size;
            Ok(s)
        }

        fn set_start(
            &self,
            start: i32,
        ) -> Self {
            self.to_owned().start(start).to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use iref::IriBuf;

    use crate::biordf::core::searching::{
        Endpoint, Pageable, Pager, PagerIterator, SearchSize,
    };

    use crate::biordf::omicsdi::{api::SearchBuilder, data::OmicsDiResponse};
    use std::iter::IntoIterator;

    #[test]
    // #[ignore = "paging not yet implementedn"]
    fn test_paging() -> Result<(), Box<dyn Error>> {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        // This is invalid and should not be allowed.
        let r = x.query(&q).size(100);
        let out = r.total_hits()?;
        let x = r;
        let pager: Pager<SearchBuilder> =
            Pager::new(x, SearchSize::Amount(500));
        let mut page = pager.into_iter().unwrap();
        for p in page {
            dbg!(p.build()?.start, p.build()?.size,);
        }

        Ok(())
    }

    #[test]
    fn test_paging_max_size() -> Result<(), Box<dyn Error>> {
        let size = 20;
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        let query = x.query(&q).start(2).size(size);
        assert!(query.max_size()? == size);

        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        let query = x.query(&q).start(2);
        assert!(query.total_hits()? == 1);

        Ok(())
    }

    #[test]
    fn test_basic_traits() -> Result<(), Box<dyn Error>> {
        let mut x = SearchBuilder::default();
        let q: String = "E-GEOD-5003".into();
        let query_1 = x.query(&q).build()?;
        let query_2 = x.query(&q).start(2).size(20).build()?;

        assert!(query_1 < query_2, "Check if the default is oke");

        let query_1 = x.query(&q).start(2).size(20).build()?;
        let query_2 = x.query(&q).start(2).size(20).build()?;
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

    use rdf_types::{dataset::DatasetView, static_iref::iri};
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
