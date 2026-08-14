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
    let compactando = false;
    let ultimaUrl = null;

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
        btnCompactar.disabled = compactando;
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

    document.addEventListener('dragover', (e) => e.preventDefault());
    document.addEventListener('drop', (e) => e.preventDefault());

    fileInput.addEventListener('change', () => selecionarArquivo(fileInput.files[0]));

    btnCompactar.addEventListener('click', async () => {
        if (!arquivoSelecionado) return;

        const arquivo = arquivoSelecionado;

        limparErro();
        resultPanel.hidden = true;
        btnCompactar.disabled = true;
        compactando = true;
        progressContainer.hidden = false;
        progressLabel.hidden = false;

        const formData = new FormData();
        formData.append('arquivo', arquivo);

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

            if (arquivoSelecionado !== arquivo) return;

            if (ultimaUrl) URL.revokeObjectURL(ultimaUrl);
            const url = URL.createObjectURL(blob);
            ultimaUrl = url;

            const nomeDownload = arquivo.name + '.huff';
            downloadLink.href = url;
            downloadLink.download = nomeDownload;
            statOriginal.textContent = formatarBytes(arquivo.size);
            statCompressed.textContent = formatarBytes(blob.size);
            statReduction.textContent = arquivo.size > 0
                ? ((1 - blob.size / arquivo.size) * 100).toFixed(1) + '%'
                : '—';
            resultPanel.hidden = false;
        } catch (erro) {
            mostrarErro(erro.message || 'Falha ao compactar o arquivo.');
        } finally {
            progressContainer.hidden = true;
            progressLabel.hidden = true;
            compactando = false;
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
