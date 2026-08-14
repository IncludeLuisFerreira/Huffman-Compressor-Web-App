/* 
mod huffman;
mod web;
use crate::{huffman::huffman, web::teste};

fn main() {
    let path = "teste.txt";
    teste();

    if let Err(error) = huffman(path) {
        eprintln!("Erro: {}", error);
        std::process::exit(1);
    }
}
*/

use axum::{
    extract::Multipart,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

// Importa o módulo da sua lógica de Huffman
mod huffman;

/// Handler para processar o upload do arquivo e retornar o .huff
async fn compactar_handler(mut multipart: Multipart) -> Result<Response, (StatusCode, String)> {
    // Procura pelo arquivo no formulário enviado
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        // Pega o nome do arquivo enviado pelo usuário (ou usa um padrão)
        let file_name = field
            .file_name()
            .unwrap_or("arquivo")
            .to_string();

        // Extrai os bytes brutos do arquivo da requisição HTTP
        let bytes = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Converte os bytes para String (o texto original do arquivo)
        let content = String::from_utf8(bytes.to_vec())
            .map_err(|_| (StatusCode::BAD_REQUEST, "O arquivo deve ser um texto válido!".to_string()))?;

        // 🧠 CHAMA A SUA ENGINE DE HUFFMAN!
        let huff_bytes = huffman::huffman(&content);

        // Prepara o nome do arquivo para download (ex: texto.txt.huff)
        let download_name = format!("{}.huff", file_name);

        // Monta a resposta HTTP que força o download do arquivo no navegador
        let headers = [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", download_name).as_str(),
            ),
        ];

        return Ok((headers, huff_bytes).into_response());
    }

    Err((StatusCode::BAD_REQUEST, "Nenhum arquivo enviado.".to_string()))
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    // Configura as rotas da sua API
    let router = Router::new()
        // Serve os arquivos da pasta static (seu HTML/CSS) na raiz "/"
        .nest_service("/", ServeDir::new("static"))
        // Rota POST para receber o arquivo para compactar
        .route("/api/compactar", post(compactar_handler));

    Ok(router.into())
}