# Windows releases

The Release workflow's `windows` job builds the Windows installers after the `linux`
job succeeds and uploads them to the same GitHub release. It needs no secrets beyond
the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` the other
jobs already use.

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

The installers aren't code-signed. Windows SmartScreen shows **Windows protected
your PC** the first time someone runs one, and they continue with **More info →
Run anyway**. The warning appears less often as more people download a release.

To sign them later, the cheapest option is
[Azure Trusted Signing](https://learn.microsoft.com/azure/trusted-signing/), at about
$10 a month. Tauri runs a custom sign command for each binary it bundles. See
[Tauri's Windows code signing guide](https://v2.tauri.app/distribute/sign/windows/)
for the `bundle.windows.signCommand` setting and the secrets the job would need. A
traditional OV or EV certificate works too, through `certificateThumbprint`.

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
