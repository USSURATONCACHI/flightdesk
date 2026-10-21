# Diagnostics for the test VM: uploads the setup transcript and MSI logs to the host, retries
# the install with Unblock-File and a verbose log. Injected via the VM keyboard when needed.
$h = "http://10.0.2.2:8000"
function Up($name, $path) { if (Test-Path $path) { try { Invoke-RestMethod -Uri "$h/upload/$name" -Method Post -InFile $path | Out-Null } catch {} } }
function UpText($name, $text) { try { Invoke-RestMethod -Uri "$h/upload/$name" -Method Post -Body $text | Out-Null } catch {} }
Up "setup-transcript.txt" "C:\setup-transcript.txt"
Up "msi.log" "C:\msi.log"
$msi = Get-ChildItem C:\*.msi | Select-Object -First 1
$info = @()
$info += "whoami: $(whoami)"
$info += "elevated: $(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))"
$info += "msi: $($msi.FullName) size=$($msi.Length)"
$info += "zone: $((Get-Content -Raw -Stream Zone.Identifier $msi.FullName -ErrorAction SilentlyContinue) -replace "`r`n", ' | ')"
$info += "installer service: $((Get-Service msiserver).Status)"
$info += "os: $([Environment]::OSVersion.VersionString) $((Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DisplayVersion)"
UpText "diag-info.txt" ($info -join "`n")
Unblock-File $msi.FullName
$p = Start-Process msiexec.exe -ArgumentList "/i `"$($msi.FullName)`" /qn /norestart /l*v C:\msi2.log" -Wait -PassThru
UpText "diag-retry.txt" "retry exit=$($p.ExitCode); exe present=$(Test-Path 'C:\Program Files\Flightdeck\flightdeck.exe')"
Up "msi2.log" "C:\msi2.log"
if (Test-Path "C:\Program Files\Flightdeck\flightdeck.exe") {
  New-NetFirewallRule -DisplayName "Flightdeck MCP" -Direction Inbound -Protocol TCP -LocalPort 7799 -Action Allow -ErrorAction SilentlyContinue | Out-Null
  Start-Process -FilePath "C:\Program Files\Flightdeck\flightdeck.exe" -ArgumentList "--mcp --mcp-port 7799 --mcp-host 0.0.0.0 --url http://10.0.2.2:8089/engine-rest --open def:order_process" -WorkingDirectory "C:\Program Files\Flightdeck"
  UpText "diag-started.txt" "started"
}
