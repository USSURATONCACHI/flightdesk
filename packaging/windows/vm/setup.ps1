# Runs inside the Windows test VM at first logon (fetched from the host by autounattend.xml).
# Downloads the MSI built on Linux, installs it silently, then starts Flightdeck with its MCP
# server bound to all interfaces so the host can drive and inspect it through the forwarded port.
$ErrorActionPreference = "Continue"
$host_ = "http://10.0.2.2:8000"
function Report($stage) { try { Invoke-WebRequest -UseBasicParsing "$host_/vm-stage/$stage" | Out-Null } catch {} }
Start-Transcript -Path C:\setup-transcript.txt -Append
Report "logon"
$msi = (Invoke-WebRequest -UseBasicParsing "$host_/msi-name").Content.Trim()
Invoke-WebRequest -UseBasicParsing "$host_/$msi" -OutFile "C:\$msi"
Report "downloaded"
$p = Start-Process msiexec.exe -ArgumentList "/i `"C:\$msi`" /qn /norestart /l*v C:\msi.log" -Wait -PassThru
Report "installed-exit-$($p.ExitCode)"
$exe = "C:\Program Files\Flightdeck\flightdeck.exe"
if (Test-Path $exe) {
  New-NetFirewallRule -DisplayName "Flightdeck MCP" -Direction Inbound -Protocol TCP -LocalPort 7799 -Action Allow | Out-Null
  Start-Process -FilePath $exe -ArgumentList "--mcp --mcp-port 7799 --mcp-host 0.0.0.0 --url http://10.0.2.2:8089/engine-rest --open def:order_process" -WorkingDirectory "C:\Program Files\Flightdeck"
  Report "started"
} else {
  Report "exe-missing"
}
# Phase 2 is driven by the host: it writes a command into /next when its checks are done.
while ($true) {
  try { $cmd = (Invoke-WebRequest -UseBasicParsing "$host_/next").Content.Trim() } catch { $cmd = "" }
  if ($cmd -eq "portable") {
    Get-Process flightdeck -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
    $p = Start-Process msiexec.exe -ArgumentList "/x `"C:\$msi`" /qn /norestart" -Wait -PassThru
    Report "uninstalled-exit-$($p.ExitCode)"
    $zip = (Invoke-WebRequest -UseBasicParsing "$host_/zip-name").Content.Trim()
    Invoke-WebRequest -UseBasicParsing "$host_/$zip" -OutFile "C:\$zip"
    Expand-Archive -LiteralPath "C:\$zip" -DestinationPath "C:\portable" -Force
    $dir = (Get-ChildItem "C:\portable" -Directory | Select-Object -First 1).FullName
    # a) run in place from the archive (PowerShell launcher), then stop it
    & powershell -NoProfile -ExecutionPolicy Bypass -File "$dir\run-in-place.ps1" --mcp --mcp-port 7799 --mcp-host 0.0.0.0 --url http://10.0.2.2:8089/engine-rest --open def:order_process
    Report "portable-started"
    Start-Sleep -Seconds 25
    Get-Process flightdeck -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
    Report "portable-config-$(Test-Path "$dir\config\settings.toml")"
    # b) directory MSI from inside the archive
    $p = Start-Process msiexec.exe -ArgumentList "/i `"$dir\flightdeck-setup.msi`" /qn /norestart /l*v C:\msi-portable.log" -Wait -PassThru
    Report "dirmsi-installed-exit-$($p.ExitCode)"
    if (Test-Path $exe) {
      Start-Process -FilePath $exe -ArgumentList "--mcp --mcp-port 7799 --mcp-host 0.0.0.0 --url http://10.0.2.2:8089/engine-rest --open def:order_process" -WorkingDirectory "C:\Program Files\Flightdeck"
      Report "dirmsi-started"
    } else {
      Report "dirmsi-exe-missing"
    }
    break
  }
  if ($cmd -eq "done") { break }
  Start-Sleep -Seconds 5
}
Stop-Transcript
