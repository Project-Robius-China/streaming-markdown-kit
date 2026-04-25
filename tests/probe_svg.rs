#![cfg(feature = "mermaid")]

#[test]
fn probe_state_diagram_renders() {
    let src = r#"stateDiagram-v2
    [*] --> 待支付: 创建订单
    待支付 --> 已取消: 超时
    待支付 --> 已支付: 支付成功
    已支付 --> [*]: 完成"#;
    match streaming_markdown_kit::render_mermaid_to_svg(src) {
        Ok(svg) => {
            eprintln!("[state] SVG length: {}", svg.len());
            let rect_count = svg.matches("<rect").count();
            let text_count = svg.matches("<text").count();
            let path_count = svg.matches("<path").count();
            eprintln!("  <rect>={rect_count}, <text>={text_count}, <path>={path_count}");
            // If SVG is < ~500 bytes it's probably just the empty <svg> shell.
            eprintln!("  non-empty render? {}", svg.len() > 500);
        }
        Err(e) => eprintln!("[state] render FAILED: {e:?}"),
    }
}

#[test]
fn probe_sequence_note_cjk_mojibake() {
    // User reports mojibake inside a yellow-framed Note node in a sequence
    // diagram (note over X). Sequence `Note` rendering is a suspect because
    // it uses a yellow fill and a distinct text path.
    let src = r#"sequenceDiagram
    participant M as 消息队列
    Note over M: 异步对账任务<br/>最终一致性保证"#;
    let svg = streaming_markdown_kit::render_mermaid_to_svg(src).expect("render");
    eprintln!("[seq-note] raw SVG length: {}", svg.len());
    // Check whether CJK chars appear literally or as escaped entities.
    let has_literal_cjk = svg.contains("异步对账任务");
    let has_escaped = svg.contains("&#");
    eprintln!("  literal '异步对账任务': {has_literal_cjk}");
    eprintln!("  any &# entity escapes:   {has_escaped}");
    for (i, m) in svg.match_indices("<text").take(5).enumerate() {
        let tail = &svg[m.0..];
        let end = tail.find("</text>").map(|e| e + 7).unwrap_or(200);
        eprintln!("  text #{i}: {}", &tail[..end.min(500)].replace('\n', " "));
    }
}

#[test]
fn probe_cylinder_with_ascii_label_mojibake() {
    // Users reported gibberish bytes in a node rendered with Cylinder shape
    // + cloud class. Test that the SVG text output actually contains the
    // literal ASCII label, not some byte-shuffled form.
    let src = r#"flowchart TD
  classDef cloud    fill:#78350f,stroke:#fbbf24,stroke-width:2px,color:#e2e8f0
  subgraph Data["数据层"]
    S3[("Object Store<br/>Backups")]:::cloud
  end"#;
    let svg = streaming_markdown_kit::render_mermaid_to_svg(src).expect("render");
    let has_object = svg.contains(">Object Store<") || svg.contains(">Object</");
    let has_backups = svg.contains(">Backups<");
    eprintln!("[mojibake] has 'Object Store' text: {has_object}, has 'Backups': {has_backups}");
    // Dump each <text>'s content so we can see if chars got mangled.
    for (i, m) in svg.match_indices("<text").take(6).enumerate() {
        let tail = &svg[m.0..];
        let end = tail.find("</text>").map(|e| e + 7).unwrap_or(200);
        eprintln!(
            "  <text> #{i}: {}",
            &tail[..end.min(500)].replace('\n', " ")
        );
    }
}

#[test]
fn probe_ampersand_multi_edge() {
    let src = "flowchart LR\n    A & B --> C\n    C --> D & E";
    let svg = streaming_markdown_kit::render_mermaid_to_svg(src).expect("render");
    eprintln!("[ampersand multi-edge] OK ({} bytes)", svg.len());
    let rect_count = svg.matches("<rect").count();
    let path_count = svg.matches("<path").count();
    // 5 nodes + bg rect = 6; at least 4 edges (A→C, B→C, C→D, C→E) =
    // 4 <path> + 1 marker <path>
    eprintln!("  rects: {rect_count}, paths: {path_count}");
    assert!(
        svg.contains(">A<") && svg.contains(">E<"),
        "all five node labels should render"
    );
}

#[test]
fn probe_real_architecture_sizing() {
    // Mirror the actual LLM output format from image #50.
    let src = "flowchart TD\n    classDef security fill:#881337,stroke:#fb7185\n    G[\"API Gateway<br/>Kong · Nginx\"]:::security";
    let svg = streaming_markdown_kit::render_mermaid_to_svg(src).expect("render");
    eprintln!("[arch sizing]");
    // rect size
    if let Some(p) = svg.find("<rect") {
        let tail = &svg[p..];
        let end = tail.find("/>").unwrap_or(300) + 2;
        eprintln!("  first rect: {}", &tail[..end.min(300)]);
    }
    // find the node's label rect (not the bg rect)
    let rects: Vec<_> = svg.match_indices("<rect").collect();
    for (i, (p, _)) in rects.iter().enumerate() {
        let tail = &svg[*p..];
        let end = tail.find("/>").unwrap_or(300) + 2;
        eprintln!("  rect #{i}: {}", &tail[..end.min(300)]);
    }
    // text element
    if let Some(p) = svg.find("<text") {
        let tail = &svg[p..];
        let end = tail.find("</text>").map(|e| e + 7).unwrap_or(400);
        eprintln!("  first <text>..</text>:\n{}", &tail[..end.min(500)]);
    }
}

#[test]
fn probe_br_layout_detail() {
    let src = "flowchart LR\n    A[\"Load Balancer<br/>Nginx\"] --> B[\"Single Line\"]";
    let svg = streaming_markdown_kit::render_mermaid_to_svg(src).expect("render");
    eprintln!("[<br/> detail] SVG:\n{svg}\n");
    // Find A's rect size.
    let rects: Vec<&str> = svg
        .match_indices("<rect")
        .map(|(i, _)| {
            let tail = &svg[i..];
            let end = tail
                .find("/>")
                .or(tail.find(">"))
                .map(|e| e + 2)
                .unwrap_or(120);
            &tail[..end.min(250)]
        })
        .collect();
    eprintln!("[<br/> detail] rects found: {}", rects.len());
    for (i, r) in rects.iter().enumerate() {
        eprintln!("  rect #{i}: {r}");
    }
}

#[test]
fn probe_cylinder_and_double_paren_quotes() {
    // rusty-mermaid mermaid syntax for cylinder/stadium/circle nodes:
    //   A[("Postgres Messages")]  -> cylinder
    //   B[["Redis Cache"]]        -> stadium
    //   C(("Kafka"))              -> circle
    // Does it strip the inner quotes on these variants?
    let src = r#"flowchart LR
    A[("Postgres Messages")] --> B[["Redis Cache"]]
    B --> C(("Kafka"))"#;
    match streaming_markdown_kit::render_mermaid_to_svg(src) {
        Ok(svg) => {
            eprintln!("[non-bracket]");
            // Find each <text>'s inner content
            for (i, chunk) in svg.match_indices("<text").take(6).enumerate() {
                let start = chunk.0;
                let tail = &svg[start..];
                let end_rel = tail
                    .find("</text>")
                    .map(|e| e + "</text>".len())
                    .unwrap_or(200);
                let t = &tail[..end_rel.min(300)];
                let inner_start = t.find('>').map(|p| p + 1).unwrap_or(0);
                let inner_end = t.find("</text>").unwrap_or(t.len());
                let inner = &t[inner_start..inner_end];
                eprintln!("  node #{i}: content = {inner:?}");
            }
        }
        Err(e) => eprintln!("[non-bracket] FAILED: {e:?}"),
    }
}

#[test]
fn probe_quoted_label_stripping() {
    // Does rusty-mermaid strip the surrounding quotes from `A["text"]`?
    // If it leaves them in, node labels render with literal `"..."` quotes.
    let src = "flowchart LR\n    A[\"Hello\"] --> B[world]";
    let svg = streaming_markdown_kit::render_mermaid_to_svg(src).expect("render");
    let quoted_hello_present = svg.contains("\"Hello\"") || svg.contains("&quot;Hello&quot;");
    let bare_hello_present = svg.contains(">Hello<") || svg.contains(">Hello ");
    let world_present = svg.contains(">world<") || svg.contains(">world ");
    eprintln!("[quote-strip]");
    eprintln!("  svg has literal \\\"Hello\\\": {quoted_hello_present}");
    eprintln!("  svg has bare Hello:        {bare_hello_present}");
    eprintln!("  svg has world (control):   {world_present}");
    // Dump each text element so we can see exact content rendered.
    for (i, chunk) in svg.match_indices("<text").take(4).enumerate() {
        let start = chunk.0;
        let tail = &svg[start..];
        let end_rel = tail
            .find("</text>")
            .map(|e| e + "</text>".len())
            .unwrap_or(200);
        eprintln!(
            "  text #{i}: {}",
            &tail[..end_rel.min(300)].replace('\n', " ")
        );
    }
}

#[test]
fn probe_arrowhead_markers() {
    let src = "flowchart LR\n    A --> B";
    let svg = streaming_markdown_kit::render_mermaid_to_svg(src).expect("render");
    eprintln!("[markers] full SVG ({} bytes):\n{}", svg.len(), svg);
}

#[test]
fn probe_classdef_support() {
    let src = "flowchart LR\n    classDef frontend fill:#083344,stroke:#22d3ee\n    classDef db fill:#4c1d95,stroke:#a78bfa\n    A[Web UI]:::frontend --> B[Cache]:::db";
    match streaming_markdown_kit::render_mermaid_to_svg(src) {
        Ok(svg) => {
            eprintln!("[classDef] render OK ({} bytes)", svg.len());
            let has_cyan = svg.contains("#22d3ee") || svg.contains("22d3ee");
            let has_violet = svg.contains("#a78bfa") || svg.contains("a78bfa");
            eprintln!("  -> cyan #22d3ee present? {has_cyan}");
            eprintln!("  -> violet #a78bfa present? {has_violet}");
            eprintln!(
                "  -> {}",
                if has_cyan && has_violet {
                    "classDef SUPPORTED"
                } else {
                    "classDef NOT honoured"
                }
            );
        }
        Err(e) => eprintln!("[classDef] render FAILED: {e:?}"),
    }
}

#[test]
fn probe_subgraph_support() {
    let src = "flowchart TD\n    subgraph UI[前端层]\n      A[Web]\n      B[Mobile]\n    end\n    subgraph API[后端层]\n      C[Gateway]\n    end\n    A --> C\n    B --> C";
    match streaming_markdown_kit::render_mermaid_to_svg(src) {
        Ok(svg) => {
            eprintln!("[subgraph] render OK ({} bytes)", svg.len());
            let has_label_ui = svg.contains("前端层") || svg.contains("UI");
            let has_label_api = svg.contains("后端层") || svg.contains("API");
            // Presence of a second-level nested rect suggests subgraph
            let rect_count = svg.matches("<rect").count();
            eprintln!("  -> rect count: {rect_count}");
            eprintln!("  -> UI label present: {has_label_ui}, API label present: {has_label_api}");
        }
        Err(e) => eprintln!("[subgraph] render FAILED: {e:?}"),
    }
}

#[test]
fn probe_br_in_label() {
    let src = "flowchart LR\n    A[\"主标题<br/>副标题\"] --> B[单行]";
    match streaming_markdown_kit::render_mermaid_to_svg(src) {
        Ok(svg) => {
            eprintln!("[<br/>] render OK ({} bytes)", svg.len());
            let main_present = svg.contains("主标题");
            let sub_present = svg.contains("副标题");
            // If rendered as 2 tspan or 2 text, we have multiline
            let tspan_count = svg.matches("<tspan").count();
            let text_nodes_with_content = svg.matches("<text").count();
            eprintln!("  -> '主标题' present: {main_present}, '副标题' present: {sub_present}");
            eprintln!("  -> <tspan> count: {tspan_count}, <text> count: {text_nodes_with_content}");
        }
        Err(e) => eprintln!("[<br/>] render FAILED: {e:?}"),
    }
}
