use clap::Parser;
use rdfbio::biordf;
use std::error::Error;
use std::path::PathBuf;
use tokio;
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
    let out = biordf::Omicsdi::DataSet::search(args.input).await.unwrap();
    print!("{:?}", out.clone().datasets.pop().unwrap().id);
}
