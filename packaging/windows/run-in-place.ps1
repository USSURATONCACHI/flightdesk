Set-Location -LiteralPath $PSScriptRoot
$env:FLIGHTDECK_CONFIG_DIR = "config"
$env:FLIGHTDECK_DATA_DIR = "data"
$env:FLIGHTDECK_LOCALE_DIR = "locales"
Start-Process -FilePath ".\flightdeck.exe" -ArgumentList $args -WorkingDirectory $PSScriptRoot
