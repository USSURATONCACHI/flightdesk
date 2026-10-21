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
Stop-Transcript
