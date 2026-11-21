# Fails unless a file carries a valid Windows code signature.
#   check_signature.ps1 -Path <file> -When <what the check follows>
param(
    [Parameter(Mandatory)] [string]$Path,
    [Parameter(Mandatory)] [string]$When
)
$signature = Get-AuthenticodeSignature -FilePath $Path
if ($signature.Status -ne 'Valid') {
    Write-Error "$Path is not validly signed ($When): $($signature.Status)"
    exit 1
}
"$Path is signed ($When) by: $($signature.SignerCertificate.Subject)"
