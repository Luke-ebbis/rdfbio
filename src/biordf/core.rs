/// identfiers endpoint
///
/// Here I write a method to crosslink new identifiers from OmicsDi ones. A identifier is
/// in the form: `http://identifiers.org/<source>:<identifier>`.
/// Identifiers has a sparql endpoint: http://sparql.api.identifiers.org/
pub mod identifiers {
    use core::fmt;

    // TODObefore linking; check your work.
    /// The databases that link between OmicsDI and identifiers.org
    /// Right now the only check that is made is whether the produced identifier
    /// is able to resolve to something.
    #[derive(Clone, Copy)]
    pub enum Databases<'a> {
        /// When the `source` = project. You remove keep the whole identifier.
        BioProject(&'a str),
        /// For the pride database, pride:035768.
        Pride(&'a str),
    }

    use reqwest::{Request, StatusCode};
    use std::error::Error;

    use crate::biordf::omicsdi::api::SearchError;

    impl Databases<'_> {
        const URL: &'static str = "http://identifiers.org/";
        fn new<'a>(namespace: &'a str, id: &'a str) -> Databases<'a> {
            match namespace {
                "pride" => Databases::Pride(id),
                "project" => Databases::BioProject(id),
                _ => todo!(),
            }
        }

        fn get_id(self) -> String {
            match self {
                Self::BioProject(id) => id.to_owned(),
                Self::Pride(id) => id.to_owned(),
            }
        }

        fn namespace(self) -> &'static str {
            match self {
                Databases::BioProject(id) => "bioproject",
                Databases::Pride(id) => "pride.project",
            }
        }

        fn to_identifier(&self) -> Result<String, SearchError> {
            let db_string = self.namespace();
            // let identifiers_check = check_identifier_namespace(&db_string)?;
            let id = format!("{}{}:{}", Self::URL, self.namespace(), self.get_id());
            let identifer_check = check_identifier_resolving(&id)?;
            match identifer_check {
                true => Ok(id),
                false => Err(SearchError::RequestFailed(
                    StatusCode::NOT_FOUND,
                    format!("This identifier does not resolve! {id}"),
                )),
            }
        }
    }

    impl fmt::Display for Databases<'_> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let out: &str = match &self {
                Databases::BioProject(id) => &format!("bioproject:{}", id),
                Databases::Pride(id) => &format!("pride:{}", id),
            };

            write!(f, "{}", out)
        }
    }

    //TODO impl to string
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error with the search itself.
    fn check_identifier_resolving(x: &str) -> Result<bool, SearchError> {
        let url = x;
        let url =
            reqwest::Url::parse(url).map_err(|e| SearchError::UrlParseFailed(e.to_string()))?;
        let client = reqwest::blocking::Client::new();
        let response = client.get(url).send().map_err(|_| {
            SearchError::RequestFailed(
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to send request".into(),
            )
        })?;

        let status = response.status();
        match status {
            reqwest::StatusCode::OK => Ok(true),
            reqwest::StatusCode::NOT_FOUND => Ok(false),
            _ => Err(SearchError::RequestFailed(
                status.clone(),
                format!("Request failed {:?}", status.canonical_reason()),
            )),
        }
    }

    #[test]
    fn test_identifiers_not_found() -> Result<(), Box<dyn Error>> {
        let identifier = Databases::new("project", "PRJ558612");
        let id = identifier.to_identifier();
        match id {
            Err(x) => assert_eq!(
                x.to_string(),
                SearchError::RequestFailed(
                    StatusCode::from_u16(404)?,
                    "This identifier does not resolve! http://identifiers.org/bioproject:PRJ558612"
                        .to_owned()
                )
                .to_string()
            ),
            Ok(_) => panic!("this test should fail"),
        }

        Ok(())
    }
    #[test]
    fn test_identifiers() -> Result<(), Box<dyn Error>> {
        let identifier = Databases::new("project", "PRJNA558612");
        let string = identifier.to_identifier()?;
        assert_eq!("http://identifiers.org/bioproject:PRJNA558612", string);

        let identifier = Databases::new("pride", "PXD001416");
        let string = identifier.to_identifier()?;
        assert_eq!("http://identifiers.org/pride.project:PXD001416", string);

        Ok(())
    }
}

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
                        // this part removes the empty taxon slots.
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

pub mod searching {

    use std::clone;

    use crate::biordf::omicsdi::{
        api::{SearchBuilder, SearchBuilderError, SearchError},
        data::OmicsDiResponse,
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
        pub fn new(search: &'a T, size: SearchSize) -> Pager<T> {
            Pager {
                search: search,
                size,
            }
        }

        // set this private pub(crate) again
        pub fn into_iter(&'a self) -> Result<PagerIterator<'a, T>, PagerError> {
            // Set the target size to the total amount of hits.
            let maximum_hits = self.search.total_hits()?;
            let target = match &self.size {
                SearchSize::Amount(i) => *i,
                SearchSize::All => maximum_hits,
            };
            let step = self.search.max_size().unwrap();
            if step <= target {
                let step = target;
            }
            let start = self.search.get_start()?;
            Ok(PagerIterator {
                pages: self,
                index: start,
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

    impl<T> Iterator for PagerIterator<'_, T>
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
            if self.index <= self.end_index {
                let start = self.index;
                self.index += self.step_size;
                let end = self.index + self.step_size;
                dbg!(start, end, self.step_size);
                let new = self
                    .pages
                    .search
                    .clone()
                    .set(start, end, self.step_size)
                    .clone();
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
        fn get_start(&self) -> Result<i32, PagerError>;
        fn total_hits(&self) -> Result<i32, PagerError>;
        fn set(self, start: i32, end: i32, step: i32) -> Self;
        fn max_size(&self) -> Result<i32, PagerError>;
        fn perform(&self) -> Result<OmicsDiResponse, PagerError>;
    }

    impl Pageable for SearchBuilder<'_> {
        /// Retrieve the max hits that can be retrieved in one go.

        /// Ask for the total amount of hits.
        fn total_hits(&self) -> Result<i32, PagerError> {
            let search = self
                .build()
                .map_err(|x: SearchBuilderError| PagerError::BuildError(x.to_string()))?;
            dbg!("total");
            search
                .total_hit()
                .map_err(|arg0: SearchError| PagerError::Api(arg0.to_string()))
        }

        fn perform(&self) -> Result<OmicsDiResponse, PagerError> {
            let r = self
                .clone()
                .start(self.get_start()?)
                .size(self.max_size()?)
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

        fn set(self, start: i32, end: i32, step: i32) -> Self {
            dbg!(format!("setting {start} and {end} using {step}"));
            self.to_owned().start(start).size(end).to_owned()
        }

        fn get_start(&self) -> Result<i32, PagerError> {
            let v = self
                .build()
                .map_err(|x| PagerError::BuildError(x.to_string()))?
                .start;
            Ok(v)
        }
    }

    pub fn page(pager: Pager<SearchBuilder>) -> Result<OmicsDiResponse, PagerError> {
        let mut first_search = pager.clone().search.perform()?;
        let mut datasets: Vec<crate::biordf::omicsdi::data::DataSet> = Vec::new();
        for p in pager.into_iter()? {
            let partial = p.perform()?.datasets.unwrap();
            datasets.extend(partial);
        }
        first_search.datasets.clone().unwrap().extend(datasets);
        Ok(first_search)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use iref::IriBuf;

    use crate::biordf::core::searching::{Endpoint, Pageable, Pager, PagerIterator, SearchSize};

    use crate::biordf::omicsdi::{api::SearchBuilder, data::OmicsDiResponse};
    use std::iter::IntoIterator;

    #[test]
    fn test_paging_api() -> Result<(), Box<dyn Error>> {
        // The into iter needs to check for total results also
        let mut x = SearchBuilder::default();
        let q: String = "Fish".into();
        let query_builder = x.query(&q).size(25);
        let pager = Pager::new(query_builder, SearchSize::Amount(100));
        let result = page(pager)?;
        dbg!(result);
        Ok(())
    }

    #[test]
    /// In this test, we verify that paging works when there are enough results.
    fn test_paging() -> Result<(), Box<dyn Error>> {
        // The into iter needs to check for total results also
        let mut x = SearchBuilder::default();
        let q: String = "Fish".into();
        // This is invalid and should not be allowed.
        let r = x.query(&q).size(2).start(4);
        let out = r.total_hits()?;
        let x = r;
        let pager: Pager<SearchBuilder> = Pager::new(x, SearchSize::Amount(10));
        let page = pager.into_iter().unwrap();
        for p in page {
            let part = p.build()?;
            dbg!(part.clone().start, part.clone().size);
            let results = part.search()?;
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

    use super::searching::page;
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
