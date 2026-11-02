use super::super::*;

pub(crate) fn dropdown_note(
    cx: &mut Context<Showcase>,
    slot: usize,
    label: impl Into<SharedString>,
) -> impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App) + 'static {
    let label = label.into();
    cx.listener(move |showcase, _: &gpui_kit::ClickEvent, _, cx| {
        showcase.dropdown_log[slot] = Some(label.clone());
        cx.notify();
    })
}

// A row handler that toggles its value in the section's own set: what a check
// mark inside a menu needs, since the mark belongs to the caller.

pub(crate) fn dropdown_check(
    cx: &mut Context<Showcase>,
    set: usize,
    value: &'static str,
) -> impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App) + 'static {
    let value = SharedString::from(value);
    cx.listener(move |showcase, _: &gpui_kit::ClickEvent, _, cx| {
        let held = &mut showcase.dropdown_sets[set];
        match held.iter().position(|item| *item == value) {
            Some(position) => {
                held.remove(position);
            }
            None => held.push(value.clone()),
        }
        cx.notify();
    })
}

// A row handler that makes its value the whole set: what a radio mark needs.

pub(crate) fn dropdown_choose(
    cx: &mut Context<Showcase>,
    set: usize,
    value: &'static str,
) -> impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App) + 'static {
    let value = SharedString::from(value);
    cx.listener(move |showcase, _: &gpui_kit::ClickEvent, _, cx| {
        showcase.dropdown_sets[set] = vec![value.clone()];
        cx.notify();
    })
}

// Whether a section's controlled set holds this value — the same question the
// menu's mark answers.

pub(crate) fn dropdown_holds(values: &[SharedString], value: &str) -> bool {
    values.iter().any(|held| held == value)
}

// The line under an example that says which row it last ran.

pub(crate) fn dropdown_ran(
    theme: YueTheme,
    log: &[Option<SharedString>],
    slot: usize,
) -> impl IntoElement {
    readout(
        theme,
        format!("最近运行：{}", value_text(log[slot].as_ref())),
    )
}

pub(crate) fn dropdown_segments(count: usize) -> Vec<YueMenuItem> {
    (1..=count)
        .map(|index| YueMenuItem::new(format!("第 {} 段", index)))
        .collect()
}

pub(crate) fn dropdown_remote_names() -> Vec<SharedString> {
    ["李工", "阿宁", "Momo", "老周"]
        .into_iter()
        .map(SharedString::from)
        .collect()
}

// The colour grid: labels short enough to compare at a glance, which is the only
// thing `columns` is for.
const DROPDOWN_COLORS: [&str; 12] = [
    "天蓝",
    "晴青",
    "水鸭",
    "薄荷",
    "青柠",
    "橄榄",
    "琥珀",
    "赭石",
    "珊瑚",
    "胭脂",
    "紫罗兰",
    "石板",
];

pub(crate) fn dropdown_content(
    theme: YueTheme,
    showcase: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    // The async example refills itself the first time its panel opens, so the
    // spinner row is only ever the wait, never a stuck state.
    let start_load = cx.listener(|showcase, open: &bool, _, cx| {
        if !*open || showcase.dropdown_loading || !showcase.dropdown_remote.is_empty() {
            return;
        }
        showcase.dropdown_loading = true;
        cx.spawn(
            async move |this: gpui_kit::WeakEntity<Showcase>, cx: &mut gpui_kit::AsyncApp| {
                cx.background_executor().timer(DROPDOWN_ASYNC_LOAD).await;
                let Some(this) = this.upgrade() else {
                    return;
                };
                this.update(cx, |showcase, cx| {
                    showcase.dropdown_remote = dropdown_remote_names();
                    showcase.dropdown_loading = false;
                    cx.notify();
                });
                cx.refresh();
            },
        )
        .detach();
        cx.notify();
    });
    let report_open = cx.listener(|showcase, open: &bool, _, cx| {
        showcase.dropdown_open = *open;
        cx.notify();
    });
    let clear_remote = cx.listener(|showcase, _: &gpui_kit::ClickEvent, _, cx| {
        showcase.dropdown_remote.clear();
        showcase.dropdown_loading = false;
        cx.notify();
    });

    let remote: Vec<YueMenuItem> = if showcase.dropdown_loading {
        vec![YueMenuItem::loading("正在读取协作者…")]
    } else if showcase.dropdown_remote.is_empty() {
        vec![YueMenuItem::new("还没有加载：打开菜单即开始").disabled(true)]
    } else {
        showcase
            .dropdown_remote
            .iter()
            .map(|name| {
                YueMenuItem::new(name.clone())
                    .avatar(name.clone())
                    .hotkey("在线")
                    .on_click(dropdown_note(cx, DD_ASYNC, name.clone()))
            })
            .collect()
    };

    let aligns = [
        DropdownAlign::Start,
        DropdownAlign::Center,
        DropdownAlign::End,
    ];
    let mut align_examples: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, align) in aligns.into_iter().enumerate() {
        align_examples.push(
            div()
                .flex()
                .flex_col()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child(format!("align = {align:?}")),
                )
                .child(
                    YueDropdownMenu::new(
                        format!("dropdown-align-{index}"),
                        theme,
                        YueButton::icon_text(
                            format!("dropdown-align-trigger-{index}"),
                            theme,
                            outline::ChevronDown,
                            "菜单",
                        )
                        .icon_position(IconPosition::Trailing),
                    )
                    .label("对齐菜单")
                    .align(align)
                    .panel_width(gpui_kit::px(200.))
                    .items([
                        YueMenuItem::new("重命名")
                            .hotkey("F2")
                            .on_click(dropdown_note(cx, DD_ALIGN, "重命名")),
                        YueMenuItem::new("复制")
                            .icon(outline::Copy)
                            .on_click(dropdown_note(cx, DD_ALIGN, "复制")),
                        YueMenuItem::new("删除")
                            .icon(outline::Trash)
                            .hotkey("Del")
                            .danger(true)
                            .on_click(dropdown_note(cx, DD_ALIGN, "删除")),
                    ]),
                )
                .into_any_element(),
        );
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_6()
        .p_8()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_2xl().child("Dropdown"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("命令菜单、二级菜单与右键面板：同一套定位、动效与键盘行为"),
                ),
        )
        .child(section(
            theme,
            "Command menu",
            "点一下展开，选中一项后关闭并把焦点还给触发器；尾部箭头随展开转半圈，面板的箭头指着触发器中心",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-command",
                        theme,
                        YueButton::icon_text(
                            "dropdown-command-trigger",
                            theme,
                            outline::ChevronDown,
                            "选择操作…",
                        )
                        .icon_position(IconPosition::Trailing),
                    )
                    .label("选择操作")
                    .items([
                        YueMenuItem::new("重命名")
                            .hotkey("F2")
                            .on_click(dropdown_note(cx, DD_COMMAND, "重命名")),
                        YueMenuItem::new("复制")
                            .icon(outline::Copy)
                            .hotkey("Ctrl+C")
                            .on_click(dropdown_note(cx, DD_COMMAND, "复制")),
                        YueMenuItem::new("在资源管理器中显示")
                            .icon(outline::Folder)
                            .on_click(dropdown_note(cx, DD_COMMAND, "在资源管理器中显示")),
                        YueMenuItem::separator(),
                        YueMenuItem::new("导出为 PDF").disabled(true),
                        YueMenuItem::new("删除")
                            .icon(outline::Trash)
                            .hotkey("Del")
                            .danger(true)
                            .on_click(dropdown_note(cx, DD_COMMAND, "删除")),
                    ]),
                )
                .child(readout(
                    theme,
                    "「导出为 PDF」需要先选择一个文件夹：禁用只降透明度，原因写在这里而不只靠对比度。".to_string(),
                ))
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_COMMAND)),
        ))
        .child(section(
            theme,
            "Alignment",
            "align 决定面板在交叉轴上的落点；指向箭头始终对准触发器中心，滑到离圆角太近时夹进 16px 安全区",
            div().flex().items_start().gap_6().children(align_examples),
        ))
        .child(section(
            theme,
            "Edge collision and flip",
            "默认先试下方，整块放不下就翻到上方；两侧都放不下取露出最多的一侧并夹回视口，四边各避让 8px。把窗口拖窄或把这一格滚到贴边，当场能看到换边",
            div()
                .flex()
                .items_end()
                .justify_between()
                .gap_6()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-flip-auto",
                        theme,
                        YueButton::icon_text(
                            "dropdown-flip-auto-trigger",
                            theme,
                            outline::Download,
                            "向下 18 项",
                        )
                        .icon_position(IconPosition::Trailing),
                    )
                    .label("导出分段")
                    .max_height(gpui_kit::px(260.))
                    .panel_width(gpui_kit::px(220.))
                    .items(
                        dropdown_segments(18)
                            .into_iter()
                            .map(|item| item.on_click(dropdown_note(cx, DD_FLIP, "选段")))
                            .collect::<Vec<YueMenuItem>>(),
                    ),
                )
                .child(
                    YueDropdownMenu::new(
                        "dropdown-flip-above",
                        theme,
                        YueButton::text("dropdown-flip-above-trigger", theme, "钉在上方"),
                    )
                    .placement(DropdownPlacement::Above)
                    .items([
                        YueMenuItem::new("上翻一项")
                            .on_click(dropdown_note(cx, DD_FLIP, "上翻一项")),
                        YueMenuItem::new("再上翻一项")
                            .on_click(dropdown_note(cx, DD_FLIP, "再上翻一项")),
                    ]),
                )
                .child(
                    YueDropdownMenu::new(
                        "dropdown-flip-side",
                        theme,
                        YueButton::text("dropdown-flip-side-trigger", theme, "钉在右侧"),
                    )
                    .placement(DropdownPlacement::Trailing)
                    .align(DropdownAlign::End)
                    .items([
                        YueMenuItem::new("换到另一侧")
                            .on_click(dropdown_note(cx, DD_FLIP, "换到另一侧")),
                        YueMenuItem::new("保持原位")
                            .on_click(dropdown_note(cx, DD_FLIP, "保持原位")),
                    ]),
                )
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_FLIP)),
        ))
        .child(section(
            theme,
            "Submenus",
            "指针在一行上停留 80ms 才展开二级面板，移开即取消计时；二级面板的名字来自打开它的那一行",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-submenu",
                        theme,
                        YueButton::icon_text(
                            "dropdown-submenu-trigger",
                            theme,
                            outline::Share,
                            "导出为…",
                        )
                        .icon_position(IconPosition::Trailing),
                    )
                    .label("导出")
                    .items([
                        YueMenuItem::new("复制路径")
                            .icon(outline::Link)
                            .on_click(dropdown_note(cx, DD_SUBMENU, "复制路径")),
                        YueMenuItem::submenu(
                            "导出为",
                            [
                                YueMenuItem::new("PDF")
                                    .on_click(dropdown_note(cx, DD_SUBMENU, "PDF")),
                                YueMenuItem::new("Word")
                                    .on_click(dropdown_note(cx, DD_SUBMENU, "Word")),
                                YueMenuItem::new("Markdown")
                                    .on_click(dropdown_note(cx, DD_SUBMENU, "Markdown")),
                            ],
                        ),
                        YueMenuItem::submenu(
                            "发送到",
                            [
                                YueMenuItem::new("邮件")
                                    .icon(outline::Mail)
                                    .on_click(dropdown_note(cx, DD_SUBMENU, "邮件")),
                                YueMenuItem::new("日历")
                                    .icon(outline::Calendar)
                                    .on_click(dropdown_note(cx, DD_SUBMENU, "日历")),
                                YueMenuItem::new(
                                    "这一行的三级面板不会展开：菜单只到二级",
                                )
                                .disabled(true),
                            ],
                        ),
                        YueMenuItem::separator(),
                        YueMenuItem::new("不导出")
                            .on_click(dropdown_note(cx, DD_SUBMENU, "不导出")),
                    ]),
                )
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_SUBMENU)),
        ))
        .child(section(
            theme,
            "Checks and radios",
            "勾选与圆点由调用方的状态决定，点击后面板不关闭；indicator_side 决定它们落在标签的哪一侧",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_6()
                        .child(
                            YueDropdownMenu::new(
                                "dropdown-marks-trailing",
                                theme,
                                YueButton::text(
                                    "dropdown-marks-trailing-trigger",
                                    theme,
                                    "显示与主题",
                                ),
                            )
                            .label("显示与主题")
                            .items(dropdown_mark_items(cx, showcase, DD_VIEWS, DD_THEME)),
                        )
                        .child(
                            YueDropdownMenu::new(
                                "dropdown-marks-leading",
                                theme,
                                YueButton::text(
                                    "dropdown-marks-leading-trigger",
                                    theme,
                                    "标记在左侧",
                                ),
                            )
                            .indicator_side(DropdownIndicatorSide::Leading)
                            .items(dropdown_mark_items(cx, showcase, DD_VIEWS, DD_THEME)),
                        ),
                )
                .child(readout(
                    theme,
                    format!(
                        "显示：{}",
                        values_text(&showcase.dropdown_sets[DD_VIEWS])
                    ),
                ))
                .child(readout(
                    theme,
                    format!(
                        "主题：{}",
                        values_text(&showcase.dropdown_sets[DD_THEME])
                    ),
                )),
        ))
        .child(section(
            theme,
            "Long list",
            "面板到达 max_height 才在内部滚动；键盘漫游走到边界会把该行滚进视野",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-long",
                        theme,
                        YueButton::icon_text(
                            "dropdown-long-trigger",
                            theme,
                            outline::FileText,
                            "跳转到段落",
                        )
                        .icon_position(IconPosition::Trailing),
                    )
                    .label("跳转")
                    .max_height(gpui_kit::px(240.))
                    .panel_width(gpui_kit::px(220.))
                    .items(
                        dropdown_segments(30)
                            .into_iter()
                            .map(|item| item.on_click(dropdown_note(cx, DD_LONG, "跳转段落")))
                            .collect::<Vec<YueMenuItem>>(),
                    ),
                )
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_LONG)),
        ))
        .child(section(
            theme,
            "Truncation",
            "定宽面板让长标签以省略号收尾，而不是把面板推得更宽",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-truncate",
                        theme,
                        YueButton::text("dropdown-truncate-trigger", theme, "最近打开"),
                    )
                    .label("最近打开")
                    .panel_width(gpui_kit::px(240.))
                    .items([
                        YueMenuItem::new(
                            "季度复盘与下季度路线图草案（含跨部门依赖清单）",
                        )
                        .on_click(dropdown_note(cx, DD_TRUNCATE, "季度复盘草案")),
                        YueMenuItem::new(
                            "组件库动效规格：面板、开关、单选与多选的同源 token",
                        )
                        .on_click(dropdown_note(cx, DD_TRUNCATE, "动效规格")),
                        YueMenuItem::new("周报草稿").hotkey("Ctrl+P")
                            .on_click(dropdown_note(cx, DD_TRUNCATE, "周报草稿")),
                    ]),
                )
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_TRUNCATE)),
        ))
        .child(section(
            theme,
            "Keyboard and accessibility",
            "↑↓ 漫游、Home/End 到首尾、Enter 或 Space 运行、Esc 关闭并把焦点还给触发器；字母键按标签前缀跳到下一项，重复按同一个字母在命中项之间轮转",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-keyboard",
                        theme,
                        YueButton::text("dropdown-keyboard-trigger", theme, "键盘可达"),
                    )
                    .label("键盘可达")
                    .on_open_change(report_open)
                    .items([
                        YueMenuItem::new("Archive 归档")
                            .on_click(dropdown_note(cx, DD_KEYBOARD, "Archive 归档")),
                        YueMenuItem::new("Copy 复制")
                            .on_click(dropdown_note(cx, DD_KEYBOARD, "Copy 复制")),
                        YueMenuItem::new("Copy path 复制路径")
                            .on_click(dropdown_note(cx, DD_KEYBOARD, "Copy path 复制路径")),
                        YueMenuItem::new("Duplicate 创建副本")
                            .on_click(dropdown_note(cx, DD_KEYBOARD, "Duplicate 创建副本")),
                        YueMenuItem::new("Export 导出")
                            .on_click(dropdown_note(cx, DD_KEYBOARD, "Export 导出")),
                        YueMenuItem::new("Rename 重命名")
                            .on_click(dropdown_note(cx, DD_KEYBOARD, "Rename 重命名")),
                        YueMenuItem::separator(),
                        YueMenuItem::loading("正在同步：这一行不接受焦点"),
                    ]),
                )
                .child(readout(
                    theme,
                    format!(
                        "面板开合：{}；Esc 或选中一项后关闭，焦点回到触发器。",
                        if showcase.dropdown_open {
                            "开"
                        } else {
                            "关"
                        }
                    ),
                ))
                .child(readout(
                    theme,
                    "标签用拉丁词开头，是为了当场能试出 typeahead：按 c 跳到「Copy 复制」，再按一次跳到「Copy path 复制路径」。".to_string(),
                )),
        ))
        .child(section(
            theme,
            "Async loading",
            "打开时先给一行 spinner，数据到达后同一个面板换成结果；这里的等待是 spinner 转三圈",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-async",
                        theme,
                        YueButton::icon_text(
                            "dropdown-async-trigger",
                            theme,
                            outline::User,
                            "添加协作者",
                        )
                        .icon_position(IconPosition::Trailing),
                    )
                    .label("添加协作者")
                    .on_open_change(start_load)
                    .items(remote),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            YueButton::text("dropdown-async-reset", theme, "清空结果以便重看")
                                .variant(ButtonVariant::Outline)
                                .on_click(clear_remote),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "状态：{}",
                                if showcase.dropdown_loading {
                                    "加载中"
                                } else if showcase.dropdown_remote.is_empty() {
                                    "尚未加载"
                                } else {
                                    "已到达 4 项"
                                }
                            ),
                        )),
                )
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_ASYNC)),
        ))
        .child(section(
            theme,
            "Icons and avatars",
            "行首的图标与 initials 头像占同一条基线；头像只说明这是哪个人，命令名仍由标签给出",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-icons",
                        theme,
                        YueButton::text("dropdown-icons-trigger", theme, "分派给…"),
                    )
                    .label("分派")
                    .items([
                        YueMenuItem::heading("在线"),
                        YueMenuItem::new("李工")
                            .avatar("李")
                            .hotkey("空闲")
                            .on_click(dropdown_note(cx, DD_ICON, "李工")),
                        YueMenuItem::new("阿宁")
                            .avatar("宁")
                            .on_click(dropdown_note(cx, DD_ICON, "阿宁")),
                        YueMenuItem::separator(),
                        YueMenuItem::heading("离线"),
                        YueMenuItem::new("老周")
                            .avatar("周")
                            .hotkey("3 天后")
                            .on_click(dropdown_note(cx, DD_ICON, "老周")),
                        YueMenuItem::new("收件箱")
                            .icon(outline::Mail)
                            .on_click(dropdown_note(cx, DD_ICON, "收件箱")),
                        YueMenuItem::new("提醒")
                            .icon(outline::Bell)
                            .on_click(dropdown_note(cx, DD_ICON, "提醒")),
                    ]),
                )
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_ICON)),
        ))
        .child(section(
            theme,
            "Trigger seam",
            "触发器不限于按钮：with_trigger 接任意元素，开合状态交给它自己表达。面板的名字来自这里给的 label，而不是不可见的 id",
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(
                    YueDropdownMenu::with_trigger("dropdown-avatar", theme, move |open, _, _| {
                        div()
                            .size_8()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_color(gpui_kit::rgb(theme.border_default()))
                            .bg(gpui_kit::rgb(if open {
                                theme.accent_subtle()
                            } else {
                                theme.surface()
                            }))
                            .text_sm()
                            .child("SY")
                            .into_any_element()
                    })
                    .label("seeyuer 的账号菜单")
                    .align(DropdownAlign::End)
                    .items([
                        YueMenuItem::new("个人资料")
                            .icon(outline::User)
                            .on_click(dropdown_note(cx, DD_ICON, "个人资料")),
                        YueMenuItem::new("设置")
                            .icon(outline::Settings)
                            .on_click(dropdown_note(cx, DD_ICON, "设置")),
                        YueMenuItem::separator(),
                        YueMenuItem::new("退出登录")
                            .danger(true)
                            .on_click(dropdown_note(cx, DD_ICON, "退出登录")),
                    ]),
                )
                .child(readout(
                    theme,
                    "点头像展开；面板右边缘对齐头像。".to_string(),
                )),
        ))
        .child(section(
            theme,
            "Groups and rules",
            "分组标题不占键盘停靠点；rule 给标题加一条下划线，separator 是两组命令之间的一条线",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-groups",
                        theme,
                        YueButton::text("dropdown-groups-trigger", theme, "通知设置"),
                    )
                    .label("通知设置")
                    .items([
                        YueMenuItem::heading("渠道").rule(true),
                        YueMenuItem::new("邮件")
                            .icon(outline::Mail)
                            .checked(dropdown_holds(
                                &showcase.dropdown_sets[DD_CHANNELS],
                                "邮件",
                            ))
                            .keep_open(true)
                            .on_click(dropdown_check(cx, DD_CHANNELS, "邮件")),
                        YueMenuItem::new("桌面通知")
                            .icon(outline::Bell)
                            .checked(dropdown_holds(
                                &showcase.dropdown_sets[DD_CHANNELS],
                                "桌面",
                            ))
                            .keep_open(true)
                            .on_click(dropdown_check(cx, DD_CHANNELS, "桌面")),
                        YueMenuItem::heading("节奏").rule(true),
                        YueMenuItem::new("即时")
                            .radio(dropdown_holds(&showcase.dropdown_sets[DD_PACE], "即时"))
                            .keep_open(true)
                            .on_click(dropdown_choose(cx, DD_PACE, "即时")),
                        YueMenuItem::new("每天一次")
                            .radio(dropdown_holds(&showcase.dropdown_sets[DD_PACE], "每天"))
                            .keep_open(true)
                            .on_click(dropdown_choose(cx, DD_PACE, "每天")),
                        YueMenuItem::new("只在被 @ 时")
                            .radio(dropdown_holds(&showcase.dropdown_sets[DD_PACE], "提及"))
                            .keep_open(true)
                            .on_click(dropdown_choose(cx, DD_PACE, "提及")),
                        YueMenuItem::separator(),
                        YueMenuItem::new("全部关闭")
                            .danger(true)
                            .on_click(dropdown_note(cx, DD_GROUP, "全部关闭")),
                    ]),
                )
                .child(readout(
                    theme,
                    format!(
                        "渠道：{} · 节奏：{}",
                        values_text(&showcase.dropdown_sets[DD_CHANNELS]),
                        values_text(&showcase.dropdown_sets[DD_PACE])
                    ),
                ))
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_GROUP)),
        ))
        .child(section(
            theme,
            "Marks",
            "勾选（多选）与圆点（单选）共用一套指示位；两者都在面板里保持打开",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-marks",
                        theme,
                        YueButton::text("dropdown-marks-trigger", theme, "列与筛选"),
                    )
                    .label("列与筛选")
                    .items(dropdown_mark_items(cx, showcase, DD_VIEWS, DD_THEME)),
                )
                .child(readout(
                    theme,
                    format!(
                        "显示：{} · 主题：{}",
                        values_text(&showcase.dropdown_sets[DD_VIEWS]),
                        values_text(&showcase.dropdown_sets[DD_THEME])
                    ),
                ))
                .child(readout(
                    theme,
                    "aria 上分别报成 menuitemcheckbox 与 menuitemradio；半选不属于菜单，属于多选组。".to_string(),
                )),
        ))
        .child(section(
            theme,
            "Attached to an input",
            "挂在输入框尾部：面板用 align = End 与触发按钮的右边缘对齐，输入框宽度不受影响",
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    YueInput::new("dropdown-suffix-input", theme, &showcase.dropdown_suffix)
                        .width(gpui_kit::rems(22.0)),
                )
                .child(
                    YueDropdownMenu::new(
                        "dropdown-suffix",
                        theme,
                        YueButton::icon(
                            "dropdown-suffix-trigger",
                            theme,
                            outline::DotsVertical,
                            "插入变量",
                        ),
                    )
                    .label("插入变量")
                    .align(DropdownAlign::End)
                    .items([
                        YueMenuItem::new("{{姓名}}")
                            .on_click(dropdown_note(cx, DD_SUFFIX, "{{姓名}}")),
                        YueMenuItem::new("{{日期}}")
                            .on_click(dropdown_note(cx, DD_SUFFIX, "{{日期}}")),
                        YueMenuItem::new("{{链接}}")
                            .on_click(dropdown_note(cx, DD_SUFFIX, "{{链接}}")),
                    ]),
                )
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_SUFFIX)),
        ))
        .child(section(
            theme,
            "Context menu",
            "在区域里点右键：面板的角落落在指针上，没有指向箭头也没有间隙；落点仍由视口碰撞决定",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueContextMenu::new("dropdown-context", theme)
                        .label("文件")
                        .items([
                            YueMenuItem::new("打开")
                                .icon(outline::Folder)
                                .hotkey("Ctrl+O")
                                .on_click(dropdown_note(cx, DD_CONTEXT, "打开")),
                            YueMenuItem::new("重命名")
                                .hotkey("F2")
                                .on_click(dropdown_note(cx, DD_CONTEXT, "重命名")),
                            YueMenuItem::new("复制路径")
                                .icon(outline::Link)
                                .on_click(dropdown_note(cx, DD_CONTEXT, "复制路径")),
                            YueMenuItem::separator(),
                            YueMenuItem::new("删除")
                                .icon(outline::Trash)
                                .hotkey("Del")
                                .danger(true)
                                .on_click(dropdown_note(cx, DD_CONTEXT, "删除")),
                        ])
                        .child(
                            div()
                                .w_full()
                                .h(gpui_kit::px(120.))
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap_1()
                                .rounded_md()
                                .border_1()
                                .border_color(gpui_kit::rgb(theme.border_default()))
                                .bg(gpui_kit::rgb(theme.surface()))
                                .child("在这个区域里点右键")
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                                        .child("面板名叫「文件」：右键菜单没有触发器可以借名字"),
                                ),
                        ),
                )
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_CONTEXT)),
        ))
        .child(section(
            theme,
            "Colour grid",
            "columns 只给可比的小集合用：方向键按列步进，到边界夹住而不是绕回；一行仍然只有一个停靠点",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownMenu::new(
                        "dropdown-grid",
                        theme,
                        YueButton::icon_text(
                            "dropdown-grid-trigger",
                            theme,
                            outline::Palette,
                            "标注色",
                        )
                        .icon_position(IconPosition::Trailing),
                    )
                    .label("标注色")
                    .columns(4)
                    .panel_width(gpui_kit::px(240.))
                    .indicator_side(DropdownIndicatorSide::Leading)
                    .items(DROPDOWN_COLORS.into_iter().map(|label| {
                        YueMenuItem::new(label)
                            .radio(dropdown_holds(&showcase.dropdown_sets[DD_COLOR], label))
                            .keep_open(true)
                            .id(label)
                            .on_click(dropdown_choose(cx, DD_COLOR, label))
                    })),
                )
                .child(readout(
                    theme,
                    format!(
                        "选中的标注色：{}",
                        value_text(showcase.dropdown_sets[DD_COLOR].first())
                    ),
                )),
        ))
        .child(section(
            theme,
            "Split button",
            "左半运行默认命令，右半打开其余命令；两个目标两个名字，面板从右半浮出，箭头指向右半",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueDropdownSplitButton::new(
                        "dropdown-split",
                        theme,
                        YueButton::icon_text(
                            "dropdown-split-action",
                            theme,
                            outline::Download,
                            "保存",
                        )
                        .on_click(dropdown_note(cx, DD_SPLIT, "保存")),
                        "更多保存选项",
                    )
                    .label("更多保存选项")
                    .items([
                        YueMenuItem::new("保存为草稿")
                            .on_click(dropdown_note(cx, DD_SPLIT, "保存为草稿")),
                        YueMenuItem::new("另存为…")
                            .hotkey("Shift+S")
                            .on_click(dropdown_note(cx, DD_SPLIT, "另存为…")),
                        YueMenuItem::new("保存并关闭")
                            .on_click(dropdown_note(cx, DD_SPLIT, "保存并关闭")),
                        YueMenuItem::separator(),
                        YueMenuItem::new("放弃改动")
                            .danger(true)
                            .on_click(dropdown_note(cx, DD_SPLIT, "放弃改动")),
                    ]),
                )
                .child(
                    YueDropdownSplitButton::new(
                        "dropdown-split-disabled",
                        theme,
                        YueButton::text("dropdown-split-disabled-action", theme, "不可保存")
                            .disabled(true),
                        "更多保存选项",
                    )
                    .disabled(true)
                    .items([YueMenuItem::new("保存为草稿")]),
                )
                .child(dropdown_ran(theme, &showcase.dropdown_log, DD_SPLIT)),
        ))
}

// The check/radio rows the two mark examples share: the marks read the page's own
// sets, so the trailing and leading examples always agree.

pub(crate) fn dropdown_mark_items(
    cx: &mut Context<Showcase>,
    showcase: &Showcase,
    views: usize,
    theme_slot: usize,
) -> Vec<YueMenuItem> {
    [
        YueMenuItem::heading("显示"),
        YueMenuItem::new("已完成")
            .checked(dropdown_holds(&showcase.dropdown_sets[views], "已完成"))
            .keep_open(true)
            .on_click(dropdown_check(cx, views, "已完成")),
        YueMenuItem::new("归档")
            .checked(dropdown_holds(&showcase.dropdown_sets[views], "归档"))
            .keep_open(true)
            .on_click(dropdown_check(cx, views, "归档")),
        YueMenuItem::new("只看我的")
            .checked(dropdown_holds(&showcase.dropdown_sets[views], "只看我的"))
            .keep_open(true)
            .on_click(dropdown_check(cx, views, "只看我的")),
        YueMenuItem::separator(),
        YueMenuItem::heading("主题"),
        YueMenuItem::new("跟随系统")
            .radio(dropdown_holds(
                &showcase.dropdown_sets[theme_slot],
                "跟随系统",
            ))
            .keep_open(true)
            .on_click(dropdown_choose(cx, theme_slot, "跟随系统")),
        YueMenuItem::new("浅色")
            .radio(dropdown_holds(&showcase.dropdown_sets[theme_slot], "浅色"))
            .keep_open(true)
            .on_click(dropdown_choose(cx, theme_slot, "浅色")),
        YueMenuItem::new("深色")
            .radio(dropdown_holds(&showcase.dropdown_sets[theme_slot], "深色"))
            .keep_open(true)
            .on_click(dropdown_choose(cx, theme_slot, "深色")),
    ]
    .into_iter()
    .collect()
}
