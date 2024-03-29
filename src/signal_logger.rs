use std::fs::File;
use std::io::{BufWriter, Write, self};

/// A debug structure to record audio signals from several sources so that can be examined by debug apps
pub struct SignalLogger {
    out: Option<BufWriter<File>>
}

impl SignalLogger {
    pub fn new(out_filename: &str) -> Self {
        let f = File::create(out_filename).unwrap();
        let out = Some(BufWriter::new(f));
        Self { out }
    }

    pub fn new_sink() -> Self {
        let out = None;
        Self { out }
    }

    pub fn log(&mut self, source: String, signal: &[f32]) -> io::Result<()> {
        let out = match &mut self.out {
            Some(out) => out,
            None => return Ok(())
        };
        let source_str_bytes = source.as_bytes();
        let signal_str_bytes: Vec<u8> = signal.iter().flat_map(|f| {
            let float_string = " ".to_owned() + &f.to_string();
            float_string.into_bytes()
        })
        .collect();

        out.write_all(source_str_bytes)?;
        out.write_all(&signal_str_bytes)?;
        out.write_all("\n".as_bytes())?;
        Ok(())
    }
}