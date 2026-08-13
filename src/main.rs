mod huffman;

use crate::huffman::huffman;

fn main() {

    let path = String::from("teste.txt");
    
    huffman(&path);

}
