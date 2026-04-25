use streaming_markdown_kit::{FenceScanError, scan_fenced_code_blocks};

#[test]
fn extracts_complete_fenced_code_blocks() {
    let blocks = scan_fenced_code_blocks("before\n```diagram\n{\"type\":\"state\"}\n```\nafter")
        .expect("complete fence should scan");

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].info, "diagram");
    assert_eq!(blocks[0].body, "{\"type\":\"state\"}\n");
}

#[test]
fn reports_unclosed_fenced_code_block() {
    let err = scan_fenced_code_blocks("```diagram\n{\"type\":\"state\"}")
        .expect_err("unclosed fence should be reported");

    assert!(matches!(err, FenceScanError::Unclosed { info } if info == "diagram"));
}

#[test]
fn handles_tilde_fences_and_info_arguments() {
    let blocks =
        scan_fenced_code_blocks("~~~diagram compact\n{}\n~~~").expect("tilde fence should scan");

    assert_eq!(blocks[0].info, "diagram");
    assert_eq!(blocks[0].info_string, "diagram compact");
    assert_eq!(blocks[0].body, "{}\n");
}
