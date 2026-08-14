use std::{fmt::format, format, fs, println};

pub fn escrever_arquivo_huff(path_to_file: &String, huff_bytes: &Vec<u8>) -> bool {
    match fs::read_to_string(path_to_file) {
        Ok(content) => {
            let output_path = format!("{}.huff",path_to_file);

            fs::write(&output_path, huff_bytes).expect("ERRO");

        }
        Err(_) => {
            println!("Erro");
        }
    }

    true
}