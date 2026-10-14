//! Catalogue search for the `items` verb and a future item picker.

use dw4core::{Category, Item, get_catalogue};

/// Catalogue entries matching an optional name fragment and category.
///
/// `query` is matched case-insensitively against the display name. Results keep
/// the catalogue's base-id order.
#[must_use]
pub fn catalogue_search(
    query: Option<&str>,
    category: Option<Category>,
    limit: usize,
) -> Vec<Item> {
    let needle = query.map(str::to_lowercase);
    get_catalogue()
        .iter()
        .filter(|item| {
            needle
                .as_deref()
                .is_none_or(|n| item.name.to_lowercase().contains(n))
        })
        .filter(|item| category.is_none_or(|c| item.category == c))
        .take(limit)
        .cloned()
        .collect()
}
