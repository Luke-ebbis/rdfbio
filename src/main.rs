use core::panic;
use std::any::Any;

use clap::ValueEnum;
use clap::{Parser, Subcommand};
use rdf_types::dataset::BTreeDataset;
use rdfbio::biordf::core::searching::{Pageable, Pager};
use rdfbio::biordf::omicsdi::data::{self, DataSet, OmicsDiResponse};
use rdfbio::biordf::{
    core::data::{dump_quads, ToRDF},
    omicsdi::api::SearchBuilder,
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Ttl,
    Json,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Subcommands for rdfbio
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Query OmicsDI endpoint
    Query {
        /// The search query (required)
        #[arg()]
        query: String,

        /// The size of the results to fetch
        #[arg(short = 'S', long, default_value = "10")]
        size: usize,

        /// The start index for the search results
        #[arg(short = 's', long, default_value = "0")]
        start: usize,

        /// Output format: RDF (ttl) or JSON (json). The JSON retrieves the raw database hits, the ttl retrieves the linked data as interepreted by this program.
        #[arg(value_enum, short, long, default_value_t = OutputFormat::Json)]
        format: OutputFormat,

        /// Output file (optional). If not provided, prints to stdout.
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Query {
            query,
            size,
            start,
            format,
            output,
        } => {
            let mut binding = SearchBuilder::default();
            let size: i32 = size.try_into().expect("Size too large for i32");
            let start: i32 =
                start.try_into().expect("Start value too large for i32");
            let search_size = {
                if size > 1000 {
                    1000 - 1
                } else {
                    size
                }
            };
            let mut builder = binding.clone();
            let var_name = query.clone();
            let query_builder =
                binding.size(search_size).start(start).query(&var_name);
            let mut pager: Pager<SearchBuilder> = Pager::new(
                query_builder,
                rdfbio::biordf::core::searching::SearchSize::Amount(size),
            );

            let mut out: OmicsDiResponse = builder
                .query(&query)
                .size(search_size)
                .build()
                .unwrap()
                .search()
                .unwrap();
            let mut datasets: Vec<DataSet> = Vec::new();
            for p in pager.into_iter().unwrap() {
                let ds =
                    p.build().unwrap().search().unwrap().datasets.unwrap();
                for d in ds.into_iter() {
                    datasets.push(d);
                }
            }
            out.datasets = Some(datasets);
            let results = out;
            match format {
                OutputFormat::Ttl => {
                    let quads = results.to_quads().unwrap();
                    if let Some(file) = output {
                        std::fs::write(file, dump_quads(quads.to_owned()))
                            .unwrap();
                    } else {
                        println!("{}", dump_quads(quads.to_owned()));
                    }
                }
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&results).unwrap();
                    if let Some(file) = output {
                        std::fs::write(file, json).unwrap();
                    } else {
                        println!("{}", json);
                    }
                }
            }
        }
    }
}
