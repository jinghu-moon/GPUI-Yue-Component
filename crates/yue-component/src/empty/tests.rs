use super::*;
use crate::{Accent, ThemeMode, YueButton, YueList, YueListItem};
use gpui_kit::base::{
    Root, TestSupportExt as _,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, ParentElement as _, Render, Role,
    StatefulInteractiveElement as _, Styled as _, TestAppContext, VisualTestContext, Window, div,
    px,
};
use std::sync::Arc;

struct Probe {
    width: f32,
    axis: Axis,
    inset: bool,
    size: EmptySize,
    mode: u8,
    count: usize,
    clicks: usize,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let empty = YueEmpty::new("empty")
            .size(self.size)
            .layout(self.axis)
            .inset(self.inset);
        let empty = match self.mode {
            1 => empty.actions(YueButton::new("action", "创建").on_click(cx.listener(|p, _, _, cx| {
                p.clicks += 1;
                cx.notify();
            }))).description("说明").title("标题"),
            2 => empty.description("长中文说明需要在窄面板中自然换行。Long English guidance andaverylongcontinuousidentifierwithoutspaces should remain readable.")
                .title("长标题 Long title"),
            3 => empty.hide_media().hide_description().title(" "),
            4 => empty.icon(crate::outline::Search).media(div().w(px(37.)).h(px(19.))).description(""),
            5..=7 => {
                let phase = self.mode;
                let source = ImageSource::Custom(Arc::new(move |_, _| match phase {
                    5 => None,
                    6 => Some(Err(gpui_kit::ImageCacheError::Asset("expected fixture failure".into()))),
                    _ => Some(Ok(Arc::new(gpui_kit::RenderImage::new(vec![image::Frame::new(
                        image::RgbaImage::new(12, 6),
                    )])))),
                }));
                empty.image(source).description("图片说明")
            }
            8 => empty.hide_media().description("被替换").hide_description()
                .description_content(div().id("rich-text").test_support().role(Role::Label)
                    .aria_label("富内容说明").child("富内容说明"))
                .actions(YueButton::new("action", "刷新")),
            _ => empty,
        };
        let content = if self.mode == 9 {
            YueList::new("list")
                .empty(empty.inset(false))
                .children(
                    (0..self.count)
                        .map(|ix| YueListItem::new(format!("item-{ix}")).content("项目")),
                )
                .into_any_element()
        } else {
            empty.into_any_element()
        };
        div().p_4().w(px(self.width)).child(content)
    }
}

fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| Probe {
            width: 400.,
            axis: Axis::Vertical,
            inset: false,
            size: EmptySize::Medium,
            mode: 0,
            count: 0,
            clicks: 0,
        });
        saved = Some(view.clone());
        Root::new(view, window, cx)
    });
    draw(cx);
    (saved.unwrap(), cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

fn find(window: &Window, id: ElementId) -> Option<ElementSnapshot> {
    snapshots(window)
        .into_iter()
        .find(|n| n.path().last() == Some(&id))
}
fn node(window: &Window, part: &str) -> ElementSnapshot {
    find(
        window,
        ElementId::from((ElementId::from("empty"), part.to_owned())),
    )
    .unwrap_or_else(|| panic!("missing empty/{part}"))
}

#[gpui_kit::test]
fn slots_keep_reading_order_and_hidden_content_has_no_layout_gap(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    cx.update(|w, _| {
        assert_eq!(node(w, "description").label(), Some("暂无数据"));
        assert!(node(w, "media").bounds().bottom() < node(w, "description").bounds().top());
        assert_eq!(find(w, "empty".into()).unwrap().role(), None);
    });
    view.update(cx, |p, cx| {
        p.mode = 1;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| {
        let m = node(w, "media").bounds();
        let t = node(w, "title").bounds();
        let d = node(w, "description").bounds();
        let a = node(w, "actions").bounds();
        assert!(m.bottom() < t.top() && t.bottom() < d.top() && d.bottom() < a.top());
        assert_eq!(node(w, "title").role(), Some(Role::Label));
        assert_eq!(node(w, "description").label(), Some("说明"));
    });
    view.update(cx, |p, cx| {
        p.mode = 3;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| {
        assert_eq!(
            find(w, "empty".into()).unwrap().bounds().size.height,
            px(0.)
        );
        assert!(
            !snapshots(w).iter().any(|n| n.path().last()
                == Some(&ElementId::from((ElementId::from("empty"), "content"))))
        );
    });
}

#[gpui_kit::test]
fn intrinsic_media_and_horizontal_text_do_not_stretch_or_overflow(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |p, cx| {
        p.mode = 4;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| {
        let media = node(w, "media").bounds();
        assert_eq!(media.size, gpui_kit::size(px(37.), px(19.)));
        assert_eq!(node(w, "content").bounds().size.height, px(19.));
    });
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for rem in [16., 24.] {
            view.update(cx, |p, cx| {
                p.mode = 2;
                p.width = 220.;
                p.axis = axis;
                cx.notify();
            });
            cx.update(|w, _| w.set_rem_size(px(rem)));
            draw(cx);
            cx.update(|w, _| {
                let root = find(w, "empty".into()).unwrap().bounds();
                let content = node(w, "content").bounds();
                let description = node(w, "description").bounds();
                assert!(content.right() <= root.right() + px(0.1));
                assert!(description.right() <= root.right() + px(0.1));
                assert!(description.size.height > px(20.) * (rem / 16.));
                if axis == Axis::Horizontal {
                    assert!(node(w, "media").bounds().right() < description.left());
                }
            });
        }
    }
}

#[gpui_kit::test]
fn density_and_inset_scale_without_changing_inner_gaps(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    for (density, icon, padding) in [(EmptySize::Small, 24., 12.), (EmptySize::Medium, 32., 24.)] {
        view.update(cx, |p, cx| {
            p.size = density;
            p.inset = false;
            cx.notify();
        });
        draw(cx);
        let before = cx.update(|w, _| find(w, "empty".into()).unwrap().bounds().size.height);
        view.update(cx, |p, cx| {
            p.inset = true;
            cx.notify();
        });
        draw(cx);
        cx.update(|w, _| {
            assert_eq!(node(w, "media").bounds().size.width, px(icon));
            assert_eq!(
                find(w, "empty".into()).unwrap().bounds().size.height,
                before + px(2. * padding)
            );
        });
    }
}

#[gpui_kit::test]
fn image_pending_failure_and_success_share_stable_bounds(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let mut baseline = None;
    for mode in [5, 6, 7] {
        view.update(cx, |p, cx| {
            p.mode = mode;
            cx.notify();
        });
        draw(cx);
        cx.update(|w, _| {
            let bounds = (node(w, "media").bounds(), node(w, "description").bounds());
            assert_eq!(bounds.0.size, gpui_kit::size(px(64.), px(64.)));
            if let Some(baseline) = baseline {
                assert_eq!(bounds, baseline);
            } else {
                baseline = Some(bounds);
            }
            assert_eq!(
                find(
                    w,
                    ElementId::from((ElementId::from("empty"), "image-fallback"))
                )
                .is_some(),
                mode == 6
            );
        });
    }
    view.update(cx, |p, cx| {
        p.width = 70.;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| {
        let media = node(w, "media").bounds();
        assert!(media.size.width < px(64.));
        assert_eq!(media.size.width, media.size.height);
    });
}

#[gpui_kit::test]
fn commands_keep_pointer_keyboard_focus_and_rich_content_semantics(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |p, cx| {
        p.mode = 1;
        cx.notify();
    });
    draw(cx);
    let target = cx.update(|w, _| find(w, "action".into()).unwrap().bounds().center());
    cx.simulate_click(target, gpui_kit::Modifiers::default());
    draw(cx);
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 1);
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|w, _| {
        assert_eq!(find(w, "action".into()).unwrap().focused(), Some(true));
        assert_ne!(find(w, "empty".into()).unwrap().focused(), Some(true));
    });
    let key = gpui_kit::Keystroke::parse("space").unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke: key });
    draw(cx);
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 2);
    view.update(cx, |p, cx| {
        p.mode = 8;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| {
        assert_eq!(node(w, "description").role(), None);
        assert_eq!(
            find(w, "rich-text".into()).unwrap().label(),
            Some("富内容说明")
        );
        assert_eq!(find(w, "action".into()).unwrap().role(), Some(Role::Button));
    });
}

#[gpui_kit::test]
fn list_empty_is_not_a_row_and_disappears_when_first_item_arrives(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    for count in [0, 1, 0] {
        view.update(cx, |p, cx| {
            p.mode = 9;
            p.count = count;
            cx.notify();
        });
        draw(cx);
        cx.update(|w, _| {
            assert_eq!(find(w, "empty".into()).is_some(), count == 0);
            assert_eq!(
                snapshots(w)
                    .iter()
                    .filter(|n| n.role() == Some(Role::ListItem))
                    .count(),
                count
            );
        });
    }
}
