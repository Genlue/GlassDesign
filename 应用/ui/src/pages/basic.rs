use glass_design::prelude::*;
use leptos::prelude::*;

#[component]
pub fn BasicPage() -> impl IntoView {
    let chip_on = RwSignal::new(true);
    let disclosure = RwSignal::new(true);
    let saved = RwSignal::new(false);

    view! {
        <div class="gd-page">
            <div class="l-stack l-stack--6">
                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"基础控件"</h1>
                    <p class="c-subtitle" style="white-space:normal">"按钮、状态、进度、折叠。全部由 Rust 组件拼装。"</p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"材质"</SectionTitle>
                    <Card>
                        // 材质舞台：只是一块彩色底，玻璃才有东西可折。
                        // 高度压到 128 —— 它是个「衬底」，不是页面的主角；
                        // 原来 190px 的整幅彩虹把下面的控件全压住了。
                        <div style="position:relative;height:128px;border-radius:14px;overflow:hidden;background:linear-gradient(115deg,#ff9a62 0%,#ff5f8f 34%,#7b61ff 68%,#2ec5ce 100%)">
                            <div style="position:absolute;inset:0;background-image:repeating-linear-gradient(90deg,rgba(255,255,255,.20) 0 2px,transparent 2px 28px)"></div>
                            <div style="position:absolute;left:20px;top:18px;color:#fff;font-size:11px;letter-spacing:.04em;opacity:.82">"静态 SVG filter · 无 JavaScript"</div>
                            <div style="position:absolute;left:20px;bottom:18px;display:flex;align-items:center;gap:12px">
                                <Capsule size=CapsuleSize::Md>
                                    <span style="font-size:13px;font-weight:500;color:#fff">"Liquid Glass"</span>
                                </Capsule>
                                <span style="color:#fff;font-size:11px;opacity:.72">"位移 + 高光贴图在构建期算好"</span>
                            </div>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"按钮"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <Row label="变体" hint="主操作只放一个">
                                <div class="l-row l-row--3">
                                    <Button label="默认" />
                                    <Button label="主要" variant=ButtonVariant::Primary />
                                    <Button label="柔和" variant=ButtonVariant::Tonal />
                                    <Button label="危险" variant=ButtonVariant::Danger />
                                </div>
                            </Row>
                            <Row label="带图标">
                                <div class="l-row l-row--3">
                                    <Button label="新建" icon=IconName::FOLDER_PLUS variant=ButtonVariant::Primary
                                        on_click=Callback::new(move |_| saved.set(true)) />
                                    <IconButton icon=IconName::REFRESH_CW label="刷新" />
                                    <IconButton icon=IconName::TRASH label="删除" variant=IconButtonVariant::Tonal />
                                </div>
                            </Row>
                            <Row label="禁用"><Button label="不可用" disabled=true /></Row>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"标签与头像"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <Row label="徽标">
                                <div class="l-row l-row--3">
                                    <Badge label="默认" />
                                    <Badge label="强调" variant=BadgeVariant::Accent />
                                    <Badge label="危险" variant=BadgeVariant::Danger />
                                </div>
                            </Row>
                            <Row label="胶囊">
                                <div class="l-row l-row--3">
                                    <Chip label="Design" icon=IconName::LAYERS selected=Signal::derive(move || chip_on.get())
                                        on_click=Callback::new(move |_| chip_on.update(|v| *v = !*v)) />
                                    <Chip label="可移除" removable=true on_remove=Callback::new(move |_| {}) />
                                    <Chip label="纯展示" />
                                </div>
                            </Row>
                            <Row label="头像">
                                <div class="l-row l-row--3" style="align-items:center">
                                    <Avatar name="李明" size=AvatarSize::Sm status=AvatarStatus::Online />
                                    <Avatar name="Ada Lovelace" size=AvatarSize::Lg status=AvatarStatus::Busy />
                                    <Avatar name="Glass Design" size=AvatarSize::Md square=true />
                                </div>
                            </Row>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"进度与占位"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <Row label="环形进度">
                                <div class="l-row l-row--4" style="align-items:center">
                                    <ProgressRing value=Signal::derive(move || 72.0) label="完成度" />
                                    <ProgressRing value=Signal::derive(move || 38.0) size=RingSize::Lg label="完成度" />
                                </div>
                            </Row>
                            <Row label="进度条">
                                <div style="width:320px"><Progress value=Signal::derive(move || 42.0) label="上传" /></div>
                            </Row>
                            <Row label="加载">
                                <div class="l-row l-row--3" style="align-items:center">
                                    <Spinner />
                                    <Skeleton width="180px" height="12px" />
                                </div>
                            </Row>
                            <Row label="骨架屏">
                                <div style="width:320px"><Skeleton lines=3 height="12px" /></div>
                            </Row>
                            <Row label="空态">
                                <Empty title="没有内容" desc="换一个筛选条件试试。" />
                            </Row>
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"行内提示"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <InlineMessage title="已保存" description="本地状态已更新。" />
                            <InlineMessage tone=Tone::Warning title="注意" description="这个操作不能撤销。" />
                            <InlineMessage tone=Tone::Danger title="失败" description="网络不可用，请稍后重试。" />
                        </div>
                    </Card>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"折叠"</SectionTitle>
                    <Card>
                        <div class="l-pad">
                            <Disclosure title="高级选项" hint="展开查看"
                                open=Signal::derive(move || disclosure.get())
                                on_toggle=Callback::new(move |v| disclosure.set(v))>
                                <div class="l-stack l-stack--3">
                                    <Row label="缓存"><Switch checked=true /></Row>
                                    <Row label="自动更新"><Switch checked=false /></Row>
                                </div>
                            </Disclosure>
                        </div>
                    </Card>
                </div>

                <Show when=move || saved.get()>
                    <InlineMessage tone=Tone::Success title="已创建" description="这是静态反馈。"
                        dismissible=true on_dismiss=Callback::new(move |_| saved.set(false)) />
                </Show>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"玻璃尺寸档"</SectionTitle>
                    <Card>
                        <div class="l-pad l-stack l-stack--4">
                            <p class="c-subtitle">
                                "只有显式 data-glass 的元素才套用固定尺寸贴图；不写档位仍走 Auto 涂层（无折射）。"
                            </p>
                            <div style="position:relative;display:flex;flex-wrap:wrap;align-items:flex-start;gap:16px;padding:20px;border-radius:18px;overflow:hidden;background:linear-gradient(115deg,#ff9a62 0%,#ff5f8f 34%,#7b61ff 68%,#2ec5ce 100%)">
                                <div style="position:absolute;inset:0;background-image:repeating-linear-gradient(90deg,rgba(255,255,255,.18) 0 2px,transparent 2px 32px)"></div>
                                <GlassPanel size=GlassPanelSize::P320x220>
                                    <div style="position:relative;padding:18px;color:#fff;font-size:12px;letter-spacing:.04em;opacity:.92">
                                        "panel 320 × 220"
                                    </div>
                                </GlassPanel>
                                <GlassPanel size=GlassPanelSize::P480x320>
                                    <div style="position:relative;padding:18px;color:#fff;font-size:12px;letter-spacing:.04em;opacity:.92">
                                        "panel 480 × 320"
                                    </div>
                                </GlassPanel>
                                <GlassPanel size=GlassPanelSize::P640x420>
                                    <div style="position:relative;padding:18px;color:#fff;font-size:12px;letter-spacing:.04em;opacity:.92">
                                        "panel 640 × 420"
                                    </div>
                                </GlassPanel>
                            </div>
                            <div style="position:relative;padding:20px;border-radius:18px;overflow:hidden;background:linear-gradient(115deg,#2ec5ce 0%,#7b61ff 46%,#ff5f8f 100%)">
                                <div style="position:absolute;inset:0;background-image:repeating-linear-gradient(90deg,rgba(255,255,255,.16) 0 2px,transparent 2px 32px)"></div>
                                <div class="c-menu" data-glass="menu-240x280" role="menu" style="position:relative;left:0;top:0">
                                    <div class="c-menu__item" role="menuitem" tabindex="-1">"新建文件"</div>
                                    <div class="c-menu__item" role="menuitem" tabindex="-1">"新建文件夹"</div>
                                    <div class="c-menu__sep" role="separator"></div>
                                    <div class="c-menu__item" role="menuitem" tabindex="-1">"重命名"</div>
                                    <div class="c-menu__item" role="menuitem" tabindex="-1">"复制链接"</div>
                                    <div class="c-menu__item" role="menuitem" tabindex="-1">"在访达中显示"</div>
                                    <div class="c-menu__item" role="menuitem" tabindex="-1">"移动到…"</div>
                                    <div class="c-menu__item" role="menuitem" tabindex="-1">"导出为 PDF"</div>
                                    <div class="c-menu__sep" role="separator"></div>
                                    <div class="c-menu__item c-menu__item--danger" role="menuitem" tabindex="-1">"删除"</div>
                                </div>
                            </div>
                        </div>
                    </Card>
                </div>
            </div>
        </div>
    }
}
