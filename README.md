<p align="center">
  <img src="icons/icon-256x256.png" alt="Gituqueiro" width="128">
</p>

<h1 align="center">Gituqueiro</h1>

<p align="center">
  <em>The git gossiper &mdash; keep an eye on what's happening across your repos</em>
</p>

<p align="center">
  <strong>gituqueiro</strong> = <strong>git</strong> + <strong>fofoqueiro</strong> (Brazilian Portuguese for <em>gossiper</em>)
</p>

---

A desktop app for monitoring GitHub Pull Requests, CI status, and repository health across multiple repositories and organizations. Built with Rust and [iced](https://iced.rs).

## Features

- **PR Dashboard** &mdash; View open PRs across repos and entire organizations in one place
- **CI Status** &mdash; See pass/fail/running status per PR with links to check details
- **Security & Quality** &mdash; Monitor code scanning and Dependabot alerts per repo
- **Filters** &mdash; Show all PRs, only yours, PRs you're reviewing, or PRs by a specific user
- **Search** &mdash; Filter PRs by title, author, repo, number, or label
- **Sorting** &mdash; Click column headers to sort by repo, number, title, author, CI status, age, or size
- **Double-click to open** &mdash; Double-click any PR to open it in the browser
- **Auto-refresh** &mdash; Automatic polling with configurable interval
- **Incremental loading** &mdash; PRs appear as each repo responds, no waiting for all to finish
- **OAuth login** &mdash; Authenticate via GitHub's Device Flow (like `gh auth login`)
- **Organization support** &mdash; Monitor all repos in a GitHub organization
- **Cross-platform** &mdash; Runs on macOS, Windows, and Linux

## Installation

### Homebrew (macOS)

```bash
brew install marcelotrevisani/tap/gituqueiro
```

Or install as a macOS app (`.dmg` with Dock icon):

```bash
brew install --cask marcelotrevisani/tap/gituqueiro
```

### Download

Pre-built binaries are available on the [Releases](https://github.com/marcelotrevisani/gituqueiro/releases) page:

| Platform | Format |
|----------|--------|
| macOS (universal) | `.dmg` |
| macOS (x86_64 / aarch64) | `.tar.gz` |
| Linux (x86_64) | `.tar.gz`, `.deb` |
| Windows (x86_64) | `.zip`, `.msi` |

### From source

```bash
# Clone
git clone https://github.com/marcelotrevisani/gituqueiro.git
cd gituqueiro

# Build and run
cargo run --release
```

### macOS .app bundle

```bash
just run-app
```

This builds a proper `Gituqueiro.app` with the icon in the Dock.

### Requirements (building from source)

- Rust 1.75+
- System dependencies:
  - **Linux**: `libwayland-dev libxkbcommon-dev`
  - **macOS / Windows**: none

## Getting Started

### 1. Authenticate with GitHub

Open the app and click the **Settings** tab (top-right).

You have two options:

#### Option A: Personal Access Token

1. Go to [github.com/settings/tokens](https://github.com/settings/tokens) and create a token with these scopes:
   - `repo` (full access to private repos)
   - `read:org` (read org membership)
2. Paste the token in the **GitHub Token** field
3. Click **Save & Connect**

#### Option B: OAuth Device Flow

1. Create a GitHub OAuth App:
   - Go to **GitHub > Settings > Developer settings > OAuth Apps > New OAuth App**
   - Set the callback URL to `http://localhost` (not used)
   - After creation, go to the app settings and **enable Device Flow**
2. Copy the **Client ID** and paste it in the **OAuth App Client ID** field
3. Click **Login with GitHub**
4. A code will appear (e.g. `WDJB-MJHT`) and your browser will open to GitHub
5. Enter the code in the browser and authorize
6. The app will automatically detect the authorization and connect

> **Tip**: You can also set the `GITHUB_TOKEN` environment variable and the app will use it automatically on startup.

### 2. Add Repositories

In the **Settings** tab, scroll to **Monitored Sources**:

- **Individual repos**: Type `owner/repo` (e.g. `torvalds/linux`) and click **+ Add**
- **Organizations**: Type the org name (e.g. `my-org`) and click **+ Add** to monitor all repos in that organization. Archived and disabled repos are automatically excluded.

### 3. Monitor PRs

Switch to the **Pull Requests** tab. PRs load incrementally as each repo responds.

- Use the **sidebar filters** to narrow down:
  - **All PRs** &mdash; everything across all repos
  - **My PRs** &mdash; PRs you authored
  - **Review Requested** &mdash; PRs where you're a reviewer
  - **By User** &mdash; PRs by a specific GitHub username
- Use the **search box** to filter by title, author, repo, number, or label
- **Click column headers** to sort (click again to reverse)
- **Double-click a PR** to open it in the browser
- **Click the CI status** (OK/X/~) to open the checks page

#### PR indicators

| Indicator | Meaning |
|-----------|---------|
| `OK` | All CI checks passed |
| `X` | One or more checks failed |
| `~` | Checks are running or pending |
| `?` | No CI checks found |
| `[DRAFT]` | Draft PR |
| Age color: green | Less than 7 days old |
| Age color: yellow | 7-30 days old |
| Age color: red | Over 30 days (stale) |
| Size: XS/S/M/L/XL | Based on lines changed |

### 4. Security & Quality

Switch to the **Security & Quality** tab to see per-repo health:

- Code scanning alerts (with severity)
- Dependabot alerts (with severity)
- Open and stale PR counts
- Click alert links to view details on GitHub

### 5. Connection Status

The bottom-left of the sidebar shows your connection status:

- **Green dot** + username: connected
- **Yellow dot**: authenticating
- **Red dot**: not connected

**Double-click** the connection status to jump to Settings.

## Development

```bash
# Run with hot-reload (debug mode)
just run

# Run all checks (format + lint + test)
just check

# Run tests
just test

# Format code
just format

# Lint
just lint

# Generate icons from SVG
just icon

# Build macOS .app
just app
```

### Project Structure

```
src/
  main.rs              # Entry point, window icon setup
  app.rs               # Iced UI: sidebar, tabs, settings, PR list, health dashboard
  config.rs            # Persistent config (~/.config/gituqueiro/config.json)
  lib.rs               # Library exports
  github/
    mod.rs
    types.rs           # GitHub API + app data types
    client.rs          # GitHub API client, incremental fetch
    oauth.rs           # OAuth Device Flow
tests/
  integration_tests.rs # Wiremock-based integration tests
icons/
  icon.svg             # Vector source
  icon.icns            # macOS
  icon.ico             # Windows
  icon.png             # 1024x1024
  icon-*.png           # All sizes
scripts/
  build-icons.sh       # SVG -> PNG/ICO/ICNS
  build-app.sh         # macOS .app bundle
```

## License

MIT
