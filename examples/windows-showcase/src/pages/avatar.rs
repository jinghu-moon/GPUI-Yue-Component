use super::super::*;
use std::path::PathBuf;
use yue_component::{AvatarShape, AvatarSize, YueAvatar, YueAvatarGroup};

pub(crate) fn avatar_content(theme: YueTheme) -> impl IntoElement {
    let photo = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/image-preview/desktop-q70.jpg"
    ));
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_6()
        .p_8()
        .child(div().text_2xl().child("Avatar 头像"))
        .child(section(
            theme,
            "Fallbacks",
            "使用名字首字符；图片缺省或加载失败时回退",
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(YueAvatar::new("avatar-default", theme).accessibility_label("默认头像"))
                .child(YueAvatar::new("avatar-latin", theme).name("Ada Lovelace"))
                .child(YueAvatar::new("avatar-cjk", theme).name("王伟"))
                .child(
                    YueAvatar::new("avatar-image-failed", theme)
                        .name("Grace Hopper")
                        .src("missing-avatar.png"),
                ),
        ))
        .child(section(
            theme,
            "Sizes and shapes",
            "四档固定尺寸与圆形、圆角方形",
            div().flex().items_center().gap_4().children([
                YueAvatar::new("avatar-small", theme)
                    .name("Ada Lovelace")
                    .size(AvatarSize::Small),
                YueAvatar::new("avatar-medium", theme)
                    .name("Ada Lovelace")
                    .size(AvatarSize::Medium),
                YueAvatar::new("avatar-large", theme)
                    .name("Ada Lovelace")
                    .size(AvatarSize::Large),
                YueAvatar::new("avatar-xlarge", theme)
                    .name("Ada Lovelace")
                    .size(AvatarSize::XLarge),
                YueAvatar::new("avatar-rounded", theme)
                    .name("W")
                    .size(AvatarSize::Large)
                    .shape(AvatarShape::Rounded),
            ]),
        ))
        .child(section(
            theme,
            "Image",
            "本地图片裁剪为 Cover；加载失败时显示姓名回退",
            YueAvatar::new("avatar-photo", theme)
                .src(photo)
                .name("Yue Component")
                .size(AvatarSize::Large),
        ))
        .child(section(
            theme,
            "Avatar group",
            "左侧头像覆盖右侧，细轮廓分隔重叠边缘；超出 max 时显示剩余数量",
            div()
                .flex()
                .flex_col()
                .items_start()
                .gap_4()
                .child(
                    YueAvatarGroup::new("avatar-team", theme)
                        .accessibility_label("设计团队")
                        .max(4)
                        .size(AvatarSize::Large)
                        .children([
                            YueAvatar::new("team-ada", theme).name("Ada Lovelace"),
                            YueAvatar::new("team-grace", theme).name("Grace Hopper"),
                            YueAvatar::new("team-lin", theme).name("林"),
                            YueAvatar::new("team-ken", theme).name("Ken Thompson"),
                            YueAvatar::new("team-margaret", theme).name("Margaret Hamilton"),
                            YueAvatar::new("team-dennis", theme).name("Dennis Ritchie"),
                        ]),
                )
                .child(
                    YueAvatarGroup::new("avatar-small-team", theme)
                        .accessibility_label("小号头像组")
                        .size(AvatarSize::Small)
                        .max(0)
                        .children([
                            YueAvatar::new("small-team-ada", theme).name("Ada Lovelace"),
                            YueAvatar::new("small-team-grace", theme).name("Grace Hopper"),
                            YueAvatar::new("small-team-lin", theme).name("林"),
                            YueAvatar::new("small-team-ken", theme).name("Ken Thompson"),
                        ]),
                ),
        ))
}
