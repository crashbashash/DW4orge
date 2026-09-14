//! Human-readable output. JSON goes straight through serde.

use dw4core::catalogue::category_label;
use dw4core::{EMPTY, Item, SaveView};
use dw4ipc::OpenResult;

/// Pretty JSON for `--json`.
pub fn json<T: serde::Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("payloads always serialise")
    );
}

/// Summary block for `info`.
pub fn info(result: &OpenResult) {
    let view = &result.view;
    let source = match result.source {
        dw4ipc::SourceKind::Raw => "raw save",
        dw4ipc::SourceKind::Memcard => "memory card",
    };
    let occupied = view.device.iter().filter(|id| **id != EMPTY).count();
    let disks = view.disks.iter().filter(|n| **n != 0).count();
    let bank = view.bank_items.iter().filter(|id| **id != EMPTY).count();

    println!(
        "path:       {}",
        result.path.as_deref().unwrap_or("(unsaved)")
    );
    println!("container:  {source}");
    println!(
        "checksum:   {}",
        if view.checksum_ok { "ok" } else { "MISMATCH" }
    );
    println!("species:    {}", view.species.display());
    println!("name:       {}", view.name);
    println!("level:      {}  exp: {}", view.level, view.exp);
    println!(
        "bit:        {}  xdata: {}  bank: {}",
        view.bit, view.xdata, view.bank_bit
    );
    println!("difficulty: {}", view.difficulty.label());
    println!("junk tier:  {}", view.junk_tier);
    println!("devices:    {occupied}/30  disks: {disks}/12  bank: {bank}/96");
}

/// Full field dump for `dump`.
pub fn dump(view: &SaveView) {
    println!("{view:#?}");
}

/// One line per catalogue entry.
pub fn items(items: &[Item]) {
    for item in items {
        println!(
            "0x{:04X}  {:<28}  {}",
            item.base_id,
            item.name,
            category_label(item.category.byte())
        );
    }
}
