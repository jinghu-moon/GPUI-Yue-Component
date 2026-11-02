use super::*;
use gpui_kit::base::{
    Root,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{ElementId, Modifiers, Role, TestAppContext, VisualTestContext, px};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn open(cx: &mut TestAppContext) -> (Entity<TagPage>, &mut VisualTestContext) {
    cx.update(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(true);
    });
    let mut page = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|cx| TagPage::new(w, cx));
        page = Some(view.clone());
        Root::new(view, w, cx)
    });
    cx.simulate_resize(gpui_kit::size(px(1400.), px(4000.)));
    cx.update(|w, _| w.activate_window());
    draw(cx);
    (page.unwrap(), cx)
}
fn node(cx: &mut VisualTestContext, label: &str) -> ElementSnapshot {
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.label() == Some(label) && n.visible())
            .unwrap_or_else(|| panic!("missing {label}"))
    })
}
fn part(cx: &mut VisualTestContext, tag: &str, part: &'static str) -> ElementSnapshot {
    let id: ElementId = (ElementId::from(tag.to_owned()), part).into();
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.path().last() == Some(&id))
            .unwrap_or_else(|| panic!("missing {tag}/{part}"))
    })
}
fn click(cx: &mut VisualTestContext, label: &str) {
    let p = node(cx, label).bounds().center();
    cx.simulate_click(p, Modifiers::default());
    draw(cx);
}
fn click_part(cx: &mut VisualTestContext, tag: &str, target: &'static str) {
    let p = part(cx, tag, target).bounds().center();
    cx.simulate_click(p, Modifiers::default());
    draw(cx);
}
fn choose(cx: &mut VisualTestContext, group: &str, label: &str) {
    let p = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| {
                n.label() == Some(label) && n.path().contains(&ElementId::from(group.to_owned()))
            })
            .unwrap()
            .bounds()
            .center()
    });
    cx.simulate_click(p, Modifiers::default());
    draw(cx);
}
fn key(cx: &mut VisualTestContext, text: &str) {
    let keystroke = gpui_kit::Keystroke::parse(text).unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    draw(cx);
}

#[gpui_kit::test]
fn configuration_updates_actual_tag_and_close_is_independent(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    assert_eq!(part(cx, "tag-preview", "body").role(), Some(Role::Caption));
    choose(cx, "tag-mode", "动作");
    click_part(cx, "tag-preview", "body");
    assert_eq!(page.read_with(cx, |p, _| p.actions), 1);
    choose(cx, "tag-size", "大");
    choose(cx, "tag-shape", "标记");
    assert_eq!(
        part(cx, "tag-preview", "body").bounds().size.height,
        px(30.)
    );
    choose(cx, "tag-variant", "描边");
    choose(cx, "tag-tone", "成功");
    click(cx, "前置图标");
    click(cx, "可关闭");
    click(cx, "禁用");
    assert_eq!(part(cx, "tag-preview", "body").disabled(), Some(true));
    assert_eq!(part(cx, "tag-preview", "close").disabled(), Some(true));
    click_part(cx, "tag-preview", "close");
    assert!(page.read_with(cx, |p, _| p.preview_visible));
    click(cx, "禁用");
    click_part(cx, "tag-preview", "close");
    assert_eq!(
        page.read_with(cx, |p, _| (p.actions, p.closes, p.preview_visible)),
        (1, 1, false)
    );
    click(cx, "恢复预览");
    choose(cx, "tag-mode", "可选");
    click_part(cx, "tag-preview", "body");
    assert_eq!(part(cx, "tag-preview", "body").checked(), Some(true));
    assert_eq!(
        page.read_with(cx, |p, _| (p.tone, p.variant, p.shape)),
        (TagTone::Success, TagVariant::Outline, TagShape::Mark)
    );
}

#[gpui_kit::test]
fn save_flags_are_independent_and_closing_path_does_not_activate_it(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    click_part(cx, "tag-open", "body");
    assert_eq!(
        page.read_with(cx, |p, _| (p.copy_path, p.open_folder)),
        (true, true)
    );
    click_part(cx, "tag-copy", "body");
    click_part(cx, "tag-scale-50", "body");
    assert_eq!(
        page.read_with(cx, |p, _| (p.copy_path, p.open_folder, p.scale)),
        (false, true, 50)
    );
    click_part(cx, "tag-path-0", "body");
    let actions = page.read_with(cx, |p, _| p.actions);
    let focus = page.read_with(cx, |p, _| p.paths[0].close.clone());
    cx.update(|w, cx| focus.focus(w, cx));
    key(cx, "space");
    assert_eq!(
        page.read_with(cx, |p, _| (p.paths.len(), p.actions)),
        (2, actions)
    );
    assert_eq!(part(cx, "tag-path-1", "body").focused(), Some(true));
    assert_eq!(
        page.read_with(cx, |p, _| p.last_path.clone()),
        SharedString::from("截图目录")
    );
}

#[gpui_kit::test]
fn add_editor_validates_and_keyboard_submit_and_cancel_work(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    click(cx, "添加标签");
    key(cx, "enter");
    assert_eq!(
        page.read_with(cx, |p, _| p.error.clone()),
        Some("请输入标签名称".into())
    );
    let input = page.read_with(cx, |p, _| p.input.clone());
    cx.update(|window, cx| input.update(cx, |s, cx| s.set_value("截图目录", window, cx)));
    draw(cx);
    key(cx, "enter");
    assert_eq!(
        page.read_with(cx, |p, _| p.error.clone()),
        Some("此标签已存在".into())
    );
    cx.update(|window, cx| input.update(cx, |s, cx| s.set_value("收藏目录", window, cx)));
    draw(cx);
    key(cx, "enter");
    assert_eq!(
        page.read_with(cx, |p, _| (p.paths.len(), p.adding)),
        (4, false)
    );
    assert_eq!(part(cx, "tag-path-3", "body").focused(), Some(true));
    click(cx, "添加标签");
    key(cx, "escape");
    assert!(!page.read_with(cx, |p, _| p.adding));
    assert_eq!(node(cx, "添加标签").focused(), Some(true));
}

#[gpui_kit::test]
fn one_active_filter_and_multiple_style_rows_remain_controlled(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    click_part(cx, "tag-single-2", "body");
    assert_eq!(page.read_with(cx, |p, _| p.single), Some(2));
    assert_eq!(part(cx, "tag-single-0", "body").checked(), Some(false));
    click_part(cx, "tag-single-2", "body");
    assert_eq!(page.read_with(cx, |p, _| p.single), None);
    click_part(cx, "tag-check-1-1", "body");
    assert_eq!(page.read_with(cx, |p, _| p.checks[1]), [true, true, false]);
    click_part(cx, "tag-check-1-2", "body");
    assert_eq!(page.read_with(cx, |p, _| p.checks[1]), [true, true, false]);
}
