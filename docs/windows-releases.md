# Windows releases

The Release workflow's `windows` job builds the Windows installers after the `linux`
job succeeds and uploads them to the same GitHub release. Besides the
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets the
other jobs use, it reads the Azure code signing variables described under
[Code signing](#code-signing).

## What gets published

The job runs twice on `windows-latest`: once for `x86_64-pc-windows-msvc` and once
for `aarch64-pc-windows-msvc`. The ARM64 build is cross-compiled on the x64 runner,
which has the MSVC ARM64 build tools.

| Asset | Contents |
| --- | --- |
| `Shipyard-windows-x64.exe` | Installer for Intel and AMD PCs |
| `Shipyard-windows-arm64.exe` | Installer for ARM PCs, like Snapdragon laptops |
| `*.exe.sig` | Updater signatures for each installer |

Both are NSIS installers that install for the current user, so they don't ask for
administrator rights. Shipyard doesn't publish an `.msi`, since Tauri can't build
one for ARM64, and one installer type keeps updates simple. Windows 10 and 11
include the WebView2 runtime Shipyard draws its window with. On a PC without it, the
installer downloads it first.

The names stay the same from release to release, so the README links to
`releases/latest/download/…` keep working.

## Code signing

The installers are signed as **RazorKode LLC** with
[Azure Artifact Signing](https://learn.microsoft.com/azure/artifact-signing/), the
same publisher the macOS builds use. Signing turns on once the repository variable
`AZURE_SIGNING_PROFILE` is set. Without it the job skips the signing steps and the
installers ship unsigned, and Windows SmartScreen shows **Windows protected your
PC** the first time someone runs one.

### How it works

1. `azure/login` signs in to Azure with the job's GitHub OIDC token. There is no
   client secret. The `Shipyard` app registration in the RazorKode LLC Microsoft Entra
   directory trusts only this repository's `release` environment, which is why the
   job sets `environment: release` and `id-token: write`.
2. `scripts/setup-windows-signing.ps1` downloads Microsoft's signing client (the
   SignTool "dlib", from the `Microsoft.ArtifactSigning.Client` NuGet package),
   finds SignTool, and writes a Tauri config file that sets
   `bundle.windows.signCommand`. The dlib needs the .NET 10 runtime, which
   `actions/setup-dotnet` installs.
3. `tauri-action` gets `--config <that file>`. Tauri then signs the app, the
   uninstaller and the installer, each with a timestamp, before it writes the
   updater `.sig` files. If SignTool fails, the build fails and nothing is uploaded.

### Repository variables

These live under **Settings → Secrets and variables → Actions → Variables**. None
of them is secret.

| Variable | Value |
| --- | --- |
| `AZURE_CLIENT_ID` | Application (client) ID of the `Shipyard` app registration |
| `AZURE_TENANT_ID` | Directory (tenant) ID of the RazorKode LLC directory |
| `AZURE_SIGNING_ENDPOINT` | `https://eus.codesigning.azure.net/` |
| `AZURE_SIGNING_ACCOUNT` | `razorkode-signing` |
| `AZURE_SIGNING_PROFILE` | The certificate profile's name. Leave it unset to ship unsigned. |

The Artifact Signing account, its certificate profile, and the RazorKode LLC
identity validation are shared by Lockshot, Recon and Shipyard. Each app has its
own app registration with the **Artifact Signing Certificate Profile Signer** role
on the account.

### Checking a signed installer

On Windows, right-click the installer, choose **Properties → Digital Signatures**,
and confirm the signer is RazorKode LLC. In PowerShell,
`Get-AuthenticodeSignature .\Shipyard-windows-x64.exe` should report `Valid`.
SmartScreen can still warn for a while after the first signed release, until the
publisher builds up download reputation.

## Updates

Each architecture adds a `windows-<arch>-nsis` entry, and a plain `windows-<arch>`
entry, to the release's `latest.json`. **Help → Check for Updates…** downloads the
new installer and runs it in passive mode, which shows a progress bar and needs no
clicks, then restarts Shipyard.

The jobs run one after another (`needs: linux` and `max-parallel: 1`) for the same
reason the Linux jobs do: each one downloads `latest.json`, adds its entries, and
uploads it again.

If an earlier job fails, the Windows job doesn't run. Re-run the failed jobs from
the Actions page once the problem is fixed.

## Verification

After the first Windows release, install it on Windows 11 with
[Git for Windows](https://git-scm.com/download/win) and confirm:

- Shipyard finds Git, lists a repository, and fetches it, and no console windows
  flash while it works.
- **Open in File Explorer** on a repository and **Show in File Explorer** on a
  changed file both open File Explorer.
- Opening a conflicted file launches the editor picked in General settings.
- The terminal panel starts PowerShell in the repository folder.
- **Help → Check for Updates…** reports that Shipyard is up to date.
