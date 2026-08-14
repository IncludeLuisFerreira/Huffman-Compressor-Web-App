mod bit_io;
mod huff_file;
mod tree;

use crate::huffman::{bit_io::BitWriter, huff_file::escrever_arquivo_huff, tree::Node};
use std::collections::BinaryHeap;
use std::fs::read_to_string;

fn calcular_frequencia(conteudo: &str, tabela: &mut [usize; 256]) {
    for letra in conteudo.bytes() {
        tabela[letra as usize] += 1;
    }
}

fn criar_node(tabela: [usize; 256]) -> BinaryHeap<Node> {
    let mut nodes = BinaryHeap::new();

    for (byte, &freq) in tabela.iter().enumerate() {
        if freq > 0 {
            nodes.push(Node::Folha {
                caracter: byte as u8,
                freq,
            });
        }
    }

    nodes
}

pub fn huffman(path_to_file: &str) -> std::io::Result<()> {
    let content = read_to_string(path_to_file)?;

    let mut tabela = [0usize; 256];

    calcular_frequencia(&content, &mut tabela); // Preenche a tabela de frequencia

    let mut nodes = criar_node(tabela); // Cria node e coloca eles na binaryheap

    // Agrupa os dois menores nós e insere o novo nó na fila
    while nodes.len() > 1 {
        let left = nodes.pop().unwrap();
        let right = nodes.pop().unwrap();
        let freq_total = left.freq() + right.freq();

        let novo_no = Node::Interno {
            freq: freq_total,
            left: Box::new(left),
            right: Box::new(right),
        };
        nodes.push(novo_no);
    }

    if let Some(raiz) = nodes.pop() {
        let mut dicionario: [Option<String>; 256] = std::array::from_fn(|_| None);

        tree::gerar_codigo(&raiz, String::new(), &mut dicionario);

        let mut writer = BitWriter::new();

        for byte in content.bytes() {
            if let Some(codigo) = &dicionario[byte as usize] {
                writer.escrever_codigo(codigo);
            }
        }

        let payload: Vec<u8> = writer.finalizar();
        let mut huff_data = Vec::new();

        let simbolos_unicos = tabela.iter().filter(|&&f| f > 0).count() as u16;

        huff_data.extend_from_slice(&simbolos_unicos.to_le_bytes());

        for (byte, &freq) in tabela.iter().enumerate() {
            if freq > 0 {
                huff_data.push(byte as u8);
                huff_data.extend_from_slice(&(freq as u32).to_le_bytes());
            }
        }

        huff_data.extend(payload);

        escrever_arquivo_huff(path_to_file, &huff_data)?;
    }

    Ok(())
}
