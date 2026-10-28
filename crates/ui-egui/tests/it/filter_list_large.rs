//! The AutoFilter menu stays responsive and complete on a column with tens of thousands of distinct
//! values: the list is built once when the menu opens and only the visible rows are laid out, so
//! searching a value and applying it still finds the right row.

use egui::{Event, Key, Modifiers, PointerButton};
use gridcraft_engine::Session;
use gridcraft_engine::core::CellRef;
use gridcraft_ui_egui::{SheetApp, grid};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

const ROWS: u32 = 30_000;

/// A1 holds a header and A2..A30001 distinct codes under an AutoFilter, with the menu opened on column A.
fn filter_menu_on_a_huge_list() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    let mut values = vec![vec![json!("Code")]];
    values.extend((1..=ROWS).map(|i| vec![json!(format!("ITEM-{i:05}"))]));
    session.execute("range.setValues", json!({"range": "A1", "values": values})).unwrap();
    session.execute("data.filter", json!({"range": format!("A1:A{}", ROWS + 1)})).unwrap();
    let mut h = crate::common::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        crate::common::app(session),
    );
    h.run_steps(4);
    let button = {
        let app = h.state();
        let sh = app.session.active().unwrap().wb.active().unwrap();
        let geo = grid::Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll);
        grid::filter_button_rect(&geo, sh, CellRef::parse("A1").unwrap()).center()
    };
    h.input_mut().events.push(Event::PointerMoved(button));
    for pressed in [true, false] {
        h.input_mut().events.push(Event::PointerButton { pos: button, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    h.run_steps(4);
    assert!(h.state().grid.filter_menu.is_some(), "the filter menu opens");
    h
}

#[test]
#[allow(clippy::disallowed_methods)] // a native-only test: wasm never runs it
fn a_huge_value_list_keeps_frames_quick_and_search_finds_the_row() {
    let mut h = filter_menu_on_a_huge_list();
    // With the list built once, frames with the menu open cost next to nothing; a per-frame rescan of
    // 30k values (or a layout of 30k checkboxes) took several seconds for these ten frames.
    let t = std::time::Instant::now();
    h.run_steps(10);
    assert!(t.elapsed() < std::time::Duration::from_millis(1500), "ten frames took {:?}", t.elapsed());

    h.input_mut().events.push(Event::Text("item-00042".into()));
    h.run_steps(3);
    h.input_mut().events.push(Event::Key {
        key: Key::Enter,
        physical_key: Some(Key::Enter),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.input_mut().events.push(Event::Key {
        key: Key::Enter,
        physical_key: Some(Key::Enter),
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(3);

    assert!(h.state().grid.filter_menu.is_none(), "Enter applies the filter and closes the menu");
    let sh = h.state().session.active().unwrap().wb.active().unwrap();
    let visible: Vec<u32> = (1..=ROWS).filter(|&r| !sh.is_row_hidden(r)).collect();
    assert_eq!(visible, vec![42], "only ITEM-00042 (sheet row 43) stays visible");
}
