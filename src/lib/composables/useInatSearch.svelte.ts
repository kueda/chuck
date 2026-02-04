import inatjs from 'inaturalistjs';
import type {
  ApiResponse,
  ComboboxItem,
  InatItem,
  Place,
  SearchResult,
  SourceType,
  Taxon,
  User,
} from '$lib/types/inaturalist';

type MapResultFn = (result: SearchResult) => ComboboxItem;

export function useInatSearch(source: SourceType, mapResultFn: MapResultFn) {
  let comboboxData = $state<ComboboxItem[]>([]);
  let selectedValue = $state<string[]>([]);
  let selectedItem = $state<InatItem | null>(null);
  let loading = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout>;

  async function search(query: string) {
    if (!query || !query.trim()) {
      comboboxData = [];
      return;
    }

    try {
      loading = true;
      const response = (await inatjs.search({
        q: query,
        sources: source,
        per_page: 10,
      })) as ApiResponse<SearchResult>;

      comboboxData = response.results.map(mapResultFn);
    } catch (e) {
      console.error('Search error:', e);
      comboboxData = [];
    } finally {
      loading = false;
    }
  }

  // Loads an item by ID and populates the combobox with it. This is needed
  // when initializing from URL hash state - we have the ID but need to fetch
  // the full item details to display the name, photo, etc.
  async function loadById(id: number | string) {
    try {
      loading = true;
      let response: ApiResponse<Taxon | Place | User> | undefined;
      if (source === 'taxa') {
        response = (await inatjs.taxa.fetch(id)) as ApiResponse<Taxon>;
      } else if (source === 'places') {
        response = (await inatjs.places.fetch(id)) as ApiResponse<Place>;
      } else if (source === 'users') {
        response = (await inatjs.users.fetch(id)) as ApiResponse<User>;
      }

      if (response?.results?.[0]) {
        const item = response.results[0];
        selectedItem = item;
        const mapped = mapResultFn({ taxon: item as Taxon, record: item });
        comboboxData = [mapped];
        selectedValue = [mapped.value];
      }
    } catch (e) {
      console.error('Error loading item by ID:', e);
    } finally {
      loading = false;
    }
  }

  function handleInputValueChange(e: { inputValue: string }) {
    const query = e.inputValue;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      search(query);
    }, 300);
  }

  function handleValueChange(
    e: { value: string[] },
    onChangeFn?: (item: InatItem | null) => void,
  ) {
    selectedValue = e.value;
    if (e.value.length > 0) {
      const selected = comboboxData.find((item) => item.value === e.value[0]);
      if (selected) {
        selectedItem = selected.item;
        if (onChangeFn) {
          onChangeFn(selected.item);
        }
      }
    } else {
      selectedItem = null;
      if (onChangeFn) {
        onChangeFn(null);
      }
    }
  }

  function clearSelection() {
    selectedValue = [];
    selectedItem = null;
    comboboxData = [];
  }

  return {
    get comboboxData() {
      return comboboxData;
    },
    get selectedValue() {
      return selectedValue;
    },
    get selectedItem() {
      return selectedItem;
    },
    set selectedItem(value) {
      selectedItem = value;
    },
    get loading() {
      return loading;
    },
    handleInputValueChange,
    handleValueChange,
    clearSelection,
    loadById,
  };
}
