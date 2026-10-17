# Authenticode-signs a file produced by `tauri build` (the app executable and the MSI).
# Tauri calls it through bundle.windows.signCommand with the file path.
#
# Certificate: the one whose thumbprint is in YTMR_SIGN_THUMBPRINT, otherwise the
# maintainer's code-signing certificate ("CN=Mirang Bhandari") from the current user's store,
# used for official releases. Without one the build continues unsigned, so anyone can build
# the open-source code. Replacing the self-signed certificate with one from a trusted CA
# needs no changes here.
param([Parameter(Mandatory = $true)][string]$Path)

$ErrorActionPreference = "Stop"
$subject = "CN=Mirang Bhandari"
$timestampServer = "http://timestamp.digicert.com"

$certs = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Where-Object { $_.NotAfter -gt (Get-Date) }
if ($env:YTMR_SIGN_THUMBPRINT) {
    $cert = $certs | Where-Object { $_.Thumbprint -eq $env:YTMR_SIGN_THUMBPRINT } | Select-Object -First 1
} else {
    $cert = $certs | Where-Object { $_.Subject -eq $subject } | Sort-Object NotAfter -Descending | Select-Object -First 1
}

if (-not $cert) {
    Write-Warning "No code-signing certificate found; leaving $Path unsigned."
    exit 0
}

$result = Set-AuthenticodeSignature -FilePath $Path -Certificate $cert -HashAlgorithm SHA256 -TimestampServer $timestampServer
# A self-signed certificate is not chained to a trusted root, which Windows reports as
# UnknownError; the signature itself is still written and verifiable.
if (-not $result.SignerCertificate -or $result.Status -notin @("Valid", "UnknownError")) {
    Write-Error "Signing $Path failed: $($result.Status) $($result.StatusMessage)"
    exit 1
}
Write-Host "Signed $Path ($($result.Status), $($cert.Subject), $($cert.Thumbprint))"
