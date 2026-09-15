## Downloads

| Package | Platform |
| --- | --- |
| `clean-the-agent-gui-@TAG@-darwin-arm64.dmg` | macOS 14+, Apple silicon |
| `clean-the-agent-gui-@TAG@-darwin-x64.dmg` | macOS 14+, Intel |
| `clean-the-agent-@TAG@-darwin-arm64.tar.gz` | macOS CLI, Apple silicon |
| `clean-the-agent-@TAG@-darwin-x64.tar.gz` | macOS CLI, Intel |
| `clean-the-agent-@TAG@-linux-x64.tar.gz` | Linux CLI, x86-64 |
| `clean-the-agent-@TAG@-windows-x64.zip` | Windows CLI, x86-64 |

`SHA256SUMS.txt` contains the checksum for every package.

## macOS signing and Gatekeeper

The macOS application, disk image, and CLI binaries are **ad-hoc signed but not
notarized**. The project does not currently publish with an Apple Developer ID
certificate or notarization credentials, so Gatekeeper cannot establish an
identified developer or an Apple notarization ticket. Verify the checksum and
only bypass Gatekeeper if you trust this repository and this release.

Verify the downloaded DMG:

```bash
TAG="@TAG@"
ARCH="$(uname -m)"
case "$ARCH" in
  arm64) ASSET_ARCH="arm64" ;;
  x86_64) ASSET_ARCH="x64" ;;
  *) echo "Unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
esac

curl -fLO "https://github.com/wibus-wee/clean-the-agent/releases/download/${TAG}/SHA256SUMS.txt"
grep "clean-the-agent-gui-${TAG}-darwin-${ASSET_ARCH}.dmg" SHA256SUMS.txt | shasum -a 256 -c -
```

After dragging the app to `/Applications`, first try Control-clicking the app
and choosing **Open**. If macOS still blocks it, the following removes only this
app's quarantine attribute and launches it:

```bash
xattr -dr com.apple.quarantine "/Applications/Clean the Agent.app"
open "/Applications/Clean the Agent.app"
```

You can inspect the local signature independently:

```bash
codesign --verify --deep --strict --verbose=2 "/Applications/Clean the Agent.app"
codesign -dv --verbose=4 "/Applications/Clean the Agent.app" 2>&1 | grep 'Signature=adhoc'
```

## CLI install examples

macOS or Linux:

```bash
TAG="@TAG@"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) TARGET="darwin-arm64" ;;
  Darwin-x86_64) TARGET="darwin-x64" ;;
  Linux-x86_64) TARGET="linux-x64" ;;
  *) echo "No prebuilt CLI for this platform" >&2; exit 1 ;;
esac

ARCHIVE="clean-the-agent-${TAG}-${TARGET}.tar.gz"
curl -fLO "https://github.com/wibus-wee/clean-the-agent/releases/download/${TAG}/${ARCHIVE}"
curl -fLO "https://github.com/wibus-wee/clean-the-agent/releases/download/${TAG}/SHA256SUMS.txt"
grep "${ARCHIVE}" SHA256SUMS.txt | shasum -a 256 -c -
tar -xzf "$ARCHIVE"
install -d "$HOME/.local/bin"
install -m 0755 "${ARCHIVE%.tar.gz}/clean-the-agent" "$HOME/.local/bin/clean-the-agent"
```

Windows PowerShell:

```powershell
$Tag = "@TAG@"
$Archive = "clean-the-agent-$Tag-windows-x64.zip"
$Base = "https://github.com/wibus-wee/clean-the-agent/releases/download/$Tag"
Invoke-WebRequest "$Base/$Archive" -OutFile $Archive
Invoke-WebRequest "$Base/SHA256SUMS.txt" -OutFile SHA256SUMS.txt
$Expected = ((Select-String -Path SHA256SUMS.txt -Pattern ([regex]::Escape($Archive))).Line -split '\s+')[0]
$Actual = (Get-FileHash $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($Actual -ne $Expected) { throw "SHA-256 mismatch for $Archive" }
Expand-Archive $Archive -DestinationPath . -Force
New-Item -ItemType Directory -Force "$HOME\bin" | Out-Null
Copy-Item "clean-the-agent-$Tag-windows-x64\clean-the-agent.exe" "$HOME\bin\clean-the-agent.exe"
```

Ensure `$HOME/.local/bin` or `$HOME\bin` is on `PATH`, then run
`clean-the-agent --version`.
