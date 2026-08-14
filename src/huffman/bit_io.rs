
pub struct BitWriter {
    buffer: u8,
    bits_count: u8,
    pub output: Vec<u8>
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter {
            buffer: 0,
            bits_count: 0,
            output: Vec::new()
        }
    }

    pub fn escrever_bits(&mut self, bit: char) {
        self.buffer <<=  1;
        
        if bit == '1' {
            self.buffer |= 1;
        }

        self.bits_count += 1;

        if self.bits_count == 8 {
            self.output.push(self.buffer);
            self.buffer = 0;
            self.bits_count = 0;
        }
    }

    pub fn escrever_codigo(&mut self, codigo: &str) {
        for bit in codigo.chars() {
            self.escrever_bits(bit);
        }
    }

    pub fn finalizar(mut self) -> Vec<u8> {
        if self.bits_count > 0 {
            self.buffer <<= 8 - self.bits_count;
            self.output.push(self.buffer);
        }
        self.output
    }
}