use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::path::PathBuf;
use anyhow::Result;
use std::io::BufReader;
use std::fs::File;

pub struct AudioPlayer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream,
            stream_handle,
        })
    }

    fn play_file(&self, path: PathBuf) -> Result<()> {
        let file = BufReader::new(File::open(path)?);

        let source = Decoder::new(file)?;

        let sink = Sink::try_new(&self.stream_handle)?;
        sink.append(source);
        
        std::thread::spawn(move || {
            sink.sleep_until_end();
        });
        
        Ok(())
    }
}