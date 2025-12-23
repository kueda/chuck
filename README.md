# Chuck
A desktop application for browsing DarwinCore Archives (DwC-A), with an added option to create DwC-As from iNaturalist.

## Features
* load and store multiple DwC-A files
* display occurrences from a DwC-A as a table, grid of cards, or a map
* filter occurrences by
  * eventDate range (e.g. occurrences from 2021-04-05 to 2024-02-05)
  * eventDate year (e.g. occurrences in 2024)
  * eventDate month (e.g. occurrences in May)
  * eventDate day (e.g. occurrences on 2024-06-03)
  * decimalLatitude / decimalLongitude in bounding box
  * recordedBy (e.g. occurrences by kueda)
  * taxonomic fields (e.g. where scientific_name is "Homo sapiens" or where genus is "Vulpes")
* view archive metadata files (eml.xml and metadata.xml)

## Tech
Chuck is a [Tauri](https://tauri.app) application where most of the logic is in Rust. Data is accessed and queried via [DuckDB](https://duckdb.org/) (conversion of DwC-A core and extension files into DuckDB tables happens when first loaded into Chuck). SvelteKit is used for the front-end along with [Skeleton](https://www.skeleton.dev/) and [Tailwind](https://tailwindcss.com/).
