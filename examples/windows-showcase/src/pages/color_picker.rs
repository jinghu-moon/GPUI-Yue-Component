use super::super::*;
use gpui_kit::{Entity, Rgba, Subscription};
use yue_component::{
    ColorFormat, ColorPickerEvent, ColorPickerSize, ColorPickerTrigger, YueColorPicker,
    YueColorPickerPanel, YueColorPickerState, YueSwatchGroup, YueSwatchOption,
};
pub(crate) struct ColorPickerPage {
    states: Vec<Entity<YueColorPickerState>>,
    changes: Vec<usize>,
    commits: Vec<usize>,
    disabled: bool,
    background: SharedString,
    _subscriptions: Vec<Subscription>,
}
impl ColorPickerPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let colors = [
            0x3366CCFF, 0xFF6655B3, 0x3366CCFF, 0x3366CCFF, 0x3366CCFF, 0x3366CCFF, 0x3366CC80,
            0x3366CC80, 0xFFFFFFFF, 0x1F1F1FFF, 0x3366CC80, 0x3366CC80, 0x3366CC80, 0x3366CC80,
            0x3366CC80, 0x3366CC00,
        ];
        let states = colors
            .into_iter()
            .enumerate()
            .map(|(ix, c)| {
                cx.new(|cx| {
                    YueColorPickerState::new(window, cx)
                        .default_value(gpui_kit::rgba(c))
                        .default_format(if ix == 7 {
                            ColorFormat::Hsl
                        } else {
                            ColorFormat::Hex
                        })
                })
            })
            .collect::<Vec<_>>();
        let subscriptions = states
            .iter()
            .enumerate()
            .map(|(ix, s)| {
                cx.subscribe(s, move |this, _, event, cx| {
                    match event {
                        ColorPickerEvent::Change(_) => this.changes[ix] += 1,
                        ColorPickerEvent::Commit(_) => this.commits[ix] += 1,
                    }
                    cx.notify();
                })
            })
            .collect();
        Self {
            changes: vec![0; states.len()],
            commits: vec![0; states.len()],
            states,
            disabled: false,
            background: "white".into(),
            _subscriptions: subscriptions,
        }
    }
}
fn presets() -> Vec<Rgba> {
    [
        0xFF3B30, 0xFF9500, 0xFFCC00, 0x4CD964, 0x5AC8FA, 0x007AFF, 0x5856D6, 0xFF2D55, 0x8E8E93, 0,
    ]
    .into_iter()
    .map(gpui_kit::rgb)
    .collect()
}
fn column() -> gpui_kit::Div {
    div().flex().flex_col().items_start().gap_4()
}
impl Render for ColorPickerPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let row = || div().flex().flex_wrap().items_start().gap_6();
        let mut basic = column();
        for (ix, label) in [(0, "填充颜色"), (1, "半透明颜色")] {
            let c = self.states[ix].read(cx).value();
            basic = basic
                .child(
                    YueColorPicker::new(format!("color-basic-{ix}"), theme, &self.states[ix])
                        .label(label)
                        .presets(presets()),
                )
                .child(readout(
                    theme,
                    format!(
                        "RGBA ({:.3}, {:.3}, {:.3}, {:.3}) · Change {} · Commit {}",
                        c.r, c.g, c.b, c.a, self.changes[ix], self.commits[ix]
                    ),
                ));
        }
        let mut sizes = row();
        for (ix, size) in [
            (2, ColorPickerSize::Small),
            (3, ColorPickerSize::Medium),
            (4, ColorPickerSize::Large),
        ] {
            sizes = sizes.child(
                YueColorPicker::new(format!("color-size-{ix}"), theme, &self.states[ix])
                    .label(format!("{size:?}"))
                    .size(size)
                    .presets(presets()),
            );
        }
        let toggle = cx.listener(|this: &mut Self, _, _, cx| {
            this.disabled = !this.disabled;
            cx.notify();
        });
        let reset = self.states[0].clone();
        let background: gpui_kit::Background = match self.background.as_str() {
            "black" => gpui_kit::rgb(0x1F1F1F).into(),
            "custom" => self.states[10].read(cx).value().into(),
            "checks" => gpui_kit::checkerboard(gpui_kit::rgb(0xCCCCCC), 4.),
            "gradient" => gpui_kit::linear_gradient(
                90.,
                gpui_kit::linear_color_stop(gpui_kit::rgb(0x3366CC), 0.),
                gpui_kit::linear_color_stop(gpui_kit::rgb(0xAA33CC), 1.),
            )
            .color_space(gpui_kit::ColorSpace::Srgb),
            _ => gpui_kit::rgb(0xFFFFFF).into(),
        };
        let mut profiles = row();
        for (ix, (name, mode, accent)) in PROFILES.into_iter().enumerate() {
            let profile = YueTheme::new(mode, accent);
            profiles = profiles.child(theme_tile(
                name,
                profile,
                YueColorPicker::new(format!("color-theme-{ix}"), profile, &self.states[11 + ix])
                    .label(name)
                    .presets(presets()),
            ));
        }
        div().debug_selector(|| "color-picker-content".into()).w_full().p_8().flex().flex_col().items_start().gap_6()
            .child(div().text_2xl().child("ColorPicker 颜色选择器"))
            .child(readout(theme,"HSV 色板 · HEX/RGB/HSL 输入 · Alpha 独立调整；即时生效，完成一次编辑再记录历史".into()))
            .child(section(theme,"基础用法","默认 160px；面板独立 300px，选择预设后保持打开",basic
                .child(YueButton::text("color-external",theme,"外部更新颜色").on_click(move |_,window,cx|reset.update(cx,|s,cx|s.set_value(gpui_kit::rgba(0xAA33CC80),window,cx))))))
            .child(section(theme,"尺寸与触发器","三档字段触发器与仅色块形式",column().child(sizes).child(YueColorPicker::new("color-compact",theme,&self.states[5]).label("紧凑色块").trigger(ColorPickerTrigger::Swatch))))
            .child(section(theme,"常驻面板","同一个编辑器内嵌；拖动时数值同步，不重建输入框",row()
                .child(YueColorPickerPanel::new("color-inline-hex",theme,&self.states[6]).label("HEX 常驻面板").presets(presets()))
                .child(YueColorPickerPanel::new("color-inline-hsl",theme,&self.states[7]).label("HSL 常驻面板").presets(presets()))))
            .child(section(theme,"禁用与宽度","禁用仍显示真实颜色；显式宽度只影响触发器",column()
                .child(YueColorPicker::new("color-always-disabled",theme,&self.states[8]).label("始终禁用").disabled(true))
                .child(YueColorPicker::new("color-disable-toggle",theme,&self.states[9]).label("可动态禁用").width(gpui_kit::rems(18.)).disabled(self.disabled))
                .child(YueButton::text("color-disabled-toggle",theme,"切换禁用").on_click(toggle))))
            .child(section(theme,"独立背景色块组","棋盘格和渐变是背景选项；自定义打开不立即改变选择",column()
                .child(YueSwatchGroup::new("color-backgrounds",theme,self.background.clone()).label("背景类型").width(gpui_kit::rems(24.))
                    .options([YueSwatchOption::solid("white","白色",gpui_kit::rgb(0xFFFFFF)),YueSwatchOption::solid("black","黑色",gpui_kit::rgb(0x1F1F1F)),YueSwatchOption::checkerboard("checks","棋盘背景",gpui_kit::rgb(0xFFFFFF),gpui_kit::rgb(0xCCCCCC)),YueSwatchOption::gradient("gradient","蓝紫渐变",90.,gpui_kit::rgb(0x3366CC),gpui_kit::rgb(0xAA33CC))])
                    .custom_color("custom","自定义颜色",&self.states[10]).on_change(cx.listener(|this,value:&SharedString,_,cx|{this.background=value.clone();cx.notify();})))
                .child(div().w(gpui_kit::rems(24.)).h_16().bg(background).border_1().border_color(gpui_kit::rgb(theme.border_default())).child(self.background.clone()))))
            .child(section(theme,"主题与透明边界","四种 profile 与 alpha=0；透明仍保留 RGB，增加 A% 后恢复原色",column().child(profiles)
                .child(YueColorPicker::new("color-zero-alpha",theme,&self.states[15]).label("完全透明").presets(presets()))))
            .child(readout(theme,"键盘：Tab 在面板内导航；色板方向键调 S/V；色相和透明度支持方向键与 Home/End；Shift 加速；Escape 先关格式菜单再关面板。".into()))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::test_support::snapshots;
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};
    fn draw(cx: &mut VisualTestContext) {
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
    }
    fn click(cx: &mut VisualTestContext, label: &str) {
        let position = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some(label) && n.visible())
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
    }
    #[gpui_kit::test]
    fn first_example_previews_and_commits_a_preset(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| ColorPickerPage::new(w, cx));
            page = Some(view.clone());
            gpui_kit::base::Root::new(view, w, cx)
        });
        draw(cx);
        click(cx, "填充颜色");
        click(cx, "#FF3B30");
        page.unwrap().read_with(cx, |page, cx| {
            assert_eq!(page.states[0].read(cx).value(), gpui_kit::rgb(0xFF3B30));
            assert_eq!((page.changes[0], page.commits[0]), (1, 1));
        });
    }
}
