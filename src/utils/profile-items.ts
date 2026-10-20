/**
 * 订阅配置项的「真实性」判定。
 *
 * ### 为什么单独抽出来
 *
 * 后端 `Config::init_config()` 每次启动都会调用 `ensure_default_profile_items()`，
 * **无条件**向 `profiles.items` 追加 `Merge`（`type: merge`）与 `Script`（`type: script`）
 * 两个内置项。因此 `profiles.items.length` **恒 ≥ 2**，永远不可能为 0。
 *
 * 首次使用引导曾用 `items.length === 0` 判断「还没添加任何订阅」，结果弹窗永不出现。
 * 判断「是否首次使用」必须只看**真实订阅**（`remote` / `local`），忽略内置项。
 */

export type ProfileTypeLike = 'local' | 'remote' | 'merge' | 'script' | string

interface ProfileItemLike {
  type?: ProfileTypeLike
}

/** 是否为用户真实添加的订阅项（远程订阅或本地文件）；内置 merge/script 不算。 */
export const isRealProfileItem = (item?: ProfileItemLike | null): boolean =>
  !!item && (item.type === 'remote' || item.type === 'local')

/** 列表里是否存在至少一个真实订阅项。 */
export const hasRealProfileItem = (
  items?: ReadonlyArray<ProfileItemLike | null | undefined> | null,
): boolean => (items ?? []).some((item) => isRealProfileItem(item))
