# Prepares Azure Artifact Signing for the Release workflow's windows job.
#
# Downloads Microsoft's signing client (the SignTool "dlib"), finds SignTool,
# writes the account details the dlib reads, and writes a Tauri config file
# whose bundle.windows.signCommand signs every binary Tauri bundles: the app,
# the uninstaller and the installer. Tauri signs before it writes the updater
# .sig files, so those match the signed installers.
#
# The dlib signs in with the Azure CLI session that azure/login opened. No
# secret is involved. See docs/windows-releases.md.
#
# Finally it appends `--config <file>` to TAURI_SIGNING_ARGS in GITHUB_ENV,
# which the tauri-action step adds to its arguments.

param(
  [Parameter(Mandatory = $true)][string]$Endpoint,
  [Parameter(Mandatory = $true)][string]$Account,
  [Parameter(Mandatory = $true)][string]$CertificateProfile
)

$ErrorActionPreference = 'Stop'

$ClientVersion = '1.0.151'
$Root = Join-Path $env:RUNNER_TEMP 'artifact-signing'
New-Item -ItemType Directory -Force -Path $Root | Out-Null

# Microsoft.ArtifactSigning.Client is a NuGet package; a .nupkg is a zip file.
$Package = Join-Path $Root 'client.zip'
$Url = "https://api.nuget.org/v3-flatcontainer/microsoft.artifactsigning.client/$ClientVersion/microsoft.artifactsigning.client.$ClientVersion.nupkg"
Invoke-WebRequest -Uri $Url -OutFile $Package
Expand-Archive -Path $Package -DestinationPath (Join-Path $Root 'client') -Force
$Dlib = Join-Path $Root 'client\bin\x64\Azure.CodeSigning.Dlib.dll'
if (-not (Test-Path $Dlib)) { throw "Azure.CodeSigning.Dlib.dll not found in the signing client package" }

# Use the newest x64 SignTool from the installed Windows SDKs.
$SignTool = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' |
  Sort-Object { [version]$_.Directory.Parent.Name } -Descending |
  Select-Object -First 1 -ExpandProperty FullName
if (-not $SignTool) { throw "signtool.exe not found in the Windows SDK" }
Write-Host "SignTool: $SignTool"

$Metadata = Join-Path $Root 'metadata.json'
@{
  Endpoint               = $Endpoint
  CodeSigningAccountName = $Account
  CertificateProfileName = $CertificateProfile
  # Only the Azure CLI session from azure/login should be tried.
  ExcludeCredentials     = @(
    'EnvironmentCredential',
    'ManagedIdentityCredential',
    'WorkloadIdentityCredential',
    'SharedTokenCacheCredential',
    'VisualStudioCredential',
    'VisualStudioCodeCredential',
    'AzurePowerShellCredential',
    'AzureDeveloperCliCredential',
    'InteractiveBrowserCredential'
  )
} | ConvertTo-Json | Set-Content -Path $Metadata -Encoding utf8

$Config = Join-Path $Root 'tauri.windows-signing.json'
@{
  bundle = @{
    windows = @{
      signCommand = @{
        cmd  = $SignTool
        args = @(
          'sign', '/v', '/fd', 'SHA256',
          '/tr', 'http://timestamp.acs.microsoft.com', '/td', 'SHA256',
          '/dlib', $Dlib, '/dmdf', $Metadata,
          '%1'
        )
      }
    }
  }
} | ConvertTo-Json -Depth 5 | Set-Content -Path $Config -Encoding utf8

$ConfigArg = $Config -replace '\\', '/'
Add-Content -Path $env:GITHUB_ENV -Value "TAURI_SIGNING_ARGS=--config $ConfigArg"
Write-Host "Windows signing is set up with certificate profile '$CertificateProfile'."
