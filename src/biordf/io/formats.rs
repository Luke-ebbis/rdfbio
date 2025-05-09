/// A module to save serde files as a CSV


pub mod csv {

    use std::error::Error;
    use std::io::Write;
    use std::{fs::OpenOptions, process::Output};
    use std::io;

    use crate::biordf::api::omicsdi::data::OmicsDiResponse;



    pub fn write_omicsdi(object: OmicsDiResponse, out: Option<String>) -> Result<(), Box<dyn Error>> {
        let writer: Box<dyn Write> = match out {
            Some(name) => {
                let mut file = OpenOptions::new().write(true).create(true).open(&name).expect(&format!("Cannot open {}", name));
                Box::new(file)
            },
            None => {
                Box::new(io::stdout())
            }};
        let mut wtr = csv::WriterBuilder::new().from_writer(writer);

        match object.datasets {
            Some(dataset) => {
                for record in dataset {
                    let organisms = match &record.organisms {
                        Some(vec) => vec.iter().map(|o| o.name.clone()).collect::<Vec<_>>().join(":"),
                        None => String::new(),
                    };

                    wtr.write_record(&[
                        &record.id.to_string(),
                        &record.title.unwrap_or_default(),
                        &record.description.unwrap_or_default(),
                        &organisms,
                    ])?;
                }
                wtr.flush()?;
                ()
            },
            None => ()
        }
        Ok(())
    }

}
