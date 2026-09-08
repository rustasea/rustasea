//! Paginator HTML views — `bootstrap-3` rendering (FR-509).
//!
//! [`bootstrap_3`] renders a `Paginator<T>` as Bootstrap-3 pagination markup;
//! [`paginator_view`] is the string-helper form used by the `make:test`
//! scaffold and snapshot tests. Views are deterministic and dependency-free
//! so CI can snapshot them without a browser.

use rustavel_orm::Paginator;

/// Render a paginator as Bootstrap-3 pagination HTML.
///
/// Produces a `<ul class="pagination">` with prev/next arrows and numbered
/// page links; the current page is marked `class="active"`. An empty page
/// range yields an empty string (callers may surface
/// [`crate::TestError::PaginatorMissing`] instead).
pub fn bootstrap_3<T>(paginator: &Paginator<T>) -> String {
    let Paginator {
        current_page,
        per_page,
        total,
        ..
    } = *paginator;
    if per_page == 0 || total == 0 {
        return String::new();
    }

    let mut out = String::from("<ul class=\"pagination\">\n");

    // Previous arrow (disabled on the first page).
    if current_page > 1 {
        out.push_str(&format!(
            "  <li><a href=\"?page={}\" rel=\"prev\">&laquo;</a></li>\n",
            current_page - 1
        ));
    } else {
        out.push_str("  <li class=\"disabled\"><span>&laquo;</span></li>\n");
    }

    let last = paginator.last_page.max(1);
    for page in 1..=last {
        if page == current_page {
            out.push_str(&format!(
                "  <li class=\"active\"><span>{page} <span class=\"sr-only\">(current)</span></span></li>\n"
            ));
        } else {
            out.push_str(&format!("  <li><a href=\"?page={page}\">{page}</a></li>\n"));
        }
    }

    // Next arrow (disabled on the last page).
    if current_page < last {
        out.push_str(&format!(
            "  <li><a href=\"?page={}\" rel=\"next\">&raquo;</a></li>\n",
            current_page + 1
        ));
    } else {
        out.push_str("  <li class=\"disabled\"><span>&raquo;</span></li>\n");
    }

    out.push_str("</ul>\n");
    out
}

/// Render a paginator view, returning the HTML or `PaginatorMissing`.
///
/// `paginator_view("bootstrap-3", &paginator)` dispatches on the view name;
/// unknown view names render the plain `bootstrap-3` layout for forward
/// compatibility (additional themes land with the resource/view layer).
pub fn paginator_view<T>(view: &str, paginator: &Paginator<T>) -> crate::Result<String> {
    let html = match view {
        "bootstrap-3" => bootstrap_3(paginator),
        other => {
            return Err(crate::TestError::Setup(format!(
                "unknown paginator view `{other}`"
            )));
        }
    };
    if html.is_empty() {
        return Err(crate::TestError::PaginatorMissing);
    }
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies bootstrap-3 markup for a mid-page paginator.
    #[test]
    fn renders_middle_page() {
        let p = Paginator::new(vec![1u8; 10], 2, 10, 25);
        let html = bootstrap_3(&p);
        assert!(html.contains("class=\"pagination\""));
        assert!(html.contains("?page=1\" rel=\"prev\""));
        assert!(html.contains("class=\"active\""));
        assert!(html.contains("?page=3\" rel=\"next\""));
        assert!(html.contains(">3</a>"));
    }

    /// Verifies the first page disables the prev arrow.
    #[test]
    fn first_page_disables_prev() {
        let p = Paginator::new(vec![1u8; 10], 1, 10, 40);
        let html = bootstrap_3(&p);
        assert!(html.contains("li class=\"disabled\"><span>&laquo;</span>"));
        assert!(!html.contains("rel=\"prev\""));
        assert!(html.contains("?page=2\" rel=\"next\""));
        assert!(html.contains(">4</a>"), "last page link missing: {html}");
    }

    /// Verifies paginator_view surfaces PaginatorMissing on no data.
    #[test]
    fn empty_paginator_is_missing() {
        let p = Paginator::new(Vec::<u8>::new(), 1, 15, 0);
        let err = paginator_view("bootstrap-3", &p).unwrap_err();
        assert!(matches!(err, crate::TestError::PaginatorMissing));
    }

    /// Verifies the last page disables the next arrow.
    #[test]
    fn last_page_disables_next() {
        let p = Paginator::new(vec![1u8; 3], 2, 3, 6);
        let html = bootstrap_3(&p);
        assert!(html.contains("li class=\"disabled\"><span>&raquo;</span>"));
        assert!(!html.contains("rel=\"next\""));
    }
}
