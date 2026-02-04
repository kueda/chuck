declare module 'inaturalistjs' {
  interface SearchParams {
    q: string;
    sources: string;
    per_page?: number;
  }

  interface SearchResponse {
    results: unknown[];
    total_results: number;
    page: number;
    per_page: number;
  }

  interface FetchResponse {
    results: unknown[];
    total_results: number;
    page: number;
    per_page: number;
  }

  interface Endpoint {
    fetch(id: number | string): Promise<FetchResponse>;
  }

  const inatjs: {
    search(params: SearchParams): Promise<SearchResponse>;
    taxa: Endpoint;
    places: Endpoint;
    users: Endpoint;
  };

  export default inatjs;
}
