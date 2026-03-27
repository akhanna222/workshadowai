import { useSearch } from "../hooks/useSearch";
import { SearchBar } from "../components/SearchBar";
import { FrameCard } from "../components/FrameCard";
import { FilterPanel } from "../components/FilterPanel";

export function Search() {
  const { results, loading, query, search } = useSearch();

  return (
    <div>
      <h2 className="text-lg font-semibold mb-4">Search</h2>
      <div className="space-y-4">
        <SearchBar onSearch={search} loading={loading} />
        <FilterPanel onApplyFilters={() => { if (query) search(query); }} />

        {results.length > 0 && (
          <p className="text-sm text-[var(--ws-text-muted)]">
            {results.length} result{results.length !== 1 ? "s" : ""} for "{query}"
          </p>
        )}

        <div className="space-y-2">
          {results.map((r) => (
            <FrameCard key={r.frame_id} result={r} />
          ))}
        </div>

        {!loading && query && results.length === 0 && (
          <div className="text-center py-12">
            <p className="text-[var(--ws-text-muted)]">
              No results found for "{query}"
            </p>
          </div>
        )}

        {!query && (
          <div className="text-center py-16">
            <p className="text-[var(--ws-text-muted)] text-lg mb-2">
              Search everything you've seen
            </p>
            <p className="text-[var(--ws-text-muted)] text-sm">
              Full-text search across all captured screen content, window titles, and URLs.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
