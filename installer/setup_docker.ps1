# AgentOS Setup -- Docker + Ollama + Worker Image + Models

Write-Host "=== AgentOS Setup ===" -ForegroundColor Cyan
Write-Host ""

# 1. Docker Desktop
$dockerPath = Get-Command docker -ErrorAction SilentlyContinue
if (-not $dockerPath) {
    Write-Host "Instalando Docker Desktop..." -ForegroundColor Yellow
    $installerUrl = "https://desktop.docker.com/win/main/amd64/Docker%20Desktop%20Installer.exe"
    $installerPath = "$env:TEMP\DockerDesktopInstaller.exe"
    Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath -UseBasicParsing
    Start-Process -Wait $installerPath -ArgumentList "install", "--quiet", "--accept-license"
    Write-Host "Docker Desktop instalado. Puede requerir reinicio." -ForegroundColor Green
} else {
    Write-Host "Docker Desktop ya instalado." -ForegroundColor Green
}

# 2. Ollama
$ollamaPath = Get-Command ollama -ErrorAction SilentlyContinue
if (-not $ollamaPath) {
    Write-Host "Instalando Ollama..." -ForegroundColor Yellow
    $ollamaUrl = "https://ollama.com/download/OllamaSetup.exe"
    $ollamaInstaller = "$env:TEMP\OllamaSetup.exe"
    Invoke-WebRequest -Uri $ollamaUrl -OutFile $ollamaInstaller -UseBasicParsing
    Start-Process -Wait $ollamaInstaller -ArgumentList "/SILENT"
    Write-Host "Ollama instalado." -ForegroundColor Green
} else {
    Write-Host "Ollama ya instalado." -ForegroundColor Green
}

# 3. Worker Image
$imageExists = docker image inspect agentos-worker:latest 2>$null
if (-not $imageExists) {
    Write-Host "Construyendo imagen del worker..." -ForegroundColor Yellow
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    docker build -t agentos-worker:latest "$scriptDir\worker-image\"
    Write-Host "Imagen construida." -ForegroundColor Green
} else {
    Write-Host "Imagen del worker ya existe." -ForegroundColor Green
}

# 4. Modelos locales
Write-Host "Descargando modelos de IA locales..." -ForegroundColor Yellow
ollama pull phi3:mini 2>$null
ollama pull llama3.2:1b 2>$null
Write-Host "Modelos listos." -ForegroundColor Green

Write-Host ""
Write-Host "=== Setup completo ===" -ForegroundColor Cyan
Write-Host "AgentOS esta listo para usar." -ForegroundColor White
