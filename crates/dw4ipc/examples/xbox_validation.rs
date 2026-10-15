//! Create a controlled Xbox edit for retail validation, using the app backend.
use dw4ipc::{EditorSession, Mode};
use std::path::Path;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(args.len() >= 3, "input output [selection]");
    let mut session = EditorSession::new_session();
    let opened = session
        .open_with_selection(Path::new(&args[1]), None, args.get(3).map(String::as_str))
        .unwrap();
    let mut edits = opened.view.to_edit_set();
    edits.name = "V04TEST".into();
    edits.bit = 234567;
    edits.xdata = 4321;
    edits.level = 17;
    edits.exp = dw4core::level_threshold(17) as u32;
    edits.tech = [2, 3, 4, 5, 6, 7, 8, 9, 10];
    edits.upcnt = [1; 11];
    edits.junk = 1000;
    edits.device[3] = 0x1000;
    edits.device[4] = 0x2000;
    edits.device[5] = 0x0019;
    edits.armor = 3;
    edits.sub = 4;
    edits.weapons[0] = 5;
    edits.wmods[0] = Some(0x3000);
    edits.amods[0] = Some(0x3005);
    edits.disks = [1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2, 3];
    edits.bank_bit = 654321;
    edits.bank_items[0] = 0x0019;
    edits.bank_items[95] = 0x1000;
    // Use a documented persistent milestone and its difficulty mirror.
    edits.story.push(dw4core::StoryEdit::flag(
        dw4core::Difficulty::Normal,
        20,
        true,
    ));
    let result = session
        .save_as(Path::new(&args[2]), &edits, Mode::Normal)
        .unwrap();
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
