import { useState, FormEvent } from "react";

interface Props {
  onSearch: (query: string) => void;
  loading: boolean;
}

export function SearchBar({ onSearch, loading }: Props) {
  const [input, setInput] = useState("");

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    onSearch(input);
  };

  return (
    <form onSubmit={handleSubmit} className="flex gap-2">
      <input
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        placeholder="Search everything you've seen..."
        className="flex-1 px-4 py-2.5 bg-[var(--ws-surface)] border border-[var(--ws-border)] rounded-lg text-[var(--ws-text)] placeholder-[var(--ws-text-muted)] focus:outline-none focus:border-[var(--ws-accent)] transition-colors"
      />
      <button
        type="submit"
        disabled={loading}
        className="px-5 py-2.5 bg-[var(--ws-accent)] hover:bg-[var(--ws-accent-hover)] disabled:opacity-50 text-white rounded-lg font-medium transition-colors"
      >
        {loading ? "..." : "Search"}
      </button>
    </form>
  );
}
