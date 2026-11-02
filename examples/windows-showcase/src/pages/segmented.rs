use super::super::*;
use yue_component::{
    SegmentedDistribution, SegmentedShape, SegmentedSize, SegmentedVariant, YueSegmented,
    YueSegmentedOption,
};

fn periods() -> Vec<YueSegmentedOption> {
    [
        ("day", "日"),
        ("week", "周"),
        ("month", "月"),
        ("year", "年"),
    ]
    .into_iter()
    .map(|(v, l)| YueSegmentedOption::new(v, l))
    .collect()
}
fn varying() -> Vec<YueSegmentedOption> {
    [("day", "日"), ("week", "一整周"), ("year", "按年 · 省 20%")]
        .into_iter()
        .map(|(v, l)| YueSegmentedOption::new(v, l))
        .collect()
}
fn demo(
    theme: YueTheme,
    view: &Showcase,
    cx: &Context<Showcase>,
    slot: usize,
    label: &'static str,
    options: Vec<YueSegmentedOption>,
) -> YueSegmented {
    YueSegmented::new(
        format!("segmented-demo-{slot}"),
        theme,
        view.segmented_values[slot].clone(),
    )
    .label(label)
    .options(options)
    .on_change(cx.listener(move |view, value: &SharedString, _, cx| {
        view.segmented_values[slot] = value.clone();
        view.segmented_changes[slot] += 1;
        cx.notify();
    }))
}
fn column() -> gpui_kit::Div {
    div().flex().flex_col().items_start().gap_6()
}
fn value(theme: YueTheme, view: &Showcase, slot: usize) -> impl IntoElement {
    readout(
        theme,
        format!(
            "当前选择：{} · 变更 {} 次",
            view.segmented_values[slot], view.segmented_changes[slot]
        ),
    )
}

pub(crate) fn segmented_content(
    theme: YueTheme,
    view: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let mut page=column().debug_selector(||"segmented-content".into()).w_full().p_8()
        .child(div().text_2xl().child("Segmented 分段控制器"))
        .child(readout(theme,"少量互斥选项，选中块连续移动；左右键、Home/End 切换，Tab 进入或离开整组".into()))
        .child(section(theme,"基础用法","中性圆角矩形、等宽与单项禁用",column()
            .child(demo(theme,view,cx,0,"时间粒度",periods()).distribution(SegmentedDistribution::Equal).width(gpui_kit::rems(22.)))
            .child(value(theme,view,0))
            .child(demo(theme,view,cx,1,"视图模式",vec![
                YueSegmentedOption::new("list","列表"),YueSegmentedOption::new("grid","网格"),
                YueSegmentedOption::new("board","看板").disabled(true),YueSegmentedOption::new("chart","图表"),
            ]).distribution(SegmentedDistribution::Equal).width(gpui_kit::rems(25.)))
            .child(value(theme,view,1))))
        .child(section(theme,"强调色胶囊","按内容分配宽度，长标签占用更多空间",column()
            .child(demo(theme,view,cx,2,"定价周期",vec![YueSegmentedOption::new("monthly","按月"),YueSegmentedOption::new("yearly","按年 · 省 20%")])
                .variant(SegmentedVariant::Accent).shape(SegmentedShape::Pill))
            .child(value(theme,view,2))))
        .child(section(theme,"宽度分配","默认自然宽度；Equal 取最长项等宽，显式宽度下均分可用空间",column().w_full()
            .child(demo(theme,view,cx,3,"Content · 按内容宽度",varying()))
            .child(demo(theme,view,cx,4,"Equal · 自然等宽",varying()).distribution(SegmentedDistribution::Equal))
            .child(demo(theme,view,cx,5,"Equal · 显式宽度",varying()).distribution(SegmentedDistribution::Equal).width(gpui_kit::rems(30.)))
            .child(YueSegmented::new("segmented-full",theme,view.segmented_values[5].clone())
                .label("Equal · 父容器全宽").options(varying()).distribution(SegmentedDistribution::Equal).width(gpui_kit::relative(1.))
                .on_change(cx.listener(|view,value:&SharedString,_,cx|{view.segmented_values[5]=value.clone();view.segmented_changes[5]+=1;cx.notify();})))
            .child(demo(theme,view,cx,13,"过窄约束 · 内容最小宽度优先",varying()).distribution(SegmentedDistribution::Equal).width(gpui_kit::rems(8.)))))
        .child(section(theme,"尺寸与形状","Small / Medium / Large；外观和轮廓分别设置",column()
            .child(demo(theme,view,cx,6,"Small",periods()).size(SegmentedSize::Small))
            .child(demo(theme,view,cx,7,"Medium · 中性胶囊",periods()).shape(SegmentedShape::Pill))
            .child(demo(theme,view,cx,8,"Large · 强调色圆角",periods()).size(SegmentedSize::Large).variant(SegmentedVariant::Accent))))
        .child(section(theme,"图标内容","Tabler SVG 与文本同坐标绘制；纯图标仍具有可访问名称和提示",column()
            .child(demo(theme,view,cx,9,"图标加文本",vec![YueSegmentedOption::new("list","列表").icon(outline::List),YueSegmentedOption::new("grid","网格").icon(outline::LayoutGrid)])
                .variant(SegmentedVariant::Accent))
            .child(demo(theme,view,cx,10,"纯图标",vec![YueSegmentedOption::icon_only("list",outline::List,"列表视图"),YueSegmentedOption::icon_only("grid",outline::LayoutGrid,"网格视图")])
                .shape(SegmentedShape::Pill).variant(SegmentedVariant::Accent))))
        .child(section(theme,"边界状态","禁用保留当前选择；不存在的 value 不自动选第一项，也不触发回调",column()
            .child(demo(theme,view,cx,11,"整组禁用",periods()).disabled(true))
            .child(demo(theme,view,cx,14,"已选项禁用",vec![YueSegmentedOption::new("day","日（不可选）").disabled(true),YueSegmentedOption::new("week","周")]))
            .child(demo(theme,view,cx,15,"全部选项禁用",vec![YueSegmentedOption::new("day","日").disabled(true),YueSegmentedOption::new("week","周").disabled(true)]))
            .child(demo(theme,view,cx,12,"外部更新 value",periods()))
            .child(value(theme,view,12))
            .child(YueButton::text("segmented-set-external",theme,"外部切换为“年”").on_click(cx.listener(|view,_,_,cx|{view.segmented_values[12]="year".into();cx.notify();})))
            .child(YueSegmented::new("segmented-empty",theme,"day").label("空选项 · 保留说明"))))
        .child(section(theme,"动画与主题","200ms 移动并改变宽度；快速反向从当前绘制位置继续。侧栏“界面动效”关闭时立即就位",column()
            .child(readout(theme,"尝试快速切换“按月”和“按年”，然后调整窗口；重排不会从旧尺寸飞入。以下四组共享时间粒度值，事件各只请求一次。".into()))));
    let mut profiles = column();
    for (ix, (mode, accent, label)) in [
        (ThemeMode::Light, Accent::Azure, "Light / Azure"),
        (ThemeMode::Dark, Accent::Azure, "Dark / Azure"),
        (ThemeMode::Light, Accent::Neutral, "Light / Neutral"),
        (ThemeMode::Dark, Accent::Neutral, "Dark / Neutral"),
    ]
    .into_iter()
    .enumerate()
    {
        let profile = YueTheme::new(mode, accent);
        profiles = profiles.child(
            div()
                .flex()
                .flex_col()
                .items_start()
                .gap_4()
                .p_4()
                .rounded_md()
                .bg(gpui_kit::rgb(profile.page()))
                .text_color(gpui_kit::rgb(profile.text_primary()))
                .child(
                    YueSegmented::new(
                        format!("segmented-profile-{ix}"),
                        profile,
                        view.segmented_values[0].clone(),
                    )
                    .label(label)
                    .options(periods())
                    .variant(SegmentedVariant::Accent)
                    .shape(SegmentedShape::Pill)
                    .on_change(cx.listener(
                        |view, value: &SharedString, _, cx| {
                            view.segmented_values[0] = value.clone();
                            view.segmented_changes[0] += 1;
                            cx.notify();
                        },
                    )),
                )
                .child(
                    YueSegmented::new(
                        format!("segmented-profile-disabled-{ix}"),
                        profile,
                        view.segmented_values[0].clone(),
                    )
                    .label("整组禁用")
                    .options(periods())
                    .variant(SegmentedVariant::Accent)
                    .shape(SegmentedShape::Pill)
                    .disabled(true),
                ),
        );
    }
    page = page.child(profiles);
    page
}
