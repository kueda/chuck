# Chuck
A desktop application for browsing biodiversity occurrences in [DarwinCore Archives](https://en.wikipedia.org/wiki/Darwin_Core_Archive) (DwC-A), with an added option to create DwC-As from iNaturalist.

## Features
* view occurrences in DarwinCore Archives with occurrence cores, including tabular, image, and map views
* filter occurrences by most of the available fields
* view archive metadata files (eml.xml and metadata.xml)
* create DarwinCore Archives from iNaturalist records, with
  * date filtering
  * user filtering
  * embedded photos
  * authentication so you can include coordinates you have permission to see
  * CLI (e.g. `cargo run -p chuck-cli -- obs --user kueda --d1 2026-01-01 --d2 2026-02-01 --format dwc`)

## Status

Under active development, hoping to get some builds up for testing in early 2026. Still very rough in terms of... well in terms of everything.

Shooting for a 1.0.0 release that views DarwinCore Archives from GBIF, Symbiota, and iNat (via Chuck) and is capable of creating a backup of my (fairly expansive) iNat contributions.

## Tech
Chuck is a [Tauri](https://tauri.app) application where most of the logic is in Rust. Data is stored and queried via [DuckDB](https://duckdb.org/). SvelteKit is used for the front-end along with [Skeleton](https://www.skeleton.dev/) and [Tailwind](https://tailwindcss.com/).

### Setup

If you want to work on Chuck or just build it, you mostly just need working Rust and TypeScript environments. Building looks roughly like this:

```sh
# Install Rust: https://rust-lang.org/tools/install/
# Install Node / NPM: https://docs.npmjs.com/downloading-and-installing-node-js-and-npm
# Clone this repo

# Install JS deps
npm i

# Run the test suite, which will also install Rust deps
npm test
# wait a long time while duckdb compiles

# Run a development build w/ debug output
npm run tauri-debug
```

## Background

Chuck primarily grew out of a desire to back up my iNat observations in a standard, portable format like DwC-A, but frankly, a backup isn't that useful if you don't have an easy way to view what's in it, and a viewer has the added potential benefit of providing offline functionality in case you're traveling to places without Internet access and want an iNat-like or GBIF-like reference.
