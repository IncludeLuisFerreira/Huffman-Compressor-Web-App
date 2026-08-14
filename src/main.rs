use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::json;
use tower_http::services::ServeDir;

mod huffman;

fn app() -> Router {
    Router::new()
        .fallback_service(ServeDir::new("static"))
        .route("/api/compactar", post(compactar_handler))
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024))
}

async fn compactar_handler(
    mut multipart: Multipart,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let arquivo = 'procurar: loop {
        match multipart.next_field().await.map_err(|erro| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "erro": format!("Falha ao ler o upload: {erro}") })),
            )
        })? {
            Some(campo) if campo.file_name().is_some() => break 'procurar Some(campo),
            Some(_) => continue 'procurar,
            None => break 'procurar None,
        }
    };

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
    Ok(app().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{HeaderMap, HeaderValue, Request, header};
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
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/compactar")
            .body(Body::from(body))
            .unwrap();
        *request.headers_mut() = headers;

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
