# 🗜️ Huffman Compressor Web App

![Status](https://img.shields.io/badge/status-funcional-brightgreen?style=for-the-badge)
![Licença](https://img.shields.io/badge/licença-MIT-green?style=for-the-badge)

> **Compressão de arquivos de texto com o algoritmo de Huffman, direto no navegador.**

App web onde você envia um arquivo de texto, o backend em Rust comprime com o algoritmo de
Huffman e devolve o arquivo `.huff` para download — com estatísticas de redução. Hospedado
no [Shuttle](https://www.shuttle.dev/).

---

## 🚀 Sobre o Projeto

O **Huffman Compressor** permite compactar arquivos de texto sem instalar nada. O algoritmo
é implementado do zero em Rust: contagem de frequência por byte, construção da árvore de
Huffman, geração dos códigos e empacotamento dos bits.

O arquivo é enviado ao backend, processado **em memória** (nada é gravado em disco) e o
resultado `.huff` é devolvido para o download. O formato de arquivo é:

```
[ nº de símbolos u16 ] + ([ byte ] + [ frequência u32 ] × n) + payload de bits
```

## ✨ Funcionalidades

- 📂 Upload de arquivos de texto (arrastar e soltar ou clicar para escolher)
- 🗜️ Compressão usando codificação de Huffman implementada do zero em Rust
- ⬇️ Download do arquivo compactado `.huff`
- 📊 Estatísticas: tamanho original, tamanho compactado e % de redução
- 🧪 Validações: arquivo deve ser texto UTF-8 válido, limite de 5 MB por arquivo
- 📱 Interface responsiva com tema terminal/neon (canvas Matrix, árvore binária animada)

## 🎨 Interface

A página mantém a estética imersiva de terminal com:

- **Árvore binária animada** com nós pulsantes e arestas tracejadas
- **Bits binários caindo** em estilo Matrix (canvas)
- **Efeito de digitação** no subtítulo
- **Barra de progresso** durante a compressão
- **Suporte a `prefers-reduced-motion`**

## 🛠️ Tecnologias

- **Backend:** Rust, [axum](https://github.com/tokio-rs/axum), [tokio](https://tokio.rs)
- **Hospedagem:** [Shuttle](https://www.shuttle.dev/) (deploy gratuito)
- **Frontend:** HTML5, CSS3, JavaScript puro (sem build)
- Fonte [Share Tech Mono](https://fonts.google.com/specimen/Share+Tech+Mono)

## 📦 Como Rodar Localmente

Pré-requisitos: Rust (stable) e, para rodar com o Shuttle, a CLI
(`cargo install cargo-shuttle`).

1. Clone o repositório:
   ```bash
   git clone <url-do-repositorio>
   cd huffman-web-app
   ```

2. Rode o servidor:
   ```bash
   cargo shuttle run
   ```

3. Abra `http://localhost:8000` no navegador.

Também funciona com `cargo run`, já que o app usa o macro `#[shuttle_runtime::main]`.

## 🚀 Deploy no Shuttle (plano gratuito)

```bash
cargo shuttle login        # autentica no navegador
cargo shuttle project start # cria o projeto e escolhe o nome
cargo shuttle deploy        # publica o app
```

O app fica disponível em `https://<nome-do-projeto>.shuttle.app`.

> **Limites do plano gratuito:** 128 MB de RAM e o app "dorme" após inatividade
> (o primeiro acesso tem um leve atraso — cold start).

## 🔮 Roadmap

- 📖 Descompressão de arquivos `.huff`
- 🗃️ Suporte a arquivos binários (imagens, PDFs, etc.)
- 🌳 Visualização interativa da árvore de Huffman gerada
- 📦 Aumentar o limite de upload

## 🧪 Testes

```bash
cargo test        # unit tests do núcleo Huffman e do handler HTTP
cargo clippy      # lint
cargo fmt --check # formatação
```

## 📄 Licença

MIT
