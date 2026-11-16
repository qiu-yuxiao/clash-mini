/**
 * BUG-385: 判断 Mixed Port 输入框 blur 时是否应跳过自动保存：
 * 1) 卡片已卸载（关闭设置或极小窗自动收起触发的卸载后尾随 blur）
 * 2) 用户按 Esc 显式取消了当前编辑
 * 3) 焦点直接落到右上角设置关闭按钮（[data-close-settings="true"]）
 */
export function shouldSkipMixedPortBlurSave({
  isMounted,
  skipRequested,
  relatedTarget,
}: {
  isMounted: boolean
  skipRequested: boolean
  relatedTarget: EventTarget | null
}): boolean {
  if (!isMounted || skipRequested) return true
  const target = relatedTarget as {
    closest?: (selector: string) => Element | null
  } | null
  return Boolean(target?.closest?.('[data-close-settings="true"]'))
}
