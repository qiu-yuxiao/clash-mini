/**
 * 全局测速锁引用计数模块（H-11）
 *
 * 多处独立调用 setResizable(false) + setDragRegionEnabled(false) 时，
 * 先完成者不会破坏另一个测速期间的 UI 锁定。
 *
 * 用法：
 *   进入测速：batchTestLockRef.current++
 *   退出测速：batchTestLockRef.current = Math.max(0, batchTestLockRef.current - 1)
 *   仅当 batchTestLockRef.current === 0 时才恢复窗口可调整/可拖动
 */

export const batchTestLockRef = { current: 0 }
