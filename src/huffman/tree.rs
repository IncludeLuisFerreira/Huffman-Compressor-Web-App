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
    pub fn new_folha(caracter: u8, freq: usize) -> Self {
        Node::Folha { caracter, freq }
    }

    pub fn new_interno(freq: usize, left: Box<Node>, right: Box<Node>) -> Self {
        Node::Interno { freq, left, right }
    }

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
