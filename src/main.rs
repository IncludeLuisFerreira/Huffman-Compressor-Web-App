mod huffman;

use crate::huffman::huffman;

fn main() {
    let path = "teste.txt";

    if let Err(error) = huffman(path) {
        eprintln!("Erro: {}", error);
        std::process::exit(1);
    }
}
