param([string]$Text)
Set-Clipboard -Value $Text
Write-Output "Copied to clipboard: $Text"
