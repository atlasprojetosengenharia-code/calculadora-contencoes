# Publica uma nova versão da Calculadora de Contenções.
#
# Uso (no PowerShell, dentro da pasta do projeto):
#     powershell -ExecutionPolicy Bypass -File .\publicar.ps1
#
# Antes de rodar: aumente "version" no Cargo.toml (ex.: 0.2.0 -> 0.3.0) e
# faça o commit. O script confere tudo, roda os testes e só então envia a
# tag que dispara a publicação no GitHub. Nada é publicado se algo falhar.

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Parar($msg) { Write-Host "`nERRO: $msg" -ForegroundColor Red; exit 1 }

# 1. Branch e alterações pendentes
$branch = (git rev-parse --abbrev-ref HEAD).Trim()
if ($branch -ne "main") { Parar "você está na branch '$branch'. Mude para a main." }
if (git status --porcelain) { Parar "há alterações sem commit. Faça o commit (ou descarte) antes de publicar." }

# 2. Versão do Cargo.toml e tag
$versao = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$tag = "v$versao"
git fetch --tags --quiet origin
if (git tag --list $tag) { Parar "a versão $tag já foi publicada. Aumente 'version' no Cargo.toml e faça o commit." }

# 3. Testes locais
Write-Host "Rodando os testes (cargo test)..." -ForegroundColor Cyan
cargo test
if ($LASTEXITCODE -ne 0) { Parar "os testes falharam. Nada foi publicado." }

# 4. Confirmação
Write-Host "`nTudo certo. Último commit:" -ForegroundColor Green
git log -1 --format="  %h  %s"
$ok = Read-Host "`nPublicar a versão $tag para os usuários? (s/n)"
if ($ok -notin @("s", "S", "sim", "Sim")) { Write-Host "Cancelado. Nada foi publicado."; exit 0 }

# 5. Envia o código e a tag (a tag dispara a publicação)
git push origin main
if ($LASTEXITCODE -ne 0) { Parar "falha ao enviar a main." }
git tag -a $tag -m "Versão $versao"
git push origin $tag
if ($LASTEXITCODE -ne 0) { Parar "falha ao enviar a tag $tag." }

Write-Host "`nTag $tag enviada. Acompanhe a publicação em:" -ForegroundColor Green
Write-Host "  https://github.com/atlasprojetosengenharia-code/calculadora-contencoes/actions"
