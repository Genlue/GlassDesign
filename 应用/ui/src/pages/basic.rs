use glass_design::prelude::*;
use leptos::prelude::*;

#[component]
pub fn BasicPage() -> impl IntoView {
    let chip_on = RwSignal::new(true);
    let disclosure = RwSignal::new(true);
    let saved = RwSignal::new(false);

    view! {
        <div class="l-scroll" style="padding:24px 32px 64px">
            <div class="l-stack l-stack--6">
                <div class="l-stack l-stack--2">
                    <h1 class="c-headline">"基础控件"</h1>
                    <p class="c-subtitle">"按钮、状态、进度、折叠。全部由 Rust 组件拼装。"</p>
                </div>

                <div class="l-stack l-stack--3">
                    <SectionTitle>"材质"</SectionTitle>
                    <Card>
                        <div style="position:relative;height:190px;border-radius:18px;overflow:hidden;background:linear-gradient(115deg,#ff9a62 0%,#ff5f8f 34%,#7b61ff 68%,#2ec5ce 100%)">
                            <div style="position:absolute;inset:0;background-image:repeating-linear-gradient(90deg,rgba(255,255,255,.20) 0 2px,transparent 2px 28px)"></div>
                            <div style="position:absolute;left:26px;top:24px;color:#fff;font-size:12px;letter-spacing:.04em;opacity:.9">"静态 SVG filter · 无 JavaScript"</div>
                            <div style="position:absolute;left:26px;bottom:26px;display:flex;align-items:center;gap:14px">
                                <Capsule size=CapsuleSize::Md>
                                    <span style="font-size:13px;font-weight:500;color:#fff">"Liquid Glass"</span>
                                </Capsule>
                                <span style="color:#fff;font-size:12px;opacity:.78">"位移 + 高光贴图在构建期算好"</span>
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
                    <InlineMessage tone=Tone::Success title="已创建" description="这是 P2 的静态反馈。"
                        dismissible=true on_dismiss=Callback::new(move |_| saved.set(false)) />
                </Show>
            </div>
        </div>
    }
}
