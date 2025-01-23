use clap::Parser;
use rdfbio::biordf::{
    core::data::{dump_quads, ToRDF},
    omicsdi::api::SearchBuilder,
};
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input data. For fasta files, a chosen distance metric is calculated.
    #[arg(name = "Search data")]
    input: String,
}

// You should have a yaml document for the parameters of the code.
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut binding = SearchBuilder::default();
    let x = binding.size(100);
    let q: String = args.input;
    print!("{}", q);
    let query = x.query(q).build().unwrap();
    let results = query.search().await.unwrap();
    let quads = results.to_quads().unwrap();
    let _ = dump_quads(quads);
    // let json = serde_json::to_string(&results).unwrap();
    // print!("{}", json);
}
