mod bit_io;
mod tree;

use crate::huffman::{bit_io::BitWriter, tree::Node};
use std::collections::BinaryHeap;

fn calcular_frequencia(conteudo: &[u8], tabela: &mut [usize; 256]) {
    for &byte in conteudo {
        tabela[byte as usize] += 1;
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

pub fn compactar_bytes(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    if bytes.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "O arquivo está vazio.",
        ));
    }

    let mut tabela = [0usize; 256];
    calcular_frequencia(bytes, &mut tabela);

    let mut nodes = criar_node(tabela);

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

    let raiz = nodes.pop().unwrap();
    let mut dicionario: [Option<String>; 256] = std::array::from_fn(|_| None);
    tree::gerar_codigo(&raiz, String::new(), &mut dicionario);

    let mut writer = BitWriter::new();
    for &byte in bytes {
        if let Some(codigo) = &dicionario[byte as usize] {
            writer.escrever_codigo(codigo);
        }
    }

    let payload = writer.finalizar();
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

    Ok(huff_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ler_metadados(huff: &[u8]) -> (u16, Vec<(u8, u32)>, &[u8]) {
        let n = u16::from_le_bytes([huff[0], huff[1]]) as usize;
        let mut tabela = Vec::with_capacity(n);
        let mut offset = 2;
        for _ in 0..n {
            let byte = huff[offset];
            let freq = u32::from_le_bytes(huff[offset + 1..offset + 5].try_into().unwrap());
            tabela.push((byte, freq));
            offset += 5;
        }
        (n as u16, tabela, &huff[offset..])
    }

    #[test]
    fn compactar_texto_conhecido_gera_cabecalho_correto() {
        let huff = compactar_bytes(b"abracadabra").unwrap();
        let (n, tabela, payload) = ler_metadados(&huff);

        assert_eq!(n, 5);
        assert_eq!(
            tabela,
            vec![(b'a', 5), (b'b', 2), (b'c', 1), (b'd', 1), (b'r', 2)]
        );
        assert!(!payload.is_empty());
    }

    #[test]
    fn compactar_arquivo_vazio_retorna_erro() {
        assert!(compactar_bytes(b"").is_err());
    }

    #[test]
    fn compactar_simbolo_unico_gera_cabecalho_valido() {
        let huff = compactar_bytes(b"aaaa").unwrap();
        let (n, tabela, payload) = ler_metadados(&huff);

        assert_eq!(n, 1);
        assert_eq!(tabela, vec![(b'a', 4)]);
        assert!(payload.is_empty());
    }

    #[test]
    fn compactar_texto_varios_caracteres_soma_frequencias() {
        let texto = "Huffman com acentuação: ção á é í ó ú 😀".repeat(50);
        let huff = compactar_bytes(texto.as_bytes()).unwrap();
        let (n, tabela, payload) = ler_metadados(&huff);

        assert_eq!(n as usize, tabela.len());
        assert!(!payload.is_empty());

        let total: u32 = tabela.iter().map(|&(_, f)| f).sum();
        assert_eq!(total as usize, texto.len());
    }
}
