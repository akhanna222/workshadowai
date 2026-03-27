import { useState, useCallback } from "react";
import { ipc, SearchResult, SearchFilters } from "../lib/ipc";

export function useSearch() {
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState("");

  const search = useCallback(async (q: string, filters?: SearchFilters) => {
    if (!q.trim()) {
      setResults([]);
      return;
    }
    setLoading(true);
    setQuery(q);
    try {
      const r = await ipc.search(q, filters);
      setResults(r);
    } catch (err) {
      console.error("Search failed:", err);
      setResults([]);
    } finally {
      setLoading(false);
    }
  }, []);

  return { results, loading, query, search };
}
