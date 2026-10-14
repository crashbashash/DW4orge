#[test]
fn save_size_is_two_blocks() {
    assert_eq!(dw4core::BLOCK, 0xA000);
    assert_eq!(dw4core::SAVE_SIZE, 81920);
    assert_eq!(dw4core::EMPTY, 0xFFFF_FFFF);
}
