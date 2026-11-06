import { useCallback, useEffect, useState } from "react";
import { readPageFromHash, type PageId } from "./navigation";

/** Hash-based page navigation so the shell behaves like a desktop app and
 * supports browser-style back/forward history without a router dependency. */
export function useAppNavigation() {
  const [page, setPage] = useState<PageId>(() => readPageFromHash());

  useEffect(() => {
    if (!window.location.hash) {
      window.history.replaceState(null, "", "#/overview");
    }

    const handleHashChange = () => setPage(readPageFromHash());
    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  const navigate = useCallback((next: PageId) => {
    if (readPageFromHash() === next) {
      setPage(next);
      return;
    }

    window.location.hash = `/${next}`;
  }, []);

  return { page, navigate };
}
