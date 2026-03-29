param([string]$Command)
if (-not $Command) { $Command = "hello-world" }
$result = docker run --rm $Command 2>&1
Write-Output $result
