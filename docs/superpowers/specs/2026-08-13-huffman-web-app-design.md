# Design — Web App de Compressão Huffman (Backend Rust + Shuttle)

**Data:** 2026-08-13
**Status:** Aprovado para implementação

## Objetivo

Transformar o projeto (que hoje tem apenas o núcleo Huffman em Rust e uma landing page
"Em Desenvolvimento") em um web app funcional de compressão: o usuário envia um arquivo
de texto, o backend comprime com o algoritmo de Huffman e devolve o arquivo `.huff` para
download. Deploy gratuito no Shuttle.

## Escopo

- **Só compressão.** Descompressão fica para um trabalho futuro.
- **Só arquivos de texto** (UTF-8 válido). Arquivos binários são rejeitados com erro claro.
- **Limite de upload: 5 MB** por arquivo (processado em memória; Shuttle free tem 128MB de RAM).
- **Interface:** manter o tema neon/terminal existente (canvas Matrix, árvore binária) e
  transformar a landing em página funcional.
- **Stack:** Rust, axum, Shuttle (já configurados no `Cargo.toml`), HTML/CSS/JS vanilla servidos
  via `ServeDir` — sem build de frontend.

## Arquitetura

Abordagem escolhida: **core Huffman em memória + app axum atual**.

```
Navegador ── POST /api/compactar (multipart) ──► axum handler
                                                     │ valida (limite, UTF-8, vazio)
                                                     ▼
                                              huffman::compactar_bytes(&[u8]) -> Vec<u8>
                                                     │
                              response: attachment "nome.txt.huff" (application/octet-stream)
                                                     ▼
Navegador ◄──────────── blob + URL.createObjectURL ────── (botão de download + estatísticas)
```

## Componentes

### 1. Núcleo Huffman em memória (`src/huffman/mod.rs`)

- Trocar a API pública de `huffman(path: &str) -> io::Result<()>` (lê/grava no disco) por:

  ```rust
  pub fn compactar_bytes(bytes: &[u8]) -> io::Result<Vec<u8>>
  ```

- Processamento idêntico ao atual, porém 100% em memória:
  1. `calcular_frequencia(bytes, &mut tabela)` — percorre os bytes do conteúdo.
  2. `criar_node(tabela)` — monta a `BinaryHeap` de folhas.
  3. Constrói a árvore de Huffman agrupando os dois menores nós.
  4. `gerar_codigo(raiz, "", &mut dicionario)` — gera o código de cada símbolo.
  5. `BitWriter` escreve o payload bit a bit; `finalizar()` devolve os bytes.
  6. Monta o `.huff`: `[nº símbolos u16 LE] + ([byte] + [freq u32 LE]) × n + payload`.
- **Erro para arquivo vazio:** nenhum símbolo → sem raiz → retornar `Err` com mensagem clara.
- As funções auxiliares (`calcular_frequencia`, `criar_node`, `gerar_codigo`, `BitWriter`,
  `Node`) não mudam de lógica.

### 2. Limpeza de código morto

- `src/huffman/huff_file.rs` (`escrever_arquivo_huff`) — removido (não usa mais disco).
- `src/web/` (servidor TCP antigo) — removido; `web` deixa de ser módulo.
- Formato `.huff` **inalterado** (compatibilidade preservada).

### 3. API HTTP (`src/main.rs`)

- Subir o limite padrão do axum: `DefaultBodyLimit::max(6 * 1024 * 1024)` (5MB do arquivo +
  folga do multipart). O default do axum é 2MB.
- Handler `POST /api/compactar` (multipart):
  - Nenhum arquivo enviado → `400` + `{"erro": "Nenhum arquivo enviado."}`.
  - Nome ausente → usa `arquivo` como padrão.
  - Bytes não-UTF-8 → `400` + `{"erro": "O arquivo deve ser um texto UTF-8 válido."}`
    (via `String::from_utf8`).
  - Vazio → `400` + `{"erro": "O arquivo está vazio."}`.
  - Sucesso → `compactar_bytes(&bytes)` e resposta `200` com
    `Content-Type: application/octet-stream` e
    `Content-Disposition: attachment; filename="{nome}.huff"`.
  - Erro inesperado no core → `500` + JSON.
- Erros sempre em JSON (`application/json`) para o frontend ler facilmente.
- Rotas: `ServeDir::new("static")` na raiz + `POST /api/compactar`.

### 4. Frontend (`static/index.html` + `static/app.js`)

Manter identidade visual (neon/terminal, canvas Matrix de fundo, árvore binária). A página
"Em Desenvolvimento" vira a página funcional:

- **Zona de upload:** drag & drop + clique para escolher arquivo (`<input type="file">`),
  exibe nome e tamanho do arquivo selecionado.
- **Botão "Compactar":** habilitado quando há arquivo selecionado; durante o processamento a
  barra de progresso existente vira indicador de "comprimindo...".
- **Resultado:** link/botão de download do `.huff` e estatísticas (tamanho original,
  tamanho compactado, % de redução).
- **Erros:** mensagem clara lendo o JSON da resposta (não é texto válido, muito grande, vazio).
- **`app.js`:** `fetch` com `FormData` para `/api/compactar`, resposta como `Blob`,
  `URL.createObjectURL` para o download, corpo JSON em caso de erro.

## Tratamento de erros

| Situação | Resposta |
|---|---|
| Sem arquivo no upload | 400 `{"erro": "Nenhum arquivo enviado."}` |
| Arquivo maior que o limite | 413 (do `DefaultBodyLimit`) — frontend exibe mensagem de limite |
| Bytes não-UTF-8 | 400 `{"erro": "O arquivo deve ser um texto UTF-8 válido."}` |
| Arquivo vazio | 400 `{"erro": "O arquivo está vazio."}` |
| Falha inesperada no core | 500 `{"erro": "Erro interno ao compactar o arquivo."}` |
| Sucesso | 200, `application/octet-stream`, `attachment; filename="nome.huff"` |

## Testes

- **Unit tests no core (`src/huffman/`):**
  - Texto conhecido → cabeçalho correto (nº de símbolos, tabela byte/freq), payload não vazio.
  - Texto com um único símbolo repetido.
  - Texto vazio → erro.
  - Texto com vários caracteres.
- **Unit tests do handler (`src/main.rs`):** via `tower::ServiceExt::oneshot` com body
  multipart montado:
  - Sucesso: 200 + `Content-Disposition` com `nome.huff`.
  - Bytes inválidos UTF-8: 400 + JSON.
  - Arquivo vazio: 400 + JSON.
  - Upload sem arquivo: 400 + JSON.
- Verificação: `cargo test`, `cargo clippy`, `cargo fmt --check`.

## Deploy no Shuttle

- Já configurado: `shuttle-runtime`, `shuttle-axum`, `#[shuttle_runtime::main]` em `main.rs`.
- `static/` está versionada e não é ignorada pelo `.gitignore` (que só exclui `*.txt`/`*.huff`).
- `ServeDir::new("static")` resolve relativo à raiz do projeto no Shuttle.
- Rotina: `cargo shuttle login` → `cargo shuttle project start` → `cargo shuttle deploy`.
- Teste local: `cargo shuttle run`.
- Plano free: 128MB RAM, cold start após inatividade, upload de 5MB tranquilo.

## Fora de escopo (futuro)

- Descompressão (`.huff` → arquivo original).
- Suporte a arquivos binários.
- Visualização interativa da árvore de Huffman gerada.
