import React, {useEffect, type ReactNode} from 'react';
import SearchPage from '@theme-original/SearchPage';

// The search plugin's results page renders its query box with `autoFocus`, so the first Tab skipped
// "Skip to main content" (UXF-005). Once the page is up, focus moves to the skip link's region, as
// Docusaurus does after a navigation; a bare blur would keep the browser's Tab starting point at the
// query box. A query already in the box (typed, or from `?q=`) keeps its focus. The query box is the
// first control after the navbar.
export default function SearchPageWrapper(props: Record<string, unknown>): ReactNode {
  useEffect(() => {
    const active = document.activeElement;
    if (!(active instanceof HTMLInputElement) || active.value !== '' || !active.closest('#__docusaurus_skipToContent_fallback')) return;
    const region = document.querySelector('a[href="#__docusaurus_skipToContent_fallback"]')?.parentElement;
    if (!region) return;
    region.setAttribute('tabindex', '-1');
    region.focus({preventScroll: true});
    region.removeAttribute('tabindex');
  }, []);
  return <SearchPage {...props} />;
}
