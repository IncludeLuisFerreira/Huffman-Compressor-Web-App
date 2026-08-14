use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

pub fn escrever_arquivo_huff(path_to_file: &str, huff_bytes: &[u8]) -> io::Result<()> {
    if !Path::new(path_to_file).exists() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("Arquivo não encontrado: {}", path_to_file),
        ));
    }

    let output_path = format!("{}.huff", path_to_file);
    fs::write(&output_path, huff_bytes)?;

    Ok(())
}
