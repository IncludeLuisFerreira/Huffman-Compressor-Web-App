use std::cmp::Ordering;

#[derive(Debug, Eq, PartialEq)] // Eq e PartialEq são necessárias para Ord
pub enum Node {
    Folha {
        caracter: u8,
        freq: usize,
    },
    Interno {
        freq: usize,
        left: Box<Node>,
        right: Box<Node>,
    },
}

impl Node {
    pub fn freq(&self) -> usize {
        match self {
            Node::Folha { freq, .. } => *freq,
            Node::Interno { freq, .. } => *freq,
        }
    }
}

// Ordem inversa: quanto menor a freq maior na heap
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.freq().cmp(&self.freq())
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn gerar_codigo(raiz: &Node, caminho_atual: String, dicionario: &mut [Option<String>; 256]) {
    match raiz {
        Node::Folha { caracter, .. } => {
            dicionario[*caracter as usize] = Some(caminho_atual);
        }
        Node::Interno { left, right, .. } => {
            gerar_codigo(left, format!("{}0", caminho_atual), dicionario);

            gerar_codigo(right, format!("{}1", caminho_atual), dicionario);
        }
    }
}
