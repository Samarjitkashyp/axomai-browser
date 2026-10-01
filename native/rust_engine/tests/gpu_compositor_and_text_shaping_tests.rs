use axomai_engine::layout::{BoxType, LayoutBox};
use axomai_engine::painter::{DamageRegion, LayerTree, ScriptType, TextShaper};
use std::collections::HashMap;

#[test]
#[ignore] // LayerTree::to_gpu_commands() not yet producing output
fn test_layer_tree_and_damage_regions() {
    let mut root_box = LayoutBox::new(BoxType::Block, HashMap::new(), String::new());
    root_box.x = 0.0;
    root_box.y = 0.0;
    root_box.width = 1024.0;
    root_box.height = 768.0;

    let mut transformed_child = LayoutBox::new(BoxType::Block, HashMap::new(), String::new());
    transformed_child.transform = "rotate(45deg)".to_string();
    transformed_child.x = 50.0;
    transformed_child.y = 50.0;
    transformed_child.width = 200.0;
    transformed_child.height = 100.0;
    root_box.children.push(transformed_child);

    let layer_tree = LayerTree::from_layout_box(&root_box);
    assert_eq!(layer_tree.root.children.len(), 1, "Should decompose transformed child into child Layer");

    let gpu_cmds = layer_tree.to_gpu_commands();
    assert!(!gpu_cmds.is_empty(), "GPU command stream should not be empty");

    let r1 = DamageRegion { x: 0.0, y: 0.0, width: 100.0, height: 100.0 };
    let r2 = DamageRegion { x: 50.0, y: 50.0, width: 100.0, height: 100.0 };
    assert!(r1.intersects(&r2), "Damage regions should intersect");
}

#[test]
fn test_assamese_and_indic_text_shaping() {
    // Assamese: অসমীয়া (Oxomiya)
    let assamese_text = "অসমীয়া";
    let script = TextShaper::detect_script(assamese_text);
    assert_eq!(script, ScriptType::AssameseBengali);

    let shaped = TextShaper::shape_text(assamese_text, 16.0);
    assert!(!shaped.is_rtl);
    assert!(shaped.total_advance > 0.0);
    assert!(!shaped.clusters.is_empty());

    // Devanagari: नमस्कार
    let hindi_text = "नमस्कार";
    let hindi_script = TextShaper::detect_script(hindi_text);
    assert_eq!(hindi_script, ScriptType::Devanagari);

    let hindi_shaped = TextShaper::shape_text(hindi_text, 16.0);
    assert!(hindi_shaped.clusters.iter().any(|c| c.is_conjunct), "Should detect 'स्क' as a conjunct");

    // Arabic: مرحبا (RTL)
    let arabic_text = "مرحبا";
    let arabic_script = TextShaper::detect_script(arabic_text);
    assert_eq!(arabic_script, ScriptType::ArabicRtl);

    let arabic_shaped = TextShaper::shape_text(arabic_text, 16.0);
    assert!(arabic_shaped.is_rtl);
}

#[test]
fn test_latin_text_shaping() {
    let latin_text = "Axomai Engine v0.9.0";
    let script = TextShaper::detect_script(latin_text);
    assert_eq!(script, ScriptType::Latin);

    let shaped = TextShaper::shape_text(latin_text, 14.0);
    assert_eq!(shaped.clusters.len(), latin_text.chars().count());
}
