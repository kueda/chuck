# Architecture

Chuck is a desktop app for browsing biodiversity data in DarwinCore Archive
(DwC-A) format and creating DwC-A exports from iNaturalist.

## Tech Stack
- **Frontend:** SvelteKit 2 + Svelte 5 (SPA via adapter-static), Skeleton UI,
  Tailwind CSS 4, MapLibre GL, TanStack Virtual
- **Backend:** Tauri 2 (Rust), DuckDB, reqwest, tokio
- **CLI:** chuck-cli crate for headless iNat exports
- **Shared lib:** chuck-core crate (auth, DwC types, iNat API client, downloader)
- **Tools:** Vite 6, Biome (lint/format), Vitest (unit), Playwright (integration)

## Directory Layout
```
src/                        # Frontend (SvelteKit)
  routes/                   # Pages: +page.svelte, inat-download/, metadata/
  lib/
    components/             # Svelte components
    composables/            # Reactive logic (useInatSearch.svelte.ts)
    types/                  # TS types (archive.ts, inaturalist.ts)
    utils/                  # Helpers (viewPreferences, filterCategories, drawerState)
    tauri-api.ts            # Tauri command wrapper (supports test mocking)
src-tauri/                  # Tauri backend
  src/
    commands/               # Tauri commands: archive.rs, inat_download.rs, inat_auth.rs
    db/database.rs          # DuckDB wrapper
    dwca/archive.rs         # DwC-A extraction and meta.xml parsing
    tile_server/            # Custom MVT tile protocol for maps
    search_params.rs        # Search/filter parameter handling
chuck-core/                 # Shared Rust library
  src/
    darwin_core/            # Types: Occurrence, Multimedia, Identification, Comment
    auth/                   # OAuth2, token storage (file/keyring)
    api/                    # iNat API client with rate limiting
    downloader.rs           # Photo/metadata download orchestration
    dwca_extension.rs       # Extension enum (SimpleMultimedia, Audiovisual, etc.)
chuck-cli/                  # CLI tool
  src/
    commands/               # CLI commands (obs export)
    output/                 # CSV/DwC output formatting
```

## Data Flow
1. User opens a .zip DwC-A → Tauri extracts, parses meta.xml, loads CSVs into
   DuckDB with type coercion (lat/lng→DOUBLE, bools→BOOLEAN)
2. Frontend sends SearchParams → Tauri `search()` → DuckDB SQL → JSON results
3. Results rendered in Table/Cards/Map/Groups views with virtualized scrolling
4. Occurrence detail: `get_occurrence()` loads extensions (multimedia, IDs, comments)
5. Photos: lazy-extracted from archive on demand, cached in app local data
6. iNat download: fetch from iNat API → build DwC-A (CSVs + meta.xml) → ZIP

## Key Types
- **Rust:** `Occurrence`, `Multimedia`, `Identification`, `Comment`
  (chuck-core/src/darwin_core/), `Archive` (src-tauri/src/dwca/archive.rs),
  `Database` (src-tauri/src/db/database.rs),
  `DwcaExtension` enum (chuck-core/src/dwca_extension.rs)
- **TS:** `Occurrence`, `SearchParams`, `ArchiveInfo`, `SearchResult`
  (src/lib/types/archive.ts), `Taxon`, `User`, `Place`
  (src/lib/types/inaturalist.ts)

## State Management
Svelte 5 runes ($state, $derived, $effect) — no centralized store. Key state
files: viewPreferences.ts (localStorage), drawerState.ts,
useInatSearch.svelte.ts.

## Testing
- **Unit:** `src/**/*.test.ts` (Vitest, jsdom)
- **Integration:** `tests/*.spec.ts` (Playwright, Desktop Safari/Edge)
- **Performance:** `tests/performance.spec.ts` (separate Playwright project)
- **Backend:** inline cargo tests, `cargo test`
- **Mocks:** `tests/mocks/tauri.ts` (window.__MOCK_TAURI__), `tests/fixtures/`

# Commands
- Frontend integration tests: `npm run test:frontend:integration`
- Frontend unit tests: `npm run test:frontend:unit`
- Frontend performance tests: `npm run test:frontend:performance` (run separately to avoid resource contention)
- Backend tests: `npm run test:backend` or `cargo test`
- All tests: `npm test`
- Lint: `npm run check`

# Development Workflow
- ALWAYS run frontend tests and lint after changing src/
- ALWAYS run backend tests after changing src-tauri/, chuck-core/, or chuck-cli/
- Branch names: start with the issue number, e.g. `123-fix-broken-embedded-photos`

# Test-Driven Development (TDD)
**ALWAYS write tests before fixing code when:**
- User reports a bug (e.g., "bug:", "broken:", "doesn't work")
- User describes unexpected behavior
- User says something "should" work differently
- You are debugging or investigating a failure

**Process:**
1. Write a failing test that reproduces the issue
2. NEVER proceed if the test does not fail; ask for permission instead
3. Fix the code to make the test pass
4. NEVER make the test pass by altering the test
5. Run full test suite to ensure no regressions

# User Interface
- When using Skeleton docs, append `.md` for LLM-optimized instructions,
  e.g. https://www.skeleton.dev/docs/svelte/framework-components/accordion.md
