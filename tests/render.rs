use ublx::render::viewers::image::label_body_error;
use ublx::ui::UI_GLYPHS;

#[test]
fn label_body_error_includes_markdown_image_glyph() {
    let s = label_body_error("not found");
    assert!(s.contains("not found"));
    assert!(s.contains(UI_GLYPHS.markdown_image));
}
