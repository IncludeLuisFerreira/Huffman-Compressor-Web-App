# Web App de Compressão Huffman — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transformar o núcleo Huffman (hoje CLI, baseado em disco) e a landing page "Em Desenvolvimento" em um web app funcional de compressão: upload de arquivo de texto → backend Rust (axum + Shuttle) comprime em memória → download do `.huff` com estatísticas.

**Architecture:** Core Huffman refatorado para operar 100% em memória (`compactar_bytes(&[u8]) -> io::Result<Vec<u8>>`), chamado por um handler axum `POST /api/compactar` (multipart, limite 6MB, validação UTF-8/vazio, erros JSON). Frontend vanilla (HTML/CSS/JS) servido por `ServeDir`, mantendo o tema neon/terminal existente. Formato `.huff` inalterado.

**Tech Stack:** Rust (edition 2024), axum 0.8 (multipart), tokio, shuttle-runtime/shuttle-axum 0.57, tower-http (ServeDir), serde_json, tower + http-body-util (testes). Frontend: HTML/CSS/JS vanilla, sem build.

**Baseline importante:** o repositório **não compila** no estado atual (`src/main.rs` usa `huffman::huffman(&content)`, que retorna `io::Result<()>`, como corpo da resposta). O plano resolve isso na Tarefa 3.

---

## Estrutura de arquivos

| Arquivo | Responsabilidade | Ação |
|---|---|---|
| `src/huffman/mod.rs` | Core de compressão em memória + unit tests | Modificar (reescrever) |
| `src/huffman/bit_io.rs` | `BitWriter` | Sem mudanças |
| `src/huffman/tree.rs` | `Node` + `gerar_codigo` | Sem mudanças |
| `src/huffman/huff_file.rs` | Gravação em disco (morta) | Remover |
| `src/web/mod.rs` | Servidor TCP antigo (morta) | Remover |
| `src/web/handlers.rs` | Servidor TCP antigo (vazia/morta) | Remover |
| `src/main.rs` | `app()` (router), handler `/api/compactar`, shuttle main + unit tests | Modificar (reescrever) |
| `src/huffman/.gitkeep` | placeholder do diretório | Manter |
| `src/web/.gitkeep` | placeholder do diretório | Remover junto |
| `static/index.html` | Página funcional (upload, resultado, erros) | Modificar (reescrever) |
| `static/app.js` | Lógica de upload/download + efeitos visuais | Modificar (reescrever) |
| `Cargo.toml` | Deps (`serde_json`) + dev-deps (`tower`, `http-body-util`) | Modificar |

---

## Task 1: Core Huffman em memória (`compactar_bytes`)

**Files:**
- Modify: `src/huffman/mod.rs` (substituir conteúdo inteiro)
- Test: `src/huffman/mod.rs` (módulo `#[cfg(test)]` inline)

- [ ] **Step 1: Escrever o novo `mod.rs` com os testes falhando**

Escreva o arquivo completo (testes `#[cfg(test)]` + implementação ainda ausente). Para TDD estrito, comece com apenas os testes e a assinatura; mas como o arquivo é pequeno, o plano fornece a versão final e você executa os testes em seguida.

```rust
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
```

- [ ] **Step 2: Executar os testes do core e ver falhas (fase TDD)**

O arquivo novo não referencia mais `huff_file`, mas o `mod huff_file;` foi removido; ainda há arquivos mortos. Rode só o core:

Run: `cargo test --bin Huffman-web-app huffman::tests`
Expected: compilar os 4 testes (o binário como um todo falha por causa de `src/main.rs`; se `cargo test` travar no erro de `main.rs`, siga para o Step 3 — a TDD purista falha aqui porque o binário inteiro não compila; o plano corrige `main.rs` na Tarefa 3).

**Nota:** como o crate inteiro não compila até a Tarefa 3, os testes de core só passarão de fato após a Tarefa 3. Se `cargo test` não conseguir compilar, avance para a Tarefa 2 e depois rode todos os testes na Tarefa 3.

- [ ] **Step 3: Rodar os testes do core**

Run: `cargo test --bin Huffman-web-app huffman::tests 2>&1`
Expected (após a Tarefa 3 destravar a compilação): `test result: ok. 4 passed`.

- [ ] **Step 4: Commit**

```bash
git add src/huffman/mod.rs
git commit -m "(refactor) Huffman core agora comprime bytes em memória (compactar_bytes)"
```

---

## Task 2: Remover código morto

**Files:**
- Delete: `src/huffman/huff_file.rs`
- Delete: `src/web/mod.rs`
- Delete: `src/web/handlers.rs`
- Delete: `src/web/.gitkeep`

- [ ] **Step 1: Remover os arquivos mortos**

```bash
git rm src/huffman/huff_file.rs src/web/mod.rs src/web/handlers.rs src/web/.gitkeep
```

- [ ] **Step 2: Verificar que o diretório `src/web` foi removido**

Run: `ls src`
Expected: listar apenas `huffman` e `main.rs` (sem `web`).

- [ ] **Step 3: Commit**

```bash
git commit -m "(chore) Remove código morto (servidor TCP antigo e gravação em disco)"
```

---

## Task 3: Handler axum + `app()` + testes do endpoint

**Files:**
- Modify: `src/main.rs` (substituir conteúdo inteiro)
- Modify: `Cargo.toml` (adicionar `serde_json` + dev-deps)

- [ ] **Step 1: Adicionar dependências**

Adicione em `Cargo.toml` (em `[dependencies]`):

```toml
serde_json = "1"
```

E no final do arquivo:

```toml
[dev-dependencies]
http-body-util = "0.1"
tower = { version = "0.5", features = ["util"] }
```

- [ ] **Step 2: Escrever o novo `src/main.rs`**

```rust
use axum::{
    extract::{DefaultBodyLimit, Multipart},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;
use tower_http::services::ServeDir;

mod huffman;

fn app() -> Router {
    Router::new()
        .nest_service("/", ServeDir::new("static"))
        .route("/api/compactar", post(compactar_handler))
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024))
}

async fn compactar_handler(
    mut multipart: Multipart,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let mut arquivo = None;

    while let Some(campo) = multipart.next_field().await.map_err(|erro| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "erro": format!("Falha ao ler o upload: {erro}") })),
        )
    })? {
        if campo.file_name().is_some() {
            arquivo = Some(campo);
            break;
        }
    }

    let campo = arquivo.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "erro": "Nenhum arquivo enviado." })),
        )
    })?;

    let file_name = campo.file_name().unwrap_or("arquivo").to_string();
    let bytes = campo.bytes().await.map_err(|erro| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "erro": format!("Falha ao ler o conteúdo: {erro}") })),
        )
    })?;

    if bytes.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "erro": "O arquivo está vazio." })),
        ));
    }

    String::from_utf8(bytes.to_vec()).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "erro": "O arquivo deve ser um texto UTF-8 válido." })),
        )
    })?;

    let huff_bytes = huffman::compactar_bytes(&bytes).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "erro": "Erro interno ao compactar o arquivo." })),
        )
    })?;

    let download_name = format!("{file_name}.huff");
    let content_disposition = format!("attachment; filename=\"{download_name}\"");

    let headers = [
        (header::CONTENT_TYPE, "application/octet-stream"),
        (header::CONTENT_DISPOSITION, content_disposition.as_str()),
    ];

    Ok((headers, huff_bytes).into_response())
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    Ok(app())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, HeaderMap, HeaderValue, Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn montar_multipart(conteudo: &[u8], nome_arquivo: Option<&str>) -> (HeaderMap, Vec<u8>) {
        let boundary = "huff-test-boundary";
        let mut body = Vec::new();

        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        let disposition = match nome_arquivo {
            Some(nome) => format!("form-data; name=\"arquivo\"; filename=\"{nome}\""),
            None => "form-data; name=\"arquivo\"".to_string(),
        };
        body.extend_from_slice(format!("Content-Disposition: {disposition}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
        body.extend_from_slice(conteudo);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&format!("multipart/form-data; boundary={boundary}")).unwrap(),
        );
        (headers, body)
    }

    async fn enviar(conteudo: &[u8], nome_arquivo: Option<&str>) -> Response {
        let (headers, body) = montar_multipart(conteudo, nome_arquivo);
        let request = Request::builder()
            .method("POST")
            .uri("/api/compactar")
            .headers(headers)
            .body(Body::from(body))
            .unwrap();

        app().oneshot(request).await.unwrap()
    }

    async fn corpo(resposta: Response) -> Vec<u8> {
        let (_partes, body) = resposta.into_parts();
        body.collect().await.unwrap().to_bytes().to_vec()
    }

    #[tokio::test]
    async fn upload_valido_retorna_200_e_disposition() {
        let resposta = enviar(b"abracadabra", Some("teste.txt")).await;
        assert_eq!(resposta.status(), StatusCode::OK);

        let disposition = resposta
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(disposition.contains("attachment; filename=\"teste.txt.huff\""));

        let bytes_corpo = corpo(resposta).await;
        assert!(!bytes_corpo.is_empty());
    }

    #[tokio::test]
    async fn bytes_nao_utf8_retorna_400() {
        let resposta = enviar(&[0xFF, 0xFE, 0x00, 0x81], Some("binario.bin")).await;
        assert_eq!(resposta.status(), StatusCode::BAD_REQUEST);

        let bytes_corpo = corpo(resposta).await;
        let json: serde_json::Value = serde_json::from_slice(&bytes_corpo).unwrap();
        assert_eq!(json["erro"], "O arquivo deve ser um texto UTF-8 válido.");
    }

    #[tokio::test]
    async fn arquivo_vazio_retorna_400() {
        let resposta = enviar(b"", Some("vazio.txt")).await;
        assert_eq!(resposta.status(), StatusCode::BAD_REQUEST);

        let bytes_corpo = corpo(resposta).await;
        let json: serde_json::Value = serde_json::from_slice(&bytes_corpo).unwrap();
        assert_eq!(json["erro"], "O arquivo está vazio.");
    }

    #[tokio::test]
    async fn sem_arquivo_retorna_400() {
        let resposta = enviar(b"conteudo sem arquivo", None).await;
        assert_eq!(resposta.status(), StatusCode::BAD_REQUEST);

        let bytes_corpo = corpo(resposta).await;
        let json: serde_json::Value = serde_json::from_slice(&bytes_corpo).unwrap();
        assert_eq!(json["erro"], "Nenhum arquivo enviado.");
    }
}
```

- [ ] **Step 3: Rodar todos os testes**

Run: `cargo test`
Expected: compilar sem erros e `test result: ok.` com 8 testes passando (4 do core + 4 do handler).

- [ ] **Step 4: Rodar clippy**

Run: `cargo clippy --all-targets 2>&1`
Expected: nenhum erro e nenhum warning.

- [ ] **Step 5: Rodar fmt**

Run: `cargo fmt --check`
Expected: sem diferenças de formatação. Se houver diferenças, rode `cargo fmt`.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs
git commit -m "(feat) API /api/compactar com validação e resposta .huff para download"
```

---

## Task 4: Frontend — `static/index.html`

**Files:**
- Modify: `static/index.html` (substituir conteúdo inteiro)

Não há infra de teste de JS no projeto; a verificação desta task é manual (servidor rodando + navegador/curl na Tarefa 6).

- [ ] **Step 1: Escrever o novo `static/index.html`**

```html
<!DOCTYPE html>
<html lang="pt-BR">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Huffman Compressor — Compacte arquivos de texto</title>
    <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🌳</text></svg>">
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        :root {
            --bg-dark: #0a0f0e;
            --bg-panel: #111816;
            --green-neon: #00ff9c;
            --green-dim: #00b874;
            --cyan-neon: #00e5ff;
            --text-primary: #e0f2e9;
            --text-secondary: #9ab0a6;
            --font-mono: 'Share Tech Mono', 'Fira Code', 'Courier New', monospace;
            --border-glow: 0 0 15px rgba(0, 255, 156, 0.4);
            --transition: 0.3s ease;
        }

        html,
        body {
            min-height: 100%;
            font-family: var(--font-mono);
            background: var(--bg-dark);
            color: var(--text-primary);
            -webkit-font-smoothing: antialiased;
            -moz-osx-font-smoothing: grayscale;
        }

        #matrix-canvas {
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            z-index: 1;
            pointer-events: none;
            opacity: 0.35;
            filter: blur(0.5px);
        }

        .container {
            position: relative;
            z-index: 2;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 2rem 1rem 4rem;
            text-align: center;
            background: radial-gradient(circle at center, rgba(10, 15, 14, 0.7) 0%, rgba(10, 15, 14, 0.95) 100%);
        }

        .logo-wrapper {
            margin-bottom: 1.5rem;
            position: relative;
        }

        .logo-tree {
            width: 120px;
            height: 120px;
            filter: drop-shadow(0 0 12px rgba(0, 255, 156, 0.5));
            animation: float 3s ease-in-out infinite;
        }

        @keyframes float {
            0%,
            100% {
                transform: translateY(0);
            }
            50% {
                transform: translateY(-12px);
            }
        }

        .tree-svg {
            width: min(280px, 80vw);
            height: auto;
            margin-bottom: 1.8rem;
            filter: drop-shadow(0 0 20px rgba(0, 255, 156, 0.3));
        }

        .node {
            fill: #0d1f1a;
            stroke: var(--green-neon);
            stroke-width: 2.5;
            transform-origin: center;
            transform-box: fill-box;
            animation: pulse 2.4s ease-in-out infinite;
            filter: drop-shadow(0 0 6px rgba(0, 255, 156, 0.7));
        }

        .node:nth-child(2) { animation-delay: 0.2s; }
        .node:nth-child(3) { animation-delay: 0.4s; }
        .node:nth-child(4) { animation-delay: 0.6s; }
        .node:nth-child(5) { animation-delay: 0.8s; }
        .node:nth-child(6) { animation-delay: 1.0s; }
        .node:nth-child(7) { animation-delay: 1.2s; }

        @keyframes pulse {
            0%,
            100% {
                transform: scale(1);
                stroke-width: 2.5;
            }
            50% {
                transform: scale(1.35);
                stroke-width: 3.5;
                fill: #0f2a22;
            }
        }

        .edge {
            stroke: var(--green-dim);
            stroke-width: 2;
            stroke-dasharray: 6 4;
            animation: dashFlow 1.2s linear infinite;
            filter: drop-shadow(0 0 4px rgba(0, 255, 156, 0.4));
        }

        .edge-reverse {
            animation-direction: reverse;
        }

        @keyframes dashFlow {
            to {
                stroke-dashoffset: -20;
            }
        }

        .bit-label {
            fill: var(--cyan-neon);
            font-size: 14px;
            font-weight: bold;
            font-family: var(--font-mono);
            text-anchor: middle;
            dominant-baseline: middle;
            filter: drop-shadow(0 0 4px rgba(0, 229, 255, 0.8));
            animation: labelGlow 1.8s ease-in-out infinite;
        }

        @keyframes labelGlow {
            0%,
            100% {
                opacity: 0.7;
            }
            50% {
                opacity: 1;
                filter: drop-shadow(0 0 8px rgba(0, 229, 255, 1));
            }
        }

        .leaf-char {
            fill: var(--text-primary);
            font-size: 13px;
            font-weight: bold;
            font-family: var(--font-mono);
            text-anchor: middle;
            dominant-baseline: middle;
        }

        .main-title {
            font-size: clamp(2rem, 6vw, 3.5rem);
            font-weight: 700;
            letter-spacing: 4px;
            text-transform: uppercase;
            margin-bottom: 0.8rem;
            color: var(--green-neon);
            text-shadow: 0 0 10px rgba(0, 255, 156, 0.6), 0 0 30px rgba(0, 255, 156, 0.3);
            animation: titlePulse 3s ease-in-out infinite;
        }

        @keyframes titlePulse {
            0%,
            100% {
                text-shadow: 0 0 10px rgba(0, 255, 156, 0.6), 0 0 30px rgba(0, 255, 156, 0.3);
            }
            50% {
                text-shadow: 0 0 20px rgba(0, 255, 156, 0.9), 0 0 50px rgba(0, 255, 156, 0.6);
            }
        }

        .subtitle {
            font-size: clamp(1rem, 3vw, 1.4rem);
            color: var(--text-secondary);
            letter-spacing: 2px;
            margin-bottom: 2.5rem;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 8px;
            flex-wrap: wrap;
        }

        .typing-text {
            display: inline-block;
            min-width: 12ch;
        }

        .cursor {
            display: inline-block;
            width: 10px;
            height: 1.2em;
            background: var(--cyan-neon);
            margin-left: 4px;
            animation: blink 0.9s step-end infinite;
            vertical-align: middle;
            box-shadow: 0 0 8px rgba(0, 229, 255, 0.8);
        }

        @keyframes blink {
            0%,
            100% {
                opacity: 1;
            }
            50% {
                opacity: 0;
            }
        }

        .progress-container {
            width: min(400px, 80vw);
            margin: 0 auto 1rem;
            background: rgba(255, 255, 255, 0.05);
            border-radius: 50px;
            padding: 3px;
            border: 1px solid rgba(0, 255, 156, 0.3);
            box-shadow: 0 0 15px rgba(0, 255, 156, 0.2);
        }

        .progress-bar {
            height: 18px;
            border-radius: 50px;
            background: linear-gradient(90deg,
                    var(--green-dim) 0%,
                    var(--green-neon) 30%,
                    var(--cyan-neon) 70%,
                    var(--green-neon) 100%);
            background-size: 200% 100%;
            animation: progressMove 2.5s linear infinite;
            box-shadow: 0 0 12px rgba(0, 255, 156, 0.7);
            position: relative;
            overflow: hidden;
        }

        .progress-bar::after {
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: linear-gradient(90deg,
                    transparent,
                    rgba(255, 255, 255, 0.3),
                    transparent);
            transform: translateX(-100%);
            animation: shimmer 2s ease-in-out infinite;
        }

        @keyframes progressMove {
            0% {
                background-position: 0% 50%;
            }
            100% {
                background-position: 200% 50%;
            }
        }

        @keyframes shimmer {
            0% {
                transform: translateX(-100%);
            }
            60%,
            100% {
                transform: translateX(100%);
            }
        }

        .progress-label {
            font-size: 0.9rem;
            color: var(--text-secondary);
            margin-bottom: 0.8rem;
            letter-spacing: 1px;
        }

        .upload-zone {
            width: min(520px, 90vw);
            margin: 0 auto 1.2rem;
            padding: 2rem 1.5rem;
            border: 2px dashed rgba(0, 255, 156, 0.4);
            border-radius: 12px;
            background: rgba(0, 255, 156, 0.04);
            cursor: pointer;
            transition: var(--transition);
        }

        .upload-zone:hover,
        .upload-zone.dragging {
            border-color: var(--green-neon);
            background: rgba(0, 255, 156, 0.1);
            box-shadow: var(--border-glow);
        }

        .upload-hint {
            color: var(--text-secondary);
            font-size: 1rem;
            letter-spacing: 1px;
        }

        .upload-hint span {
            color: var(--green-neon);
            text-decoration: underline;
        }

        .upload-info {
            margin-top: 0.8rem;
            color: var(--cyan-neon);
            font-size: 0.95rem;
            word-break: break-all;
        }

        .btn-compactar {
            font-family: var(--font-mono);
            font-size: 1.05rem;
            letter-spacing: 2px;
            text-transform: uppercase;
            padding: 0.8rem 2.4rem;
            margin-bottom: 1.2rem;
            color: var(--bg-dark);
            background: linear-gradient(90deg, var(--green-dim), var(--green-neon));
            border: none;
            border-radius: 50px;
            cursor: pointer;
            box-shadow: 0 0 18px rgba(0, 255, 156, 0.5);
            transition: var(--transition);
        }

        .btn-compactar:hover:not(:disabled) {
            transform: translateY(-2px);
            box-shadow: 0 0 28px rgba(0, 255, 156, 0.8);
        }

        .btn-compactar:disabled {
            opacity: 0.35;
            cursor: not-allowed;
        }

        .error-message {
            width: min(520px, 90vw);
            margin: 0 auto 1.2rem;
            padding: 0.8rem 1.2rem;
            color: #ff6b6b;
            background: rgba(255, 0, 60, 0.08);
            border: 1px solid rgba(255, 80, 80, 0.4);
            border-radius: 8px;
            font-size: 0.95rem;
            letter-spacing: 0.5px;
        }

        .result-panel {
            width: min(520px, 90vw);
            margin: 0 auto 1.2rem;
            padding: 1.2rem;
            background: rgba(0, 255, 156, 0.05);
            border: 1px solid rgba(0, 255, 156, 0.3);
            border-radius: 12px;
        }

        .stats-grid {
            display: flex;
            justify-content: space-around;
            gap: 1rem;
            margin-bottom: 1.2rem;
            flex-wrap: wrap;
        }

        .stat {
            display: flex;
            flex-direction: column;
            align-items: center;
        }

        .stat-value {
            color: var(--green-neon);
            font-size: 1.3rem;
            font-weight: bold;
        }

        .stat-label {
            color: var(--text-secondary);
            font-size: 0.8rem;
            letter-spacing: 1px;
            text-transform: uppercase;
        }

        .download-link {
            display: inline-block;
            font-family: var(--font-mono);
            letter-spacing: 1px;
            padding: 0.7rem 1.6rem;
            color: var(--bg-dark);
            background: var(--cyan-neon);
            border-radius: 50px;
            text-decoration: none;
            font-weight: bold;
            box-shadow: 0 0 18px rgba(0, 229, 255, 0.5);
            transition: var(--transition);
        }

        .download-link:hover {
            transform: translateY(-2px);
            box-shadow: 0 0 28px rgba(0, 229, 255, 0.8);
        }

        .footer {
            text-align: center;
            font-size: 0.8rem;
            color: rgba(255, 255, 255, 0.4);
            letter-spacing: 1px;
            padding: 1rem;
            position: relative;
            z-index: 2;
        }

        .footer a {
            color: var(--green-dim);
            text-decoration: none;
            transition: var(--transition);
        }

        .footer a:hover {
            color: var(--green-neon);
            text-shadow: 0 0 8px rgba(0, 255, 156, 0.6);
        }

        @media (max-width: 600px) {
            .tree-svg { width: 200px; }
            .logo-tree { width: 80px; height: 80px; }
            .subtitle { font-size: 0.9rem; gap: 5px; }
            .progress-container { width: 90vw; }
        }

        @media (prefers-reduced-motion: reduce) {
            *,
            *::before,
            *::after {
                animation-duration: 0.01ms !important;
                animation-iteration-count: 1 !important;
                transition-duration: 0.01ms !important;
            }
        }
    </style>
    <link href="https://fonts.googleapis.com/css2?family=Share+Tech+Mono&display=swap" rel="stylesheet">
</head>
<body>

    <canvas id="matrix-canvas"></canvas>

    <div class="container">
        <div class="logo-wrapper">
            <svg class="logo-tree" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
                <circle cx="50" cy="30" r="8" fill="#0d1f1a" stroke="#00ff9c" stroke-width="2.5"/>
                <line x1="50" y1="38" x2="30" y2="60" stroke="#00b874" stroke-width="2" stroke-dasharray="4 3"/>
                <line x1="50" y1="38" x2="70" y2="60" stroke="#00b874" stroke-width="2" stroke-dasharray="4 3"/>
                <circle cx="30" cy="60" r="6" fill="#0d1f1a" stroke="#00ff9c" stroke-width="2"/>
                <circle cx="70" cy="60" r="6" fill="#0d1f1a" stroke="#00ff9c" stroke-width="2"/>
                <line x1="30" y1="66" x2="20" y2="85" stroke="#00b874" stroke-width="2" stroke-dasharray="4 3"/>
                <line x1="30" y1="66" x2="40" y2="85" stroke="#00b874" stroke-width="2" stroke-dasharray="4 3"/>
                <line x1="70" y1="66" x2="60" y2="85" stroke="#00b874" stroke-width="2" stroke-dasharray="4 3"/>
                <line x1="70" y1="66" x2="80" y2="85" stroke="#00b874" stroke-width="2" stroke-dasharray="4 3"/>
                <circle cx="20" cy="85" r="4" fill="#0d1f1a" stroke="#00ff9c" stroke-width="1.5"/>
                <circle cx="40" cy="85" r="4" fill="#0d1f1a" stroke="#00ff9c" stroke-width="1.5"/>
                <circle cx="60" cy="85" r="4" fill="#0d1f1a" stroke="#00ff9c" stroke-width="1.5"/>
                <circle cx="80" cy="85" r="4" fill="#0d1f1a" stroke="#00ff9c" stroke-width="1.5"/>
                <text x="36" y="50" fill="#00e5ff" font-size="9" font-weight="bold" font-family="monospace">0</text>
                <text x="62" y="50" fill="#00e5ff" font-size="9" font-weight="bold" font-family="monospace">1</text>
                <text x="18" y="76" fill="#00e5ff" font-size="8" font-weight="bold" font-family="monospace">0</text>
                <text x="42" y="76" fill="#00e5ff" font-size="8" font-weight="bold" font-family="monospace">1</text>
                <text x="58" y="76" fill="#00e5ff" font-size="8" font-weight="bold" font-family="monospace">0</text>
                <text x="82" y="76" fill="#00e5ff" font-size="8" font-weight="bold" font-family="monospace">1</text>
            </svg>
        </div>

        <svg class="tree-svg" viewBox="0 0 300 200" xmlns="http://www.w3.org/2000/svg">
            <line x1="150" y1="30" x2="90" y2="80" class="edge" />
            <line x1="150" y1="30" x2="210" y2="80" class="edge edge-reverse" />
            <line x1="90" y1="80" x2="50" y2="140" class="edge" />
            <line x1="90" y1="80" x2="130" y2="140" class="edge edge-reverse" />
            <line x1="210" y1="80" x2="170" y2="140" class="edge" />
            <line x1="210" y1="80" x2="250" y2="140" class="edge edge-reverse" />

            <text x="112" y="52" class="bit-label">0</text>
            <text x="188" y="52" class="bit-label">1</text>
            <text x="62" y="108" class="bit-label">0</text>
            <text x="118" y="108" class="bit-label">1</text>
            <text x="182" y="108" class="bit-label">0</text>
            <text x="238" y="108" class="bit-label">1</text>

            <circle cx="150" cy="30" r="12" class="node" />
            <circle cx="90" cy="80" r="10" class="node" />
            <circle cx="210" cy="80" r="10" class="node" />
            <circle cx="50" cy="140" r="8" class="node" />
            <circle cx="130" cy="140" r="8" class="node" />
            <circle cx="170" cy="140" r="8" class="node" />
            <circle cx="250" cy="140" r="8" class="node" />

            <text x="50" y="165" class="leaf-char">'A'</text>
            <text x="130" y="165" class="leaf-char">'B'</text>
            <text x="170" y="165" class="leaf-char">'C'</text>
            <text x="250" y="165" class="leaf-char">'D'</text>
        </svg>

        <h1 class="main-title">Huffman Compressor</h1>

        <div class="subtitle">
            <span>⚙️</span>
            <span class="typing-text" id="typing-text">Compacte arquivos de texto</span>
            <span class="cursor"></span>
            <span>⚙️</span>
        </div>

        <div class="upload-zone" id="upload-zone">
            <input type="file" id="file-input" hidden>
            <p class="upload-hint">📂 Arraste um arquivo de texto aqui ou <span>clique para escolher</span></p>
            <p class="upload-info" id="upload-info" hidden></p>
        </div>

        <button class="btn-compactar" id="btn-compactar" disabled>🗜️ Compactar</button>

        <div class="progress-label" id="progress-label" hidden>⏳ Comprimindo...</div>
        <div class="progress-container" id="progress-container" hidden>
            <div class="progress-bar"></div>
        </div>

        <div class="error-message" id="error-message" hidden></div>

        <div class="result-panel" id="result-panel" hidden>
            <div class="stats-grid">
                <div class="stat">
                    <span class="stat-value" id="stat-original">—</span>
                    <span class="stat-label">Original</span>
                </div>
                <div class="stat">
                    <span class="stat-value" id="stat-compressed">—</span>
                    <span class="stat-label">Compactado</span>
                </div>
                <div class="stat">
                    <span class="stat-value" id="stat-reduction">—</span>
                    <span class="stat-label">Redução</span>
                </div>
            </div>
            <a class="download-link" id="download-link" href="#" download>⬇️ Baixar arquivo .huff</a>
        </div>
    </div>

    <div class="footer">
        © 2025 Huffman Compressor • <a href="#">Sobre</a> • <a href="#">Contato</a>
    </div>

    <script src="app.js"></script>
</body>
</html>
```

- [ ] **Step 2: Verificar que o HTML não tem referências quebradas**

Run: `grep -n 'id="' static/index.html`
Expected: IDs presentes para `matrix-canvas`, `typing-text`, `upload-zone`, `file-input`, `upload-info`, `btn-compactar`, `progress-label`, `progress-container`, `error-message`, `result-panel`, `download-link`, `stat-original`, `stat-compressed`, `stat-reduction`.

- [ ] **Step 3: Commit**

```bash
git add static/index.html
git commit -m "(feat) Página funcional de upload/compressão mantendo o tema neon"
```

---

## Task 5: Frontend — `static/app.js`

**Files:**
- Modify: `static/app.js` (substituir conteúdo inteiro)

- [ ] **Step 1: Escrever o novo `static/app.js`**

```js
(function () {
    const MAX_SIZE = 5 * 1024 * 1024;

    const uploadZone = document.getElementById('upload-zone');
    const fileInput = document.getElementById('file-input');
    const uploadInfo = document.getElementById('upload-info');
    const btnCompactar = document.getElementById('btn-compactar');
    const progressContainer = document.getElementById('progress-container');
    const progressLabel = document.getElementById('progress-label');
    const errorMessage = document.getElementById('error-message');
    const resultPanel = document.getElementById('result-panel');
    const downloadLink = document.getElementById('download-link');
    const statOriginal = document.getElementById('stat-original');
    const statCompressed = document.getElementById('stat-compressed');
    const statReduction = document.getElementById('stat-reduction');

    let arquivoSelecionado = null;

    function formatarBytes(bytes) {
        if (bytes === 0) return '0 B';
        const unidades = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(1024));
        return (bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 2) + ' ' + unidades[i];
    }

    function mostrarErro(mensagem) {
        errorMessage.textContent = mensagem;
        errorMessage.hidden = false;
        resultPanel.hidden = true;
    }

    function limparErro() {
        errorMessage.hidden = true;
    }

    function selecionarArquivo(arquivo) {
        if (!arquivo) return;
        if (arquivo.size > MAX_SIZE) {
            mostrarErro('O arquivo excede o limite de 5 MB.');
            arquivoSelecionado = null;
            uploadInfo.hidden = true;
            btnCompactar.disabled = true;
            return;
        }
        arquivoSelecionado = arquivo;
        uploadInfo.textContent = arquivo.name + ' • ' + formatarBytes(arquivo.size);
        uploadInfo.hidden = false;
        btnCompactar.disabled = false;
        limparErro();
    }

    uploadZone.addEventListener('click', () => fileInput.click());

    uploadZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        uploadZone.classList.add('dragging');
    });

    uploadZone.addEventListener('dragleave', () => uploadZone.classList.remove('dragging'));

    uploadZone.addEventListener('drop', (e) => {
        e.preventDefault();
        uploadZone.classList.remove('dragging');
        selecionarArquivo(e.dataTransfer.files[0]);
    });

    fileInput.addEventListener('change', () => selecionarArquivo(fileInput.files[0]));

    btnCompactar.addEventListener('click', async () => {
        if (!arquivoSelecionado) return;

        limparErro();
        resultPanel.hidden = true;
        btnCompactar.disabled = true;
        progressContainer.hidden = false;
        progressLabel.hidden = false;

        const formData = new FormData();
        formData.append('arquivo', arquivoSelecionado);

        try {
            const resposta = await fetch('/api/compactar', { method: 'POST', body: formData });

            if (!resposta.ok) {
                let mensagem = 'Falha ao compactar o arquivo.';
                try {
                    const dados = await resposta.json();
                    if (dados && dados.erro) mensagem = dados.erro;
                } catch (erroIgnorado) { }
                throw new Error(mensagem);
            }

            const blob = await resposta.blob();
            const nomeDownload = arquivoSelecionado.name + '.huff';
            const url = URL.createObjectURL(blob);

            downloadLink.href = url;
            downloadLink.download = nomeDownload;
            statOriginal.textContent = formatarBytes(arquivoSelecionado.size);
            statCompressed.textContent = formatarBytes(blob.size);
            statReduction.textContent = arquivoSelecionado.size > 0
                ? ((1 - blob.size / arquivoSelecionado.size) * 100).toFixed(1) + '%'
                : '—';
            resultPanel.hidden = false;
        } catch (erro) {
            mostrarErro(erro.message || 'Falha ao compactar o arquivo.');
        } finally {
            progressContainer.hidden = true;
            progressLabel.hidden = true;
            btnCompactar.disabled = false;
        }
    });

    const typingElement = document.getElementById('typing-text');
    const baseText = 'Compacte arquivos de texto';
    let isDeleting = false;
    let charIndex = baseText.length;

    function typeEffect() {
        if (!isDeleting) {
            if (charIndex < baseText.length) {
                charIndex++;
                typingElement.textContent = baseText.substring(0, charIndex);
                setTimeout(typeEffect, 60);
            } else {
                isDeleting = true;
                setTimeout(typeEffect, 3000);
            }
        } else {
            if (charIndex > 0) {
                charIndex--;
                typingElement.textContent = baseText.substring(0, charIndex);
                setTimeout(typeEffect, 25);
            } else {
                isDeleting = false;
                setTimeout(typeEffect, 1000);
            }
        }
    }
    setTimeout(typeEffect, 500);

    const canvas = document.getElementById('matrix-canvas');
    const ctx = canvas.getContext('2d');
    let width, height;
    let fontSize = 16;
    let columns, drops;

    function setupCanvas() {
        width = canvas.width = window.innerWidth;
        height = canvas.height = window.innerHeight;
        columns = Math.floor(width / fontSize);
        drops = Array(columns).fill(0).map(() => Math.random() * -100);
    }

    function drawMatrix() {
        ctx.fillStyle = 'rgba(10, 15, 14, 0.1)';
        ctx.fillRect(0, 0, width, height);
        ctx.font = fontSize + 'px "Share Tech Mono", monospace';
        ctx.fillStyle = '#00ff9c';
        ctx.shadowColor = '#00ff9c';
        ctx.shadowBlur = 4;

        for (let i = 0; i < columns; i++) {
            const char = Math.random() > 0.5 ? '1' : '0';
            const x = i * fontSize;
            const y = drops[i] * fontSize;
            ctx.fillText(char, x, y);

            if (y > height && Math.random() > 0.975) {
                drops[i] = 0;
            }
            drops[i]++;
        }

        ctx.shadowBlur = 0;
        requestAnimationFrame(drawMatrix);
    }

    window.addEventListener('resize', setupCanvas);
    setupCanvas();
    drawMatrix();
})();
```

- [ ] **Step 2: Validar sintaxe do JS**

Run: `node --check static/app.js`
Expected: sem output (sintaxe válida). Se `node` não estiver instalado, pule para o Step 3.

- [ ] **Step 3: Commit**

```bash
git add static/app.js
git commit -m "(feat) Lógica de upload, compressão e download no frontend"
```

---

## Task 6: Verificação integrada (testes + build + endpoint real)

**Files:** nenhum (verificação)

- [ ] **Step 1: Rodar testes, clippy e fmt**

```bash
cargo test 2>&1
cargo clippy --all-targets 2>&1
cargo fmt --check
```

Expected: 8 testes passando, sem warnings do clippy, `fmt --check` sem diffs.

- [ ] **Step 2: Subir o servidor local e testar a API com curl**

```bash
cargo shuttle run
```

Em outro terminal:

```bash
printf 'aaabbbccc' > /tmp/opencode/texto.txt
curl -s -o /tmp/opencode/texto.txt.huff -D - \
  -F "arquivo=@/tmp/opencode/texto.txt;type=text/plain" \
  http://localhost:8000/api/compactar
```

Expected: resposta com `HTTP/1.1 200 OK`, header `Content-Disposition: attachment; filename="texto.txt.huff"`, `Content-Type: application/octet-stream`, e o arquivo `/tmp/opencode/texto.txt.huff` criado não vazio.

- [ ] **Step 3: Testar os casos de erro via curl**

```bash
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8000/api/compactar
curl -s http://localhost:8000/api/compactar \
  -F "arquivo=@/dev/null;filename=vazio.txt;type=text/plain"
```

Expected: primeiro comando retorna `400` (sem arquivo) e o segundo imprime JSON com `"O arquivo está vazio."`.

- [ ] **Step 4: Testar a página estática**

Run: `curl -s http://localhost:8000/ | grep -c "Huffman Compressor"`
Expected: `1` (a página index.html é servida na raiz).

- [ ] **Step 5: Verificação manual no navegador**

Abra `http://localhost:8000/` no navegador e valide:
1. Arrastar um `.txt` para a zona de upload mostra nome e tamanho; botão "Compactar" habilita.
2. Clicar em "Compactar" mostra a barra de progresso e, ao final, o painel de resultado com Original/Compactado/Redução e o botão "Baixar arquivo .huff".
3. Baixar o arquivo e conferir que o nome termina em `.huff`.
4. Enviar um arquivo binário (ex.: `.png`) → aparece "O arquivo deve ser um texto UTF-8 válido."
5. Selecionar um arquivo grande (acima de 5MB) → aparece "O arquivo excede o limite de 5 MB."

---

## Task 7: Deploy no Shuttle

**Files:** nenhum (comandos; exige conta no Shuttle)

- [ ] **Step 1: Login no Shuttle**

```bash
cargo shuttle login
```

- [ ] **Step 2: Criar o projeto**

```bash
cargo shuttle project start
```

- [ ] **Step 3: Deploy**

```bash
cargo shuttle deploy
```

- [ ] **Step 4: Validar o deploy**

Abra a URL publicada (mostrada pelo comando `deploy`, no formato `https://<projeto>.shuttle.app`) e repita a verificação manual da Task 6.

**Notas do plano free:** 128MB de RAM; o app "dorme" após inatividade e o primeiro acesso tem cold start de alguns segundos; o limite de upload de 5MB cabe confortavelmente.

---

## Self-Review (executado após escrever o plano)

- **Cobertura do spec:** core em memória (T1), limpeza de código morto (T2), API + validações + erros JSON + DefaultBodyLimit (T3), frontend upload/download/estatísticas/erros (T4/T5), testes/clippy/fmt (T6), deploy Shuttle (T7). ✓
- **Placeholders:** nenhum; todo código fornecido completo. ✓
- **Consistência de tipos:** `compactar_bytes(&[u8]) -> io::Result<Vec<u8>>` usado em `mod.rs` (T1) e no handler (T3); erro JSON `{"erro": "..."}` consistente entre handler, testes e frontend; IDs do HTML (T4) batem com os `getElementById` do `app.js` (T5). ✓
